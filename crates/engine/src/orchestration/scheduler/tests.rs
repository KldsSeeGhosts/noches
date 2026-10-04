// Conformance tests mirror T3 Schedule.test.ts and ScheduledTaskService*.test.ts.
use super::*;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use chrono_tz::Tz;
use futures::future::BoxFuture;
use serde_json::{Value, json};
use zeron_proto::orchestration_mcp::OrchestrationToolInput;
use zeron_proto::scheduler::*;
use zeron_sync::DocsStore;

use super::service::SchedulerService;
use crate::orchestration::command::{Command, Operation};
use crate::orchestration::service::CallerScope;
use crate::orchestration::{Kernel, WriteBoundary};

const NOW: i64 = 1_800_000_000_000;

struct FakeClock {
    now: AtomicI64,
    zone: Tz,
}
impl FakeClock {
    fn set(&self, now: i64) {
        self.now.store(now, Ordering::SeqCst);
    }
}
impl schedule::Clock for FakeClock {
    fn now_ms(&self) -> i64 {
        self.now.load(Ordering::SeqCst)
    }
    fn next_run_at(&self, schedule: &ScheduledTaskSchedule, from: i64) -> Option<i64> {
        schedule::next_in_zone(schedule, from, &self.zone)
    }
}

type Hook = Arc<
    dyn Fn(ScheduledDispatch) -> BoxFuture<'static, std::result::Result<(), String>> + Send + Sync,
>;
#[derive(Default)]
struct Recorder {
    runs: StdMutex<Vec<ScheduledDispatch>>,
    hook: StdMutex<Option<Hook>>,
    fail: AtomicBool,
}
#[async_trait]
impl ScheduledTaskDispatch for Recorder {
    async fn dispatch(&self, run: ScheduledDispatch) -> std::result::Result<(), String> {
        self.runs.lock().unwrap().push(run.clone());
        let hook = self.hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            return hook(run).await;
        }
        if self.fail.load(Ordering::SeqCst) {
            Err("Dispatch refused.".into())
        } else {
            Ok(())
        }
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    scheduler: Arc<Scheduler>,
    clock: Arc<FakeClock>,
    recorder: Arc<Recorder>,
    kernel: Kernel,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Kernel::open(Arc::new(DocsStore::open(dir.path()).unwrap()), "host").unwrap();
        let clock = Arc::new(FakeClock {
            now: AtomicI64::new(NOW),
            zone: chrono_tz::UTC,
        });
        let recorder = Arc::new(Recorder::default());
        let scheduler = Arc::new(Scheduler::with_clock(
            kernel.store.clone(),
            recorder.clone(),
            clock.clone(),
        ));
        Self {
            _dir: dir,
            scheduler,
            clock,
            recorder,
            kernel,
        }
    }
    fn input(&self, id: &str, schedule: Value) -> ScheduledTaskUpsertInput {
        serde_json::from_value(json!({
            "id":id,"title":"Review","prompt":"Review the changes","enabled":true,
            "schedule":schedule,"projectId":"project","threadId":"parent","workspaceStrategy":{"type":"root"},
            "modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default"
        })).unwrap()
    }
    fn task(&self, id: &str) -> ScheduledTask {
        self.scheduler
            .upsert(self.input(id, json!({"type":"interval","everyMs":60_000})))
            .unwrap()
    }
    fn due(&self, id: &str, offset: i64) -> ScheduledTask {
        self.task(id);
        self.scheduler
            .mutate(&id.into(), "test", |old, _| {
                let mut task = old.unwrap();
                task.next_run_at = Some(event::iso(NOW + offset)?);
                Ok(task)
            })
            .unwrap()
    }
    fn parent(&self) -> CallerScope {
        let command = Command::wire(serde_json::from_value(json!({
            "type":"thread.create","commandId":"create-parent","threadId":"parent","projectId":"project","title":"Parent",
            "createdBy":"user","creationSource":"web","modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default","branch":"main","worktreePath":"/repo"
        })).unwrap()).unwrap();
        self.kernel.store.dispatch(&command, NOW).unwrap();
        let thread = self.kernel.store.thread(&"parent".into()).unwrap().unwrap();
        let seed = crate::orchestration::task::execution_seed(
            &thread.thread,
            1,
            "input",
            "starting",
            "mock",
            NOW,
        )
        .unwrap();
        self.kernel
            .store
            .dispatch(
                &Command {
                    id: "seed".into(),
                    thread_id: "parent".into(),
                    operation: Operation::CreateExecution(Box::new(seed.clone())),
                },
                NOW,
            )
            .unwrap();
        CallerScope {
            thread_id: "parent".into(),
            run_id: seed.run.id,
            session_id: "session/parent:1".into(),
            project_id: "project".into(),
            workspace_root: "/repo".into(),
            runtime_mode: zeron_proto::RuntimeMode::FullAccess,
            interaction_mode: zeron_proto::InteractionMode::Default,
            provider_instance_id: "mock".into(),
        }
    }
    fn set_parent(&self, edit: impl FnOnce(&mut OrchestrationV2AppThread)) {
        let mut thread = self
            .kernel
            .store
            .thread(&"parent".into())
            .unwrap()
            .unwrap()
            .thread;
        edit(&mut thread);
        let command = Command {
            id: format!("edit-{}", uuid::Uuid::new_v4()).into(),
            thread_id: "parent".into(),
            operation: Operation::SessionBinding(Box::new(thread)),
        };
        self.kernel.store.dispatch(&command, NOW).unwrap();
    }
    fn claims(&self) -> Vec<(String, String, String)> {
        self.scheduler.store.read(|conn| {
            let mut stmt = conn.prepare("SELECT claim_id,trigger,status FROM orchestration_scheduled_runs ORDER BY rowid")?;
            Ok(stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        }).unwrap()
    }
}

