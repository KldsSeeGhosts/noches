//! Durable host scheduler. Success records dispatch acceptance, never completion
//! of a provider turn. SQLite claims fence overlap and restart uncertainty.
pub mod schedule;

use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use futures::FutureExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;
use zeron_proto::orchestration::*;

use super::{Error, Result, Store, event};
use schedule::{Clock, HostClock};

pub const TICK_MS: u64 = 5_000;
const RESTART_ERROR: &str = "Run was interrupted by a server restart.";

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct SchedulerError {
    #[serde(rename = "_tag")]
    pub tag: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<ScheduledTaskId>,
}

impl SchedulerError {
    fn new(message: impl Into<String>, id: Option<&ScheduledTaskId>) -> Self {
        Self {
            tag: "ScheduledTaskError".into(),
            message: message.into(),
            task_id: id.cloned(),
        }
    }
}
pub type SchedulerResult<T> = std::result::Result<T, SchedulerError>;

/// TODO(merge-threads): adapt the threads slice's normal sendToThread(mode=auto)
/// and launch service here. Bound sends must honour the *current* worktree;
/// unbound sends launch a fresh top-level worktree with the stored strategy.
/// Carry scheduledTaskId, actor/source and these stable command/message IDs
/// into the ordinary command queue. Never start a harness directly.
#[async_trait]
pub trait ScheduledTaskDispatch: Send + Sync + 'static {
    async fn dispatch(&self, run: ScheduledDispatch) -> std::result::Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct ScheduledDispatch {
    pub task: ScheduledTask,
    pub claim_id: String,
    pub command_id: CommandId,
    pub message_id: MessageId,
    pub trigger: String,
}

pub struct UnavailableDispatch;
#[async_trait]
impl ScheduledTaskDispatch for UnavailableDispatch {
    async fn dispatch(&self, _: ScheduledDispatch) -> std::result::Result<(), String> {
        Err("The operation could not be completed.".into())
    }
}

pub struct Scheduler {
    pub store: Store,
    pub clock: Arc<dyn Clock>,
    dispatcher: RwLock<Arc<dyn ScheduledTaskDispatch>>,
    tick_lane: Mutex<()>,
    changes: watch::Sender<u64>,
}

impl Scheduler {
    pub fn new(store: Store, dispatcher: Arc<dyn ScheduledTaskDispatch>) -> Self {
        Self::with_clock(store, dispatcher, Arc::new(HostClock))
    }

    pub fn with_clock(
        store: Store,
        dispatcher: Arc<dyn ScheduledTaskDispatch>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let (changes, _) = watch::channel(0);
        Self {
            store,
            clock,
            dispatcher: RwLock::new(dispatcher),
            tick_lane: Mutex::new(()),
            changes,
        }
    }

    pub fn set_dispatcher(&self, dispatcher: Arc<dyn ScheduledTaskDispatch>) {
        *self
            .dispatcher
            .write()
            .unwrap_or_else(PoisonError::into_inner) = dispatcher;
    }

    /// Subscribe before taking a snapshot. Signals coalesce; no growing queue.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }

    fn changed(&self) {
        self.changes
            .send_modify(|version| *version = version.wrapping_add(1));
    }

    pub(super) fn next(&self, task: &ScheduledTask, now: i64) -> Option<String> {
        task.enabled
            .then(|| self.clock.next_run_at(&task.schedule, now))
            .flatten()
            .and_then(|next| event::iso(next).ok())
    }