fn sched(value: Value) -> ScheduledTaskSchedule {
    serde_json::from_value(value).unwrap()
}
fn epoch(value: &str) -> i64 {
    parse_epoch(value).unwrap()
}
fn tool(name: &str, args: Value) -> OrchestrationToolInput {
    serde_json::from_value(json!({"name":name,"arguments":args})).unwrap()
}

struct ThreadTargets;
#[async_trait]
impl crate::orchestration::task::DelegationTargets for ThreadTargets {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&zeron_proto::orchestration_mcp::DelegateTaskInputTarget>,
    ) -> std::result::Result<
        crate::orchestration::task::ResolvedTarget,
        crate::orchestration::service::ToolError,
    > {
        let mut selection = parent.model_selection.clone();
        if let Some(model) = target.and_then(|t| t.model.as_ref()) {
            selection.model = model.clone();
        }
        Ok(crate::orchestration::task::ResolvedTarget {
            selection,
            driver: "mock".into(),
        })
    }
}

fn thread_dispatch(kernel: Kernel) -> Arc<dispatch::ThreadDispatch> {
    let threads = Arc::new(crate::orchestration::threads::KernelThreadService {
        delegation: Arc::new(crate::orchestration::task::DelegationService {
            kernel: kernel.clone(),
            targets: Arc::new(ThreadTargets),
        }),
        kernel: kernel.clone(),
    });
    Arc::new(dispatch::ThreadDispatch {
        store: kernel.store,
        threads,
        launch: None,
    })
}

#[tokio::test]
async fn bound_thread_dispatch_replays_lost_acceptance_once_with_current_binding_and_provenance() {
    let f = Fixture::new();
    f.parent();
    f.set_parent(|t| t.worktree_path = Some("/repo/current-checkout".into()));
    let mut input = f.input("host-dispatch", json!({"type":"interval","everyMs":60_000}));
    input.created_by = Optional::Present(OrchestrationV2Actor::System);
    input.creation_source = Optional::Present(OrchestrationV2CreationSource::Server);
    input.model_selection.model = "scheduled-model".into();
    f.scheduler.upsert(input).unwrap();
    let kernel = f.kernel.clone();
    let path = f._dir.path().to_owned();
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |run| {
        let kernel = kernel.clone();
        let path = path.clone();
        Box::pin(async move {
            kernel.store.inject_failure(WriteBoundary::AfterCommit, 1);
            assert!(thread_dispatch(kernel).dispatch(run.clone()).await.is_err());
            // Fresh service/store after an uncertain committed acceptance.
            // Replay the durable claim identities, not a new manual run.
            let restarted = Kernel::open(Arc::new(DocsStore::open(path).unwrap()), "host").unwrap();
            thread_dispatch(restarted.clone())
                .dispatch(run.clone())
                .await?;
            let p = restarted.store.thread(&"parent".into()).unwrap().unwrap();
            let messages = crate::orchestration::task::records(&p, "message");
            let messages: Vec<_> = messages
                .iter()
                .filter(|m| m["id"] == run.message_id.0)
                .collect();
            assert_eq!(messages.len(), 1);
            let m = messages[0];
            assert_eq!(m["scheduledTaskId"], "host-dispatch");
            assert_eq!(m["createdBy"], "system");
            assert_eq!(m["creationSource"], "server");
            assert!(m["senderThreadId"].is_null());
            let accepted = p.runs.iter().find(|r| m["runId"] == r.id.0).unwrap();
            assert_eq!(accepted.model_selection.model, "scheduled-model");
            assert_eq!(
                p.thread.worktree_path.as_deref(),
                Some("/repo/current-checkout")
            );
            assert_eq!(accepted.status, OrchestrationV2RunStatus::Queued);
            Ok(())
        })
    }));
    assert_eq!(
        f.scheduler
            .run_now(&"host-dispatch".into())
            .await
            .unwrap()
            .last_run_status,
        ScheduledTaskRunStatus::Succeeded
    );
}

#[tokio::test]
async fn thread_dispatch_revalidates_paused_or_deleted_claim_before_intake() {
    for deleted in [false, true] {
        let f = Fixture::new();
        f.parent();
        f.due("revalidate", 0);
        let scheduler = f.scheduler.clone();
        let adapter = thread_dispatch(f.kernel.clone());
        *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |run| {
            if deleted {
                scheduler.delete(&run.task.id).unwrap();
            } else {
                scheduler
                    .mutate(&run.task.id, "test", |old, _| {
                        let mut task = old.unwrap();
                        task.enabled = false;
                        Ok(task)
                    })
                    .unwrap();
            }
            let adapter = adapter.clone();
            Box::pin(async move {
                assert!(adapter.dispatch(run).await.is_err());
                Err("Refused before intake.".into())
            })
        }));
        f.scheduler.tick().await.unwrap();
        let p = f.kernel.store.thread(&"parent".into()).unwrap().unwrap();
        assert!(crate::orchestration::task::records(&p, "message").is_empty());
        assert_eq!(f.claims()[0].2, "failed");
    }
}

#[tokio::test]
async fn unbound_thread_dispatch_is_unavailable_without_installed_launch() {
    let f = Fixture::new();
    let mut input = f.input("unbound", json!({"type":"interval","everyMs":60_000}));
    input.thread_id = Optional::Present(None);
    f.scheduler.upsert(input).unwrap();
    f.scheduler
        .set_dispatcher(thread_dispatch(f.kernel.clone()));
    let task = f.scheduler.run_now(&"unbound".into()).await.unwrap();
    assert_eq!(task.last_run_status, ScheduledTaskRunStatus::Failed);
    assert_eq!(
        task.last_run_error.as_deref(),
        Some("The operation could not be completed.")
    );
}

#[test]
fn parse_times() {
    assert_eq!(schedule::parse_time("09:30"), Some((9, 30)));
    assert_eq!(schedule::parse_time("9:30"), Some((9, 30)));
    assert_eq!(schedule::parse_time("23:59"), Some((23, 59)));
    assert_eq!(schedule::parse_time("25:00"), None);
    for time in [
        "009:00", "9:0", "9:60", "-1:30", "2a:00", "24:00", "9:00:00",
    ] {
        assert_eq!(schedule::parse_time(time), None, "{time}");
    }
}

#[test]
fn interval_and_legacy_floor_and_overflow() {
    let from = epoch("2026-07-01T16:00:00.000Z");
    assert_eq!(
        schedule::next_in_zone(
            &sched(json!({"type":"interval","everyMs":300000})),
            from,
            &chrono_tz::UTC
        ),
        Some(from + 300000)
    );
    assert_eq!(
        schedule::next_in_zone(
            &sched(json!({"type":"interval","everyMs":1000})),
            from,
            &chrono_tz::UTC
        ),
        Some(from + 60000)
    );
    assert_eq!(
        schedule::next_in_zone(
            &sched(json!({"type":"interval","everyMs":i64::MAX})),
            from,
            &chrono_tz::UTC
        ),
        None
    );
}

#[test]
fn weekday_and_semantic_equality_match_t3_oracle() {
    let a = sched(json!({"type":"fixed_time","timeOfDay":"9:00","weekdays":[5,1]}));
    let b = sched(json!({"type":"fixed_time","timeOfDay":"09:00","weekdays":[1,5,5]}));
    assert!(schedule::same_schedule(&a, &b));
    let daily = sched(json!({"type":"fixed_time","timeOfDay":"09:00"}));
    for days in [json!([]), json!([0, 1, 2, 3, 4, 5, 6])] {
        assert!(schedule::same_schedule(
            &daily,
            &sched(json!({"type":"fixed_time","timeOfDay":"9:00","weekdays":days}))
        ));
    }
    assert!(!schedule::same_schedule(&a, &daily));
    assert!(!schedule::same_schedule(
        &daily,
        &sched(json!({"type":"fixed_time","timeOfDay":"09:30"}))
    ));
    assert!(!schedule::same_schedule(
        &daily,
        &sched(json!({"type":"interval","everyMs":60000}))
    ));
    let from = epoch("2026-07-03T17:00:00.000Z"); // Friday 10am Los Angeles.
    let weekdays = sched(json!({"type":"fixed_time","timeOfDay":"09:00","weekdays":[1,2,3,4,5]}));
    assert_eq!(
        schedule::next_in_zone(&weekdays, from, &chrono_tz::America::Los_Angeles),
        Some(epoch("2026-07-06T16:00:00.000Z"))
    );
}

#[test]
fn dst_gap_fold_and_exact_slot() {
    let zone = chrono_tz::America::New_York;
    let gap = sched(json!({"type":"fixed_time","timeOfDay":"02:30"}));
    assert_eq!(
        schedule::next_in_zone(&gap, epoch("2026-03-08T05:00:00Z"), &zone),
        Some(epoch("2026-03-08T07:30:00Z"))
    );
    let fold = sched(json!({"type":"fixed_time","timeOfDay":"01:30"}));
    assert_eq!(
        schedule::next_in_zone(&fold, epoch("2026-11-01T04:00:00Z"), &zone),
        Some(epoch("2026-11-01T05:30:00Z"))
    );
    // Do not execute the second copy of a repeated wall-clock slot.
    assert_eq!(
        schedule::next_in_zone(&fold, epoch("2026-11-01T05:30:00Z"), &zone),
        Some(epoch("2026-11-02T06:30:00Z"))
    );
    let half_hour_gap = sched(json!({"type":"fixed_time","timeOfDay":"02:15"}));
    assert_eq!(
        schedule::next_in_zone(
            &half_hour_gap,
            epoch("2026-10-03T14:00:00Z"),
            &chrono_tz::Australia::Lord_Howe
        ),
        Some(epoch("2026-10-03T15:45:00Z"))
    );
}

#[test]
fn fixed_time_grace_boundary_and_interval_catchup() {
    let fixed = sched(json!({"type":"fixed_time","timeOfDay":"09:00"}));
    assert!(!schedule::missed_fixed(&fixed, NOW, NOW + 600000));
    assert!(schedule::missed_fixed(&fixed, NOW, NOW + 600001));
    assert!(!schedule::missed_fixed(
        &sched(json!({"type":"interval","everyMs":60000})),
        NOW,
        NOW + 9000000
    ));
}