    pub fn list(&self) -> SchedulerResult<ScheduledTaskListResult> {
        self.store
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT * FROM orchestration_scheduled_tasks
                     ORDER BY updated_at DESC, task_id ASC",
                )?;
                let rows = stmt.query_map([], stored_row)?;
                let tasks = rows
                    .map(|row| decode_row(row?))
                    .collect::<Result<Vec<_>>>()?;
                Ok(ScheduledTaskListResult { tasks })
            })
            .map_err(|_| SchedulerError::new("Could not list schedule tasks.", None))
    }

    pub fn find(&self, id: &ScheduledTaskId) -> SchedulerResult<Option<ScheduledTask>> {
        self.store
            .read(|conn| find(conn, id))
            .map_err(|_| SchedulerError::new("Could not load schedule task.", Some(id)))
    }

    pub fn load(&self, id: &ScheduledTaskId) -> SchedulerResult<ScheduledTask> {
        self.find(id)?
            .ok_or_else(|| SchedulerError::new("Schedule task not found.", Some(id)))
    }

    /// A keyed create is an upsert, not a receipt-only replay. Like T3, a
    /// changed payload under the same key edits the definition and preserves
    /// run history; a different provider session has a different resolved ID.
    pub fn upsert(&self, input: ScheduledTaskUpsertInput) -> SchedulerResult<ScheduledTask> {
        let id = input.id.as_ref().cloned().unwrap_or_else(|| {
            ScheduledTaskId(format!(
                "scheduled-task:{}",
                input
                    .command_id
                    .as_ref()
                    .map(|id| id.0.clone())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            ))
        });
        // Validate writes even when called through a typed Rust/UI API.
        let input: ScheduledTaskUpsertInput = normalize_contract(
            "ScheduledTaskUpsertInput",
            serde_json::to_value(input)
                .map_err(|_| SchedulerError::new("Could not save schedule task.", Some(&id)))?,
        )
        .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
        .map_err(|_| SchedulerError::new("Could not save schedule task.", Some(&id)))?;
        self.mutate(&id, "Could not save schedule task.", |existing, now| {
            if input.require_existing.as_ref() == Some(&true) && existing.is_none() {
                return Err(Error::Invariant("Schedule task not found.".into()));
            }
            let schedule: ScheduledTaskSchedule =
                serde_json::from_value(serde_json::to_value(&input.schedule)?)?;
            let unchanged = existing.as_ref().is_some_and(|old| {
                old.enabled == input.enabled && schedule::same_schedule(&old.schedule, &schedule)
            });
            let mut task = ScheduledTask {
                id: id.clone(),
                title: input.title.clone(),
                prompt: input.prompt.clone(),
                enabled: input.enabled,
                schedule,
                project_id: input.project_id.clone(),
                thread_id: input.thread_id.as_ref().cloned().flatten(),
                workspace_strategy: input.workspace_strategy.clone(),
                model_selection: input.model_selection.clone(),
                runtime_mode: input.runtime_mode,
                interaction_mode: input.interaction_mode,
                created_by: existing
                    .as_ref()
                    .map(|task| task.created_by.clone())
                    .or_else(|| input.created_by.as_ref().cloned())
                    .unwrap_or(OrchestrationV2Actor::User),
                creation_source: input
                    .creation_source
                    .as_ref()
                    .cloned()
                    .unwrap_or(OrchestrationV2CreationSource::Web),
                created_at: existing
                    .as_ref()
                    .map(|task| task.created_at.clone())
                    .unwrap_or(event::iso(now)?),
                updated_at: event::iso(now)?,
                next_run_at: None,
                last_run_at: existing.as_ref().and_then(|task| task.last_run_at.clone()),
                last_run_status: existing
                    .as_ref()
                    .map(|task| task.last_run_status.clone())
                    .unwrap_or(ScheduledTaskRunStatus::Never),
                last_run_error: existing
                    .as_ref()
                    .and_then(|task| task.last_run_error.clone()),
                run_count: existing.as_ref().map(|task| task.run_count).unwrap_or(0),
            };
            task.next_run_at = if unchanged {
                existing.and_then(|task| task.next_run_at)
            } else {
                self.next(&task, now)
            };
            Ok(task)
        })
    }

    /// Atomic partial edit: never rewrites history from an old form snapshot.
    pub(super) fn mutate(
        &self,
        id: &ScheduledTaskId,
        error: &str,
        edit: impl FnOnce(Option<ScheduledTask>, i64) -> Result<ScheduledTask>,
    ) -> SchedulerResult<ScheduledTask> {
        let result = self.store.write(|tx| {
            let task = edit(find(tx, id)?, self.clock.now_ms())?;
            save(tx, &task)?;
            Ok(task)
        });
        let task = result.map_err(|cause| {
            let message = if matches!(&cause, Error::Invariant(s) if s == "Schedule task not found.") {
                "Schedule task not found."
            } else {
                error
            };
            SchedulerError::new(message, Some(id))
        })?;
        self.changed();
        Ok(task)
    }

    pub fn delete(&self, id: &ScheduledTaskId) -> SchedulerResult<ScheduledTaskDeleteResult> {
        self.store
            .write(|tx| {
                tx.execute(
                    "DELETE FROM orchestration_scheduled_tasks WHERE task_id=?1",
                    [&id.0],
                )?;
                Ok(())
            })
            .map_err(|_| SchedulerError::new("Could not delete schedule task.", Some(id)))?;
        self.changed();
        Ok(ScheduledTaskDeleteResult { id: id.clone() })
    }

    /// Startup under host InstanceLock. Dispatch may have occurred; never
    /// replay a running claim. Corrupt definitions are released independently.
    pub fn recover(&self) -> Result<()> {
        let now = self.clock.now_ms();
        let stamp = event::iso(now)?;
        self.store.write(|tx| {
            let mut stmt = tx.prepare(
                "SELECT * FROM orchestration_scheduled_tasks
                 WHERE last_run_status='running'",
            )?;
            let rows = stmt
                .query_map([], stored_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(stmt);
            for row in rows {
                let id = row.id.clone();
                match decode_row(row) {
                    Ok(mut task) => {
                        task.last_run_status = ScheduledTaskRunStatus::Failed;
                        task.last_run_error = Some(RESTART_ERROR.into());
                        task.updated_at = stamp.clone();
                        task.next_run_at = self.next(&task, now);
                        task.run_count += 1;
                        save(tx, &task)?;
                    }
                    Err(error) => {
                        tracing::warn!(task_id=%id, %error, "Recovering undecodable schedule task row without rescheduling");
                        tx.execute(
                            "UPDATE orchestration_scheduled_tasks
                             SET last_run_status='failed',updated_at=?2,last_run_error=?3,
                                 run_count=run_count+1 WHERE task_id=?1",
                            params![id, stamp, RESTART_ERROR],
                        )?;
                    }
                }
                tx.execute(
                    "UPDATE orchestration_scheduled_tasks SET active_claim_id=NULL WHERE task_id=?1",
                    [id],
                )?;
            }
            tx.execute(
                "UPDATE orchestration_scheduled_runs SET status='failed',error=?1,completed_at=?2
                 WHERE status='running'",
                params![RESTART_ERROR, stamp],
            )?;
            Ok(())
        })?;
        self.changed();
        Ok(())
    }

    /// Due IDs only; decode/revalidate each *fresh* row at the claim boundary.
    /// Corrupt rows cannot poison unrelated schedules. Tick calls never overlap
    /// and busy ticks are dropped rather than queued into a backlog.
    pub async fn tick(&self) -> Result<()> {
        let Ok(_lane) = self.tick_lane.try_lock() else {
            return Ok(());
        };
        let stamp = event::iso(self.clock.now_ms())?;
        let ids = self.store.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT task_id FROM orchestration_scheduled_tasks
                 WHERE enabled=1 AND next_run_at IS NOT NULL AND next_run_at<=?1
                   AND last_run_status<>'running' ORDER BY next_run_at ASC,task_id ASC",
            )?;
            Ok(stmt
                .query_map([stamp], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })?;
        for id in ids {
            if let Err(error) = self.run(&ScheduledTaskId(id.clone()), "scheduled").await {
                tracing::warn!(task_id=%id, %error, "Scheduled task run failed");
            }
        }
        Ok(())
    }

    pub fn spawn(self: &Arc<Self>, stop: CancellationToken) -> tokio::task::JoinHandle<()> {
        let service = self.clone();
        tokio::spawn(async move {
            // interval's first tick is immediate; Skip prevents catch-up ticks.
            let mut ticks = tokio::time::interval(Duration::from_millis(TICK_MS));
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut running: Option<futures::future::BoxFuture<'static, Result<()>>> = None;
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    _ = ticks.tick() => {
                        if running.is_none() {
                            let source = service.clone();
                            running = Some(Box::pin(async move { source.tick().await }));
                        }
                    }
                    result = async { running.as_mut().expect("guarded source").await },
                        if running.is_some() => {
                        running = None;
                        if let Err(error) = result {
                            tracing::warn!(%error, "Could not list schedule tasks");
                        }
                    }
                }
            }
        })
    }

    pub async fn run_now(&self, id: &ScheduledTaskId) -> SchedulerResult<ScheduledTask> {
        self.load(id)?;
        self.run(id, "manual")
            .await
            .map_err(|_| SchedulerError::new("Could not run schedule task.", Some(id)))?
            .ok_or_else(|| SchedulerError::new("Could not run schedule task.", Some(id)))
    }

    async fn run(&self, id: &ScheduledTaskId, trigger: &str) -> Result<Option<ScheduledTask>> {
        let claim_id = uuid::Uuid::new_v4().to_string();
        let claimed = self.store.write(|tx| {
            let Some(mut task) = find(tx, id)? else {
                return if trigger == "manual" {
                    Err(Error::Invariant("Schedule task not found.".into()))
                } else {
                    Ok(None)
                };
            };
            if task.last_run_status == ScheduledTaskRunStatus::Running {
                return if trigger == "manual" {
                    Err(Error::Invariant("Schedule task is already running.".into()))
                } else {
                    Ok(None)
                };
            }
            let now = self.clock.now_ms();
            if trigger == "scheduled" {
                let due = task.next_run_at.as_deref().and_then(parse_epoch);
                if !task.enabled || due.is_none_or(|due| due > now) {
                    return Ok(None);
                }
                if schedule::missed_fixed(&task.schedule, due.unwrap(), now) {
                    task.next_run_at = self.next(&task, now);
                    task.updated_at = event::iso(now)?;
                    save(tx, &task)?;
                    return Ok(None);
                }
            }
            // UUID makes *every* manual invocation distinct, even under a
            // frozen clock. Command/message identity survives in the claim.
            let fire_key = format!("{}:{now}:{trigger}:{claim_id}", id.0);
            let run = ScheduledDispatch {
                task: task.clone(),
                claim_id: claim_id.clone(),
                command_id: CommandId(format!("scheduled-task:{fire_key}")),
                message_id: MessageId(format!("scheduled-task-message:{fire_key}")),
                trigger: trigger.into(),
            };
            task.updated_at = event::iso(now)?;
            task.last_run_at = Some(task.updated_at.clone());
            task.last_run_status = ScheduledTaskRunStatus::Running;
            task.last_run_error = None;
            save(tx, &task)?;
            tx.execute(
                "UPDATE orchestration_scheduled_tasks SET active_claim_id=?2 WHERE task_id=?1",
                params![id.0, claim_id],
            )?;
            tx.execute(
                "INSERT INTO orchestration_scheduled_runs
                 (claim_id,task_id,trigger,command_id,message_id,host_id,task_json,started_at,status)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'running')",
                params![claim_id, id.0, trigger, run.command_id.0, run.message_id.0,
                    self.store.host_id.as_ref(), serde_json::to_string(&run.task)?, task.updated_at],
            )?;
            Ok(Some(run))
        });
        let claimed = match claimed {
            Ok(claimed) => claimed,
            Err(error) => {
                // A lost response after COMMIT must not leave a claim stuck
                // until reboot. Resolve only *this* attempt's unique claim.
                let accepted = self.store.read(|conn| {
                    conn.query_row(
                        "SELECT task_json,command_id,message_id,trigger
                         FROM orchestration_scheduled_runs WHERE claim_id=?1 AND status='running'",
                        [&claim_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                            ))
                        },
                    )
                    .optional()?
                    .map(|(payload, command, message, trigger)| {
                        Ok(ScheduledDispatch {
                            task: serde_json::from_str(&payload)?,
                            claim_id: claim_id.clone(),
                            command_id: CommandId(command),
                            message_id: MessageId(message),
                            trigger,
                        })
                    })
                    .transpose()
                });
                if let Ok(Some(run)) = accepted {
                    self.release(&run);
                }
                return Err(error);
            }
        };
        self.changed();
        let Some(run) = claimed else {
            return Ok(None);
        };
        let mut guard = RunGuard {
            scheduler: self,
            run: &run,
            armed: true,
        };
        let dispatcher = self
            .dispatcher
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let outcome = std::panic::AssertUnwindSafe(dispatcher.dispatch(run.clone()))
            .catch_unwind()
            .await;
        let error = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error),
            Err(_) => Some("The operation could not be completed.".into()),
        };
        let completed = self.complete(&run, error)?;
        guard.armed = false;
        Ok(Some(completed))
    }

    fn complete(&self, run: &ScheduledDispatch, error: Option<String>) -> Result<ScheduledTask> {
        let now = self.clock.now_ms();
        let completed = self.store.write(|tx| {
            // Use current schedule, prompt and enabled state after dispatch.
            // Never resurrect a deleted definition or stamp its recreation.
            let current = find(tx, &run.task.id)?;
            let mut task = current.clone().unwrap_or_else(|| run.task.clone());
            task.updated_at = event::iso(now)?;
            task.last_run_at = Some(tx.query_row(
                "SELECT started_at FROM orchestration_scheduled_runs WHERE claim_id=?1",
                [&run.claim_id], |row| row.get(0),
            )?);
            task.last_run_status = if error.is_some() {
                ScheduledTaskRunStatus::Failed
            } else {
                ScheduledTaskRunStatus::Succeeded
            };
            task.last_run_error = error.clone();
            task.next_run_at = self.next(&task, now);
            task.run_count += 1;
            let owns_claim: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM orchestration_scheduled_tasks
                 WHERE task_id=?1 AND active_claim_id=?2 AND last_run_status='running')",
                params![task.id.0, run.claim_id], |row| row.get(0),
            )?;
            if current.is_some() && owns_claim {
                save(tx, &task)?;
                tx.execute(
                    "UPDATE orchestration_scheduled_tasks SET active_claim_id=NULL WHERE task_id=?1",
                    [&task.id.0],
                )?;
            }
            tx.execute(
                "UPDATE orchestration_scheduled_runs SET status=?2,error=?3,completed_at=?4
                 WHERE claim_id=?1 AND status='running'",
                params![run.claim_id, if error.is_some() { "failed" } else { "succeeded" },
                    error, task.updated_at],
            )?;
            Ok(task)
        })?;
        self.changed();
        Ok(completed)
    }

    fn release(&self, run: &ScheduledDispatch) {
        if let Err(error) =
            self.complete(run, Some("Schedule task dispatch was interrupted.".into()))
        {
            // Decode/bookkeeping failures must still clear running state.
            let now = self.clock.now_ms();
            let fallback = self.store.write(|tx| {
                tx.execute(
                    "UPDATE orchestration_scheduled_tasks SET last_run_status='failed',
                     active_claim_id=NULL,next_run_at=?3,updated_at=?4,last_run_error=?5,
                     run_count=run_count+1 WHERE task_id=?1 AND active_claim_id=?2",
                    params![run.task.id.0, run.claim_id, self.next(&run.task, now),
                        event::iso(now)?, "Could not record schedule task run."],
                )?;
                tx.execute(
                    "UPDATE orchestration_scheduled_runs SET status='failed',error=?2,completed_at=?3
                     WHERE claim_id=?1 AND status='running'",
                    params![run.claim_id, "Could not record schedule task run.", event::iso(now)?],
                )?;
                Ok(())
            });
            tracing::warn!(%error, ?fallback, "Could not release stuck schedule task run");
            self.changed();
        }
    }
}