#[test]
fn rejects_short_writes_but_legacy_read_pause_rename_delete_work() {
    let f = Fixture::new();
    assert!(serde_json::from_value::<ScheduledTaskUpsertInput>(json!({
        "title":"task","prompt":"task","enabled":true,"schedule":{"type":"interval","everyMs":59999},
        "projectId":"project","workspaceStrategy":{"type":"root"},"modelSelection":{"instanceId":"mock","model":"mock-1"},
        "runtimeMode":"full-access","interactionMode":"default"
    })).is_err());
    let task = f.task("legacy");
    f.scheduler
        .mutate(&task.id, "test", |old, _| {
            let mut old = old.unwrap();
            old.schedule = sched(json!({"type":"interval","everyMs":1000}));
            Ok(old)
        })
        .unwrap();
    assert_eq!(f.scheduler.list().unwrap().tasks.len(), 1);
    let paused = f
        .scheduler
        .update_for_user(ScheduledTaskUpdateRequest {
            owner_host_id: "host".into(),
            id: "legacy".into(),
            enabled: Some(false),
            title: Some("New".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(!paused.enabled);
    assert_eq!(paused.next_run_at, None);
    f.scheduler.delete(&task.id).unwrap();
    assert!(f.scheduler.list().unwrap().tasks.is_empty());
}

#[test]
fn persisted_contract_trimming_nonempty_and_count_checks_match_t3() {
    let f = Fixture::new();
    let mut input = f.input(" task ", json!({"type":"interval","everyMs":60000}));
    input.title = "  Title  ".into();
    input.prompt = "  Prompt  ".into();
    let task = f.scheduler.upsert(input).unwrap();
    assert_eq!(task.id.0, "task");
    assert_eq!(task.title, "Title");
    assert_eq!(task.prompt, "Prompt");
    let error = f
        .scheduler
        .update_for_user(ScheduledTaskUpdateRequest {
            owner_host_id: "host".into(),
            id: "task".into(),
            title: Some("  ".into()),
            ..Default::default()
        })
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Could not update schedule task.")
    );
    assert_eq!(f.scheduler.load(&"task".into()).unwrap().title, "Title");
    f.scheduler
        .store
        .write(|tx| {
            tx.execute(
                "UPDATE orchestration_scheduled_tasks SET run_count=-1 WHERE task_id='task'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(f.scheduler.load(&"task".into()).is_err());
}

#[test]
fn title_prompt_format_only_edits_keep_pending_due_and_deleted_form_fails() {
    let f = Fixture::new();
    let input = f.input("task", json!({"type":"fixed_time","timeOfDay":"9:00"}));
    f.scheduler.upsert(input.clone()).unwrap();
    let due = f
        .scheduler
        .mutate(&"task".into(), "test", |old, _| {
            let mut old = old.unwrap();
            old.next_run_at = Some(event::iso(NOW - 1000)?);
            Ok(old)
        })
        .unwrap();
    f.clock.set(NOW + 1000);
    let mut edit = input.clone();
    edit.title = "Edited".into();
    edit.prompt = "Updated prompt".into();
    edit.schedule =
        serde_json::from_value(json!({"type":"fixed_time","timeOfDay":"09:00","weekdays":[]}))
            .unwrap();
    let updated = f.scheduler.upsert(edit.clone()).unwrap();
    assert_eq!(updated.next_run_at, due.next_run_at);
    assert_eq!(updated.created_at, due.created_at);
    f.scheduler.delete(&due.id).unwrap();
    edit.require_existing = Optional::Present(true);
    assert_eq!(
        f.scheduler.upsert(edit.clone()).unwrap_err().message,
        "Schedule task not found."
    );
    edit.require_existing = Optional::Absent;
    assert!(f.scheduler.upsert(edit).is_ok()); // Explicit-ID creates remain valid.
}

#[tokio::test]
async fn downtime_interval_single_catchup_and_manual_each_call_new_claim() {
    let f = Fixture::new();
    let task = f.due("task", -86_400_000);
    f.scheduler.tick().await.unwrap();
    f.scheduler.tick().await.unwrap();
    assert_eq!(f.recorder.runs.lock().unwrap().len(), 1);
    assert_eq!(
        f.scheduler.load(&task.id).unwrap().next_run_at,
        Some(event::iso(NOW + 60000).unwrap())
    );
    f.scheduler.run_now(&task.id).await.unwrap();
    let completed = f.scheduler.run_now(&task.id).await.unwrap();
    assert_eq!(completed.last_run_status, ScheduledTaskRunStatus::Succeeded);
    assert_eq!(completed.run_count, 3);
    let runs = f.recorder.runs.lock().unwrap();
    assert_eq!(runs.len(), 3);
    assert_ne!(runs[1].command_id, runs[2].command_id);
    assert_ne!(runs[1].message_id, runs[2].message_id);
    assert_ne!(runs[1].claim_id, runs[2].claim_id);
    assert_eq!(
        f.claims().iter().map(|c| c.1.as_str()).collect::<Vec<_>>(),
        ["scheduled", "manual", "manual"]
    );
}

#[tokio::test]
async fn fixed_slots_skip_only_past_ten_minutes_without_counting_a_run() {
    let f = Fixture::new();
    let mut input = f.input("fixed", json!({"type":"fixed_time","timeOfDay":"09:00"}));
    f.clock.set(epoch("2026-07-01T08:00:00Z"));
    let first = f.scheduler.upsert(input.clone()).unwrap();
    let due = parse_epoch(first.next_run_at.as_ref().unwrap()).unwrap();
    f.clock.set(due + 600001);
    f.scheduler.tick().await.unwrap();
    assert!(f.recorder.runs.lock().unwrap().is_empty());
    let skipped = f.scheduler.load(&first.id).unwrap();
    assert_eq!(skipped.run_count, 0);
    assert_eq!(skipped.next_run_at, Some("2026-07-02T09:00:00.000Z".into()));
    f.clock.set(due - 3600000);
    input.enabled = false;
    f.scheduler.upsert(input.clone()).unwrap();
    input.enabled = true;
    f.scheduler.upsert(input).unwrap();
    f.clock.set(due + 600000);
    f.scheduler.tick().await.unwrap();
    assert_eq!(f.recorder.runs.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn pause_and_manual_disabled_run_and_dispatch_error_are_durable() {
    let f = Fixture::new();
    let task = f.due("task", 0);
    f.scheduler
        .update_for_user(ScheduledTaskUpdateRequest {
            owner_host_id: "host".into(),
            id: "task".into(),
            enabled: Some(false),
            ..Default::default()
        })
        .unwrap();
    f.scheduler.tick().await.unwrap();
    assert!(f.recorder.runs.lock().unwrap().is_empty());
    f.recorder.fail.store(true, Ordering::SeqCst);
    let failed = f.scheduler.run_now(&task.id).await.unwrap();
    assert_eq!(failed.last_run_status, ScheduledTaskRunStatus::Failed);
    assert_eq!(failed.last_run_error.as_deref(), Some("Dispatch refused."));
    assert_eq!(failed.next_run_at, None);
    assert_eq!(failed.run_count, 1);
    assert_eq!(f.claims()[0].2, "failed");
}

#[tokio::test]
async fn stale_due_rows_revalidate_delete_pause_postpone_and_latest_binding_prompt_model() {
    let f = Fixture::new();
    for id in [
        "a",
        "b-delete",
        "c-pause",
        "d-postpone",
        "e-edit",
        "f-corrupt-date",
    ] {
        f.due(id, 0);
    }
    let weak = Arc::downgrade(&f.scheduler);
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |run| {
        let weak = weak.clone();
        Box::pin(async move {
            if run.task.id.0 == "a" {
                let service = weak.upgrade().unwrap();
                service.delete(&"b-delete".into()).unwrap();
                service
                    .update_for_user(ScheduledTaskUpdateRequest {
                        owner_host_id: "host".into(),
                        id: "c-pause".into(),
                        enabled: Some(false),
                        ..Default::default()
                    })
                    .unwrap();
                service
                    .update_for_user(ScheduledTaskUpdateRequest {
                        owner_host_id: "host".into(),
                        id: "d-postpone".into(),
                        schedule: Some(
                            serde_json::from_value(json!({"type":"interval","everyMs":120000}))
                                .unwrap(),
                        ),
                        ..Default::default()
                    })
                    .unwrap();
                service
                    .update_for_user(ScheduledTaskUpdateRequest {
                        owner_host_id: "host".into(),
                        id: "e-edit".into(),
                        prompt: Some("Fresh".into()),
                        thread_id: Optional::Present(None),
                        workspace_strategy: Some(
                            serde_json::from_value(
                                json!({"type":"worktree","baseRef":"main","startFromOrigin":true}),
                            )
                            .unwrap(),
                        ),
                        model_selection: Some(
                            serde_json::from_value(json!({"instanceId":"custom","model":"new"}))
                                .unwrap(),
                        ),
                        ..Default::default()
                    })
                    .unwrap();
                service
                    .mutate(&"f-corrupt-date".into(), "test", |old, _| {
                        let mut task = old.unwrap();
                        task.next_run_at = Some("broken".into());
                        Ok(task)
                    })
                    .unwrap();
            }
            Ok(())
        })
    }));
    f.scheduler.tick().await.unwrap();
    let runs = f.recorder.runs.lock().unwrap();
    assert_eq!(
        runs.iter()
            .map(|r| r.task.id.0.as_str())
            .collect::<Vec<_>>(),
        ["a", "e-edit"]
    );
    assert_eq!(runs[1].task.prompt, "Fresh");
    assert_eq!(runs[1].task.thread_id, None);
    assert_eq!(runs[1].task.model_selection.model, "new");
    assert_eq!(
        serde_json::to_value(&runs[1].task.workspace_strategy).unwrap(),
        json!({"type":"worktree","baseRef":"main","startFromOrigin":true})
    );
}

#[tokio::test]
async fn concurrent_ticks_and_manual_calls_do_not_overlap_or_backlog() {
    let f = Fixture::new();
    let task = f.due("task", 0);
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (e, r) = (entered.clone(), release.clone());
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |_| {
        let (e, r) = (e.clone(), r.clone());
        Box::pin(async move {
            e.notify_one();
            r.notified().await;
            Ok(())
        })
    }));
    let scheduler = f.scheduler.clone();
    let tick = tokio::spawn(async move { scheduler.tick().await });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    f.scheduler.tick().await.unwrap();
    assert!(f.scheduler.run_now(&task.id).await.is_err());
    let independent = Scheduler::with_clock(
        f.scheduler.store.clone(),
        f.recorder.clone(),
        f.clock.clone(),
    );
    independent.tick().await.unwrap();
    assert!(independent.run_now(&task.id).await.is_err());
    assert_eq!(f.recorder.runs.lock().unwrap().len(), 1);
    release.notify_one();
    tick.await.unwrap().unwrap();
    assert_eq!(f.scheduler.load(&task.id).unwrap().run_count, 1);
    assert_eq!(f.claims().len(), 1);
}

#[tokio::test]
async fn completion_uses_current_schedule_pause_and_never_resurrects_or_stamps_recreation() {
    let f = Fixture::new();
    let weak = Arc::downgrade(&f.scheduler);
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |run| {
        let weak = weak.clone();
        Box::pin(async move {
            let service = weak.upgrade().unwrap();
            if run.task.id.0 == "pause" {
                service
                    .update_for_user(ScheduledTaskUpdateRequest {
                        owner_host_id: "host".into(),
                        id: "pause".into(),
                        title: Some("Latest".into()),
                        enabled: Some(false),
                        ..Default::default()
                    })
                    .unwrap();
            } else {
                service.delete(&run.task.id).unwrap();
                if run.task.id.0 == "recreate" {
                    let mut input: ScheduledTaskUpsertInput = serde_json::from_value(json!({
                        "id":"recreate","title":"New","prompt":"New","enabled":true,
                        "schedule":{"type":"interval","everyMs":60000},"projectId":"project",
                        "workspaceStrategy":{"type":"root"},"modelSelection":{"instanceId":"mock","model":"mock-1"},
                        "runtimeMode":"full-access","interactionMode":"default"
                    })).unwrap();
                    input.created_by = Optional::Present(OrchestrationV2Actor::User);
                    service.upsert(input).unwrap();
                }
            }
            Ok(())
        })
    }));
    for id in ["pause", "delete", "recreate"] {
        let task = f.task(id);
        f.scheduler.run_now(&task.id).await.unwrap();
    }
    let paused = f.scheduler.load(&"pause".into()).unwrap();
    assert_eq!(paused.title, "Latest");
    assert_eq!(paused.next_run_at, None);
    assert_eq!(paused.run_count, 1);
    assert!(f.scheduler.find(&"delete".into()).unwrap().is_none());
    let recreated = f.scheduler.load(&"recreate".into()).unwrap();
    assert_eq!(recreated.run_count, 0);
    assert_eq!(recreated.last_run_status, ScheduledTaskRunStatus::Never);
    assert!(f.claims().iter().all(|c| c.2 == "succeeded"));
}

#[tokio::test]
async fn restart_releases_stuck_and_corrupt_rows_without_refire_storm() {
    let f = Fixture::new();
    for id in ["running", "corrupt", "huge"] {
        let task = f.due(id, 0);
        f.scheduler
            .mutate(&task.id, "test", |old, _| {
                let mut task = old.unwrap();
                task.last_run_status = ScheduledTaskRunStatus::Running;
                if task.id.0 == "huge" {
                    task.schedule = sched(json!({"type":"interval","everyMs":i64::MAX}));
                }
                Ok(task)
            })
            .unwrap();
    }
    let payload = serde_json::to_string(&f.scheduler.load(&"running".into()).unwrap()).unwrap();
    f.scheduler.store.write(|tx| {
        tx.execute("UPDATE orchestration_scheduled_tasks SET task_json='broken' WHERE task_id='corrupt'", [])?;
        tx.execute(
            "INSERT INTO orchestration_scheduled_runs
             (claim_id,task_id,trigger,command_id,message_id,host_id,task_json,started_at,status)
             VALUES('stuck-claim','corrupt','scheduled','stuck-command','stuck-message','host',?1,?2,'running')",
            params![payload,event::iso(NOW)?],
        )?;
        Ok(())
    }).unwrap();
    f.due("due", 0);
    f.scheduler.recover().unwrap();
    assert_eq!(f.claims()[0].2, "failed");
    let running = f.scheduler.load(&"running".into()).unwrap();
    assert_eq!(running.last_run_status, ScheduledTaskRunStatus::Failed);
    assert_eq!(running.last_run_error.as_deref(), Some(RESTART_ERROR));
    assert_eq!(running.run_count, 1);
    assert_eq!(running.next_run_at, Some(event::iso(NOW + 60000).unwrap()));
    assert_eq!(f.scheduler.load(&"huge".into()).unwrap().next_run_at, None);
    let corrupt: (String, String, i64) = f.scheduler.store.read(|conn| Ok(conn.query_row(
        "SELECT last_run_status,last_run_error,run_count FROM orchestration_scheduled_tasks WHERE task_id='corrupt'",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    )?)).unwrap();
    assert_eq!(corrupt, ("failed".into(), RESTART_ERROR.into(), 1));
    f.scheduler.tick().await.unwrap();
    assert_eq!(
        f.recorder
            .runs
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.task.id.0.clone())
            .collect::<Vec<_>>(),
        ["due"]
    );
}

#[tokio::test]
async fn cancelling_dispatch_and_bookkeeping_failure_release_claims() {
    let f = Fixture::new();
    let task = f.task("task");
    let entered = Arc::new(tokio::sync::Notify::new());
    let e = entered.clone();
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |_| {
        let e = e.clone();
        Box::pin(async move {
            e.notify_one();
            futures::future::pending().await
        })
    }));
    let scheduler = f.scheduler.clone();
    let id = task.id.clone();
    let worker = tokio::spawn(async move { scheduler.run_now(&id).await });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    assert_eq!(
        f.scheduler.load(&task.id).unwrap().last_run_status,
        ScheduledTaskRunStatus::Failed
    );
    assert_eq!(f.claims()[0].2, "failed");
    *f.recorder.hook.lock().unwrap() = Some(Arc::new({
        let store = f.scheduler.store.clone();
        move |_| {
            store.inject_failure(WriteBoundary::BeforeCommit, 1);
            Box::pin(async { Ok(()) })
        }
    }));
    assert!(f.scheduler.run_now(&task.id).await.is_err());
    let released = f.scheduler.load(&task.id).unwrap();
    assert_eq!(released.last_run_status, ScheduledTaskRunStatus::Failed);
    assert_eq!(released.run_count, 2);
    assert!(f.claims().iter().all(|c| c.2 == "failed"));
}