struct RunGuard<'a> {
    scheduler: &'a Scheduler,
    run: &'a ScheduledDispatch,
    armed: bool,
}
impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.scheduler.release(self.run);
        }
    }
}

pub(super) fn parse_epoch(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.timestamp_millis())
}

struct StoredTask {
    id: String,
    project: String,
    enabled: bool,
    next_run_at: Option<String>,
    updated_at: String,
    status: String,
    last_run_at: Option<String>,
    error: Option<String>,
    count: i64,
    payload: String,
}

fn stored_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredTask> {
    Ok(StoredTask {
        id: row.get("task_id")?,
        project: row.get("project_id")?,
        enabled: row.get::<_, i64>("enabled")? == 1,
        next_run_at: row.get("next_run_at")?,
        updated_at: row.get("updated_at")?,
        status: row.get("last_run_status")?,
        last_run_at: row.get("last_run_at")?,
        error: row.get("last_run_error")?,
        count: row.get("run_count")?,
        payload: row.get("task_json")?,
    })
}

fn decode_row(row: StoredTask) -> Result<ScheduledTask> {
    let mut value: serde_json::Value = serde_json::from_str(&row.payload)?;
    if !value.is_object() {
        return Err(Error::Invariant(
            "Could not decode schedule task row.".into(),
        ));
    }
    // Operational columns are authoritative, including recovery of a corrupt
    // definition. Never trust stale duplicated status/cadence from task_json.
    value["id"] = serde_json::json!(row.id);
    value["projectId"] = serde_json::json!(row.project);
    value["enabled"] = serde_json::json!(row.enabled);
    value["nextRunAt"] = serde_json::json!(row.next_run_at);
    value["updatedAt"] = serde_json::json!(row.updated_at);
    value["lastRunStatus"] = serde_json::json!(row.status);
    value["lastRunAt"] = serde_json::json!(row.last_run_at);
    value["lastRunError"] = serde_json::json!(row.error);
    value["runCount"] = serde_json::json!(row.count);
    let value = normalize_contract("ScheduledTask", value)
        .map_err(|_| Error::Invariant("Could not decode schedule task row.".into()))?;
    Ok(serde_json::from_value(value)?)
}