#[tokio::test]
async fn daemon_first_tick_immediate_and_shutdown_releases_inflight_dispatch() {
    let f = Fixture::new();
    f.due("task", 0);
    let entered = Arc::new(tokio::sync::Notify::new());
    let e = entered.clone();
    *f.recorder.hook.lock().unwrap() = Some(Arc::new(move |_| {
        let e = e.clone();
        Box::pin(async move {
            e.notify_one();
            futures::future::pending().await
        })
    }));
    let stop = CancellationToken::new();
    let worker = f.scheduler.spawn(stop.clone());
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    stop.cancel();
    worker.await.unwrap();
    assert_eq!(
        f.scheduler.load(&"task".into()).unwrap().last_run_status,
        ScheduledTaskRunStatus::Failed
    );
}

#[tokio::test]
async fn mcp_create_defaults_unbind_rebind_and_session_key_upserts() {
    let f = Fixture::new();
    let caller = f.parent();
    let args = json!({"prompt":"Review daily\nDetail","schedule":{"type":"interval","everyMs":60000},"clientRequestId":"key /:"});
    let result = f
        .scheduler
        .invoke(caller.clone(), tool("schedule_task", args.clone()))
        .await
        .unwrap();
    assert_eq!(result["title"], "Review daily");
    assert_eq!(result["boundThreadId"], "parent");
    assert_eq!(result["enabled"], true);
    assert_eq!(
        result["scheduledTaskId"],
        "scheduled-task:command:mcp:session%2Fparent%3A1:schedule-task:key%20%2F%3A"
    );
    let id = ScheduledTaskId(result["scheduledTaskId"].as_str().unwrap().into());
    f.scheduler.run_now(&id).await.unwrap();
    f.clock.set(NOW + 1000);
    let mut retry = args.clone();
    retry["title"] = json!("Changed payload");
    let replay = f
        .scheduler
        .invoke(caller.clone(), tool("schedule_task", retry))
        .await
        .unwrap();
    assert_eq!(replay["scheduledTaskId"], result["scheduledTaskId"]);
    assert_eq!(f.scheduler.load(&id).unwrap().run_count, 1);
    assert_eq!(
        f.scheduler.load(&id).unwrap().created_by,
        OrchestrationV2Actor::Agent
    );
    let unbound = f
        .scheduler
        .invoke(
            caller.clone(),
            tool(
                "update_scheduled_task",
                json!({"scheduledTaskId":id,"bindToCurrentThread":false}),
            ),
        )
        .await
        .unwrap();
    assert_eq!(unbound["boundThreadId"], Value::Null);
    assert_eq!(
        serde_json::to_value(f.scheduler.load(&id).unwrap().workspace_strategy).unwrap(),
        json!({"type":"worktree","baseRef":"main","startFromOrigin":true})
    );
    f.scheduler
        .invoke(
            caller.clone(),
            tool(
                "update_scheduled_task",
                json!({"scheduledTaskId":id,"bindToCurrentThread":true}),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(f.scheduler.load(&id).unwrap().workspace_strategy).unwrap(),
        json!({"type":"root"})
    );
    let mut another = caller.clone();
    another.session_id = "other".into();
    let separate = f
        .scheduler
        .invoke(another, tool("schedule_task", args))
        .await
        .unwrap();
    assert_ne!(separate["scheduledTaskId"], result["scheduledTaskId"]);
    let created = f.scheduler.invoke(caller, tool("schedule_task", json!({
        "prompt":"unbound","bindToCurrentThread":false,
        "schedule":"{\"type\":\"fixed_time\",\"timeOfDay\":\"09:00\",\"weekdays\":[1,2,3,4,5]}"
    }))).await.unwrap();
    assert_eq!(created["boundThreadId"], Value::Null);
    assert_eq!(created["schedule"]["timeOfDay"], "09:00");
}

#[tokio::test]
async fn mcp_project_scope_crud_without_live_gate_and_manual_refusal_order() {
    let f = Fixture::new();
    let caller = f.parent();
    f.task("visible");
    let mut hidden = f.input("hidden", json!({"type":"interval","everyMs":60000}));
    hidden.project_id = "another-project".into();
    f.scheduler.upsert(hidden).unwrap();
    let list = f
        .scheduler
        .invoke(caller.clone(), tool("list_scheduled_tasks", json!({})))
        .await
        .unwrap();
    assert_eq!(list["tasks"].as_array().unwrap().len(), 1);
    for name in ["update_scheduled_task", "delete_scheduled_task"] {
        let error = f
            .scheduler
            .invoke(
                caller.clone(),
                tool(name, json!({"scheduledTaskId":"hidden"})),
            )
            .await
            .unwrap_err();
        assert_eq!(
            serde_json::to_value(error.into_failure()).unwrap(),
            json!({
                "_tag":"OrchestratorMcpFailure","code":"task_not_found",
                "message":"Scheduled task hidden was not found in the calling project."
            })
        );
    }
    let missing = f
        .scheduler
        .invoke(
            caller.clone(),
            tool("run_scheduled_task_now", json!({"taskId":"node:subagent"})),
        )
        .await
        .unwrap_err();
    assert_eq!(
        missing.message,
        "The task was not found in the calling project."
    );
    f.set_parent(|thread| thread.runtime_mode = zeron_proto::RuntimeMode::Auto);
    let denied = f
        .scheduler
        .invoke(
            caller.clone(),
            tool("run_scheduled_task_now", json!({"taskId":"missing"})),
        )
        .await
        .unwrap_err();
    assert_eq!(
        denied.message,
        "Running a scheduled task requires a live full-access/default thread."
    );
    f.set_parent(|thread| thread.archived_at = Some(event::iso(NOW).unwrap()));
    let inactive = f
        .scheduler
        .invoke(
            caller.clone(),
            tool("run_scheduled_task_now", json!({"taskId":"visible"})),
        )
        .await
        .unwrap_err();
    assert_eq!(
        inactive.message,
        "The calling provider no longer owns an active thread run."
    );
    // CRUD requires capability + projection, not manual-run liveness/full-access.
    f.scheduler
        .invoke(
            caller,
            tool(
                "update_scheduled_task",
                json!({"scheduledTaskId":"visible","title":"Paused title","enabled":false}),
            ),
        )
        .await
        .unwrap();
    assert!(!f.scheduler.load(&"visible".into()).unwrap().enabled);
    assert!(f.recorder.runs.lock().unwrap().is_empty());
}

#[tokio::test]
async fn actual_toolkit_structured_string_validation_capability_and_result_shapes() {
    use crate::HarnessRegistry;
    use crate::mcp::{auth::InvocationScope, toolkit::Toolkit};
    let f = Fixture::new();
    let caller = f.parent();
    let toolkit = Toolkit::new(Arc::new(HarnessRegistry::new()));
    toolkit.set_scheduler(f.scheduler.clone());
    let scope = InvocationScope {
        environment_id: "host".into(),
        selection: serde_json::from_value(json!({"instanceId":"mock","model":"mock-1"})).unwrap(),
        caller,
        capabilities: ["orchestration".into()].into(),
        issued_at: 0,
        task_id: None,
    };
    for schedule in [
        json!({"type":"interval","everyMs":60000}),
        json!("{\"type\":\"interval\",\"everyMs\":60000}"),
    ] {
        let reply = toolkit.request(scope.clone(),json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"schedule_task","arguments":{"prompt":"Task","schedule":schedule}}
        })).await.unwrap();
        assert!(
            reply["result"]["isError"].as_bool() != Some(true),
            "{reply}"
        );
        let _: zeron_proto::orchestration_mcp::ScheduleTaskResult =
            serde_json::from_value(reply["result"]["structuredContent"].clone()).unwrap();
    }
    let denied_scope = InvocationScope {
        capabilities: Default::default(),
        ..scope.clone()
    };
    let before = f.scheduler.list().unwrap().tasks.len();
    let denied = toolkit.request(denied_scope,json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"schedule_task","arguments":{"prompt":"Task","schedule":{"type":"interval","everyMs":60000}}}
    })).await.unwrap();
    assert_eq!(
        denied["result"]["structuredContent"]["code"],
        "capability_denied"
    );
    assert_eq!(f.scheduler.list().unwrap().tasks.len(), before);
    for bad in [
        json!({"type":"interval","everyMs":59999}),
        json!({"type":"fixed_time","timeOfDay":"9:0"}),
        json!({"type":"fixed_time","timeOfDay":"09:00","weekdays":[7]}),
    ] {
        let reply = toolkit.request(scope.clone(),json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"schedule_task","arguments":{"prompt":"Task","schedule":bad}}
        })).await.unwrap();
        // T3's failureMode="return": parameter errors are structured AiError
        // results, not MCP isError=true or JSON-RPC framework failures.
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        assert_eq!(
            reply["result"]["structuredContent"]["_tag"], "AiError",
            "{reply}"
        );
    }
    let task = f.task("manual");
    let result = f
        .scheduler
        .invoke(
            scope.caller.clone(),
            tool("run_scheduled_task_now", json!({"taskId":task.id})),
        )
        .await
        .unwrap();
    let _: zeron_proto::orchestration_mcp::RunScheduledTaskNowResult =
        serde_json::from_value(result).unwrap();
    let scoped_to_node = InvocationScope {
        task_id: Some("node:delegated-task".into()),
        ..scope
    };
    let reply = toolkit
        .request(
            scoped_to_node,
            json!({
                "jsonrpc":"2.0","id":2,"method":"tools/call",
                "params":{"name":"run_scheduled_task_now","arguments":{"taskId":task.id}}
            }),
        )
        .await
        .unwrap();
    assert_eq!(reply["result"]["structuredContent"]["taskId"], task.id.0);
}

#[tokio::test]
async fn lost_claim_response_releases_only_that_claim_and_never_refires() {
    let f = Fixture::new();
    let task = f.due("task", 0);
    f.scheduler
        .store
        .inject_failure(WriteBoundary::AfterCommit, 1);
    assert!(f.scheduler.run_now(&task.id).await.is_err());
    assert!(f.recorder.runs.lock().unwrap().is_empty());
    let current = f.scheduler.load(&task.id).unwrap();
    assert_eq!(current.last_run_status, ScheduledTaskRunStatus::Failed);
    assert_eq!(current.run_count, 1);
    assert_eq!(f.claims()[0].2, "failed");
    f.scheduler.tick().await.unwrap();
    assert!(f.recorder.runs.lock().unwrap().is_empty());
}

#[tokio::test]
async fn raw_corrupt_due_rows_and_next_run_columns_do_not_poison_poll() {
    let f = Fixture::new();
    for id in ["a-bad-date", "b-bad-json", "c-invalid-id", "z-valid"] {
        f.due(id, 0);
    }
    f.scheduler.store.write(|tx| {
        tx.execute("UPDATE orchestration_scheduled_tasks SET next_run_at='!broken' WHERE task_id='a-bad-date'",[])?;
        tx.execute("UPDATE orchestration_scheduled_tasks SET task_json='{}' WHERE task_id='b-bad-json'",[])?;
        tx.execute("UPDATE orchestration_scheduled_tasks SET task_id=' ' WHERE task_id='c-invalid-id'",[])?;
        Ok(())
    }).unwrap();
    f.scheduler.tick().await.unwrap();
    assert_eq!(
        f.recorder
            .runs
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.task.id.0.clone())
            .collect::<Vec<_>>(),
        ["z-valid"]
    );
}