fn find(conn: &Connection, id: &ScheduledTaskId) -> Result<Option<ScheduledTask>> {
    conn.query_row(
        "SELECT * FROM orchestration_scheduled_tasks WHERE task_id=?1",
        [&id.0],
        stored_row,
    )
    .optional()?
    .map(decode_row)
    .transpose()
}

fn save(conn: &Connection, task: &ScheduledTask) -> Result<()> {
    conn.execute(
        "INSERT INTO orchestration_scheduled_tasks
         (task_id,project_id,enabled,next_run_at,updated_at,last_run_status,task_json,
          last_run_at,last_run_error,run_count)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
         ON CONFLICT(task_id) DO UPDATE SET project_id=excluded.project_id,
         enabled=excluded.enabled,next_run_at=excluded.next_run_at,updated_at=excluded.updated_at,
         last_run_status=excluded.last_run_status,task_json=excluded.task_json,
         last_run_at=excluded.last_run_at,last_run_error=excluded.last_run_error,run_count=excluded.run_count",
        params![
            task.id.0,
            task.project_id.0,
            task.enabled,
            task.next_run_at,
            task.updated_at,
            serde_json::to_value(&task.last_run_status)?
                .as_str()
                .unwrap(),
            serde_json::to_string(task)?,
            task.last_run_at,
            task.last_run_error,
            task.run_count
        ],
    )?;
    Ok(())
}