#[tokio::test]
async fn ui_owner_api_partial_update_watch_and_no_resurrection() {
    use crate::orchestration::ui_scheduler;
    use zeron_rpc::scheduled_tasks::methods;
    let f = Fixture::new();
    assert!(
        ui_scheduler::dispatch(&f.scheduler, methods::LIST, json!({"ownerHostId":"viewer"}))
            .await
            .is_err()
    );
    let created = ui_scheduler::dispatch(&f.scheduler, methods::CREATE, json!({
        "ownerHostId":"host","input":{
            "title":"User task","prompt":"Run","enabled":true,"schedule":{"type":"interval","everyMs":60000},
            "projectId":"project","workspaceStrategy":{"type":"root"},"modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default"
        }
    })).await.unwrap();
    let zeron_rpc::RpcReply::Value(created) = created else {
        panic!()
    };
    let created: ScheduledTaskViewResult = serde_json::from_value(created).unwrap();
    let id = created.task.task.id;
    assert_eq!(created.task.cadence, "Every minute");
    let watch = ui_scheduler::dispatch(
        &f.scheduler,
        methods::WATCH,
        json!({"ownerHostId":"host","projectId":"project"}),
    )
    .await
    .unwrap();
    let zeron_rpc::RpcReply::Stream(mut stream) = watch else {
        panic!()
    };
    use futures::StreamExt;
    assert_eq!(
        stream.next().await.unwrap()["tasks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let before = f.scheduler.load(&id).unwrap().next_run_at;
    f.clock.set(NOW + 10000);
    ui_scheduler::dispatch(
        &f.scheduler,
        methods::UPDATE,
        json!({"ownerHostId":"host","id":id,"title":"New title"}),
    )
    .await
    .unwrap();
    assert_eq!(f.scheduler.load(&id).unwrap().next_run_at, before);
    let changed = stream.next().await.unwrap();
    assert_eq!(changed["tasks"][0]["title"], "New title");
    assert_eq!(changed["tasks"][0]["lastRun"]["status"], "never");
    ui_scheduler::dispatch(
        &f.scheduler,
        methods::RUN_NOW,
        json!({"ownerHostId":"host","id":id}),
    )
    .await
    .unwrap();
    assert_eq!(f.scheduler.load(&id).unwrap().run_count, 1);
    ui_scheduler::dispatch(
        &f.scheduler,
        methods::DELETE,
        json!({"ownerHostId":"host","id":id}),
    )
    .await
    .unwrap();
    let error = ui_scheduler::dispatch(
        &f.scheduler,
        methods::UPDATE,
        json!({"ownerHostId":"host","id":id,"title":"Resurrect"}),
    )
    .await
    .err()
    .unwrap();
    assert!(error.to_string().contains("Schedule task not found."));
    assert!(f.scheduler.find(&id).unwrap().is_none());
}
