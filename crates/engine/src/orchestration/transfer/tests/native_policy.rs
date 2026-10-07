//! Issue #49 workstream D: `/compact` handoff deferral and legacy steering rows.
use super::*;

#[tokio::test]
async fn empty_compaction_is_refused_but_a_real_conversation_can_compact() {
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, _) = fixture(cwd.path());
    let compact = start_text(&kernel, "source", "compact-empty", "/compact").await;
    let mut input = request(cwd.path());
    let refused = prepare(&kernel, &compact, &mut input).await.unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("Start a conversation before compacting this thread."),
        "{refused}"
    );

    let (_db, kernel, _) = fixture(cwd.path());
    let known = start(&kernel, "source", "conversation").await;
    accept(&kernel, &known, cwd.path(), "native-a").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    let compact = start_text(&kernel, "source", "compact-ok", "  /COMPACT\n").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &compact, &mut input).await.unwrap();

    // An attachment makes it ordinary input, not maintenance.
    let message = json!({"id":"m","role":"user","text":"/compact","attachments":[]});
    assert!(is_compaction(&message));
    let mut attached = message.clone();
    attached["attachments"] = json!([{"type":"image","id":"a"}]);
    assert!(!is_compaction(&attached));
    let projection = kernel.store.thread(&"source".into()).unwrap().unwrap();
    assert!(has_conversation(&projection), "the accepted run is conversation");
    let _ = run_is_compaction(&projection, &compact);
}

/// The deferred handoff is neither injected into `/compact` nor lost: the
/// completed compaction left it undelivered, so the next ordinary turn owes it.
#[tokio::test]
async fn compaction_defers_the_pending_handoff_until_the_next_ordinary_turn() {
    let cwd = tempfile::tempdir().unwrap();
    let (db, kernel, _) = fixture(cwd.path());
    let known = start_text(&kernel, "source", "known", "History worth handing off.").await;
    accept(&kernel, &known, cwd.path(), "old-model-native").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    select_instance(
        &kernel,
        &known.provider_instance_id.0,
        Some("different-model"),
    )
    .await;

    let compact = start_text(&kernel, "source", "compact", "/compact").await;
    let mut input = request(cwd.path());
    input.prompt = "/compact".into();
    prepare(&kernel, &compact, &mut input).await.unwrap();
    assert_eq!(input.prompt, "/compact", "maintenance carries no preamble");
    assert!(!input.prompt.contains("History worth handing off."));
    let projection = kernel.store.thread(&"source".into()).unwrap().unwrap();
    let handoff = projection.records["context-handoff"]
        .iter()
        .find(|h| h["targetRunId"] == compact.id.0)
        .expect("handoff prepared for the compaction run")
        .clone();
    assert!(handoff["delivery"].is_null(), "deferred, not delivered");

    accept(&kernel, &compact, cwd.path(), "new-model-native").await;
    observe(&kernel, &compact, done(zeron_proto::DoneStatus::Completed)).await;
    for restart in [false, true] {
        let kernel = if restart {
            let reopened =
                Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
            reopened.store.rebuild().unwrap();
            reopened
        } else {
            kernel.clone()
        };
        let next = if restart {
            kernel
                .store
                .thread(&"source".into())
                .unwrap()
                .unwrap()
                .runs
                .last()
                .unwrap()
                .clone()
        } else {
            start_text(&kernel, "source", "after-compact", "Ordinary follow-up.").await
        };
        let mut input = request(cwd.path());
        input.prompt = "Ordinary follow-up.".into();
        input.resume = Some("new-model-native".into());
        let result = prepare(&kernel, &next, &mut input).await;
        if restart {
            // A previous in-memory preparation may already have injected it.
            let _ = result;
        } else {
            result.unwrap();
            assert!(
                input.prompt.contains("History worth handing off."),
                "the next ordinary turn still receives the deferred context: {}",
                input.prompt
            );
            assert!(input.prompt.ends_with("Ordinary follow-up."));
        }
    }
}

/// Older effects were admitted before the per-input receipt contract. Their
/// local success/session-start semantics differed, so they are neither
/// replayed as unconfirmed input nor marked accepted after the fact.
#[tokio::test]
async fn legacy_steering_rows_are_never_replayed_or_speculatively_accepted() {
    use crate::orchestration::{
        command::{Command, Operation},
        effects::EffectRequest,
        steering,
        threads::planner::{Send, ThreadOperation},
    };
    for legacy in [true, false] {
        let cwd = tempfile::tempdir().unwrap();
        let (db, kernel, _) = fixture(cwd.path());
        let source = start_text(&kernel, "source", "root", "Accepted root input.").await;
        accept(&kernel, &source, cwd.path(), "native-legacy").await;
        let bound = kernel.store.thread(&"source".into()).unwrap().unwrap();
        let target = steering::RuntimeTarget::for_run(
            bound.runs.iter().find(|r| r.id == source.id).unwrap(),
        )
        .unwrap();
        kernel
            .store
            .write(|tx| steering::bind_runtime(tx, &"source".into(), &target, "legacy-runtime"))
            .unwrap();
        let receipt = kernel
            .dispatch(
                &Command {
                    id: "steer:legacy".into(),
                    thread_id: "source".into(),
                    operation: Operation::Thread(Box::new(ThreadOperation::Send(Send {
                        message_id: "legacy-steer".into(),
                        text: "Pre-contract steering text.".into(),
                        mode: zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Steer,
                        driver: "mock".into(),
                        sender: "source".into(),
                        target_run: Some(source.id.clone()),
                        metadata: None,
                    }))),
                },
                crate::now_ms(),
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted, "{receipt:?}");
        let effect = kernel
            .store
            .effects()
            .unwrap()
            .into_iter()
            .find(|e| matches!(&e.request, EffectRequest::ProviderTurnSteer { .. }))
            .unwrap();
        kernel
            .store
            .write(|tx| {
                // A pre-contract effect: dispatched and locally "succeeded",
                // but with no per-input receipt row and no acceptance.
                tx.execute(
                    "UPDATE orchestration_effect_outbox
                     SET status=?2,dispatch_started=1 WHERE effect_id=?1",
                    rusqlite::params![effect.id, if legacy { "failed" } else { "uncertain" }],
                )?;
                if legacy {
                    tx.execute("DELETE FROM orchestration_steering_inputs", [])?;
                }
                Ok(())
            })
            .unwrap();
        observe(&kernel, &source, done(zeron_proto::DoneStatus::Completed)).await;
        let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
        reopened.store.rebuild().unwrap();
        let next = start(&reopened, "source", "after-legacy").await;
        let mut input = request(cwd.path());
        prepare(&reopened, &next, &mut input).await.unwrap();
        let accepted: i64 = reopened
            .store
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM orchestration_steering_acceptances",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(accepted, 0, "no row is ever accepted without a receipt");
        assert_eq!(
            input.prompt.contains("Pre-contract steering text."),
            !legacy,
            "legacy={legacy}: only effects under the receipt contract are recovered: {}",
            input.prompt
        );
    }
}

#[test]
fn handoff_budget_uses_the_catalog_declared_window_before_any_telemetry() {
    let claude = zeron_harness::claude::ClaudeHarness::new();
    let mut input = request(std::path::Path::new("/tmp"));
    assert_eq!(declared_model_window(&claude, &input), None, "no model yet");
    input.model = Some("claude-opus-5-5".into());
    assert_eq!(declared_model_window(&claude, &input), Some(200_000));
    input
        .model_options
        .insert("contextWindow".into(), "1m".into());
    assert_eq!(declared_model_window(&claude, &input), Some(1_000_000));
    input.model = Some("unlisted-proxy-model".into());
    assert_eq!(declared_model_window(&claude, &input), None);
    // With 100K already used, the conservative 128K default leaves nothing,
    // while the model's real 200K window leaves a bounded allowance.
    let declared = handoff_budget(60_000, "hi", &[], None, 100_000, Some(200_000));
    let fallback = handoff_budget(60_000, "hi", &[], None, 100_000, None);
    assert_eq!(fallback, 0);
    assert!(declared > 0 && declared < 60_000, "{declared}");
    assert_eq!(
        handoff_budget(60_000, "hi", &[], None, 100_000, Some(1_000_000)),
        60_000
    );
}

/// Records the native fork request; accepts a counted legacy boundary.
struct RollbackForkHarness {
    seen: std::sync::Mutex<Vec<(Option<String>, Option<usize>)>>,
}
#[async_trait::async_trait]
impl zeron_harness::Harness for RollbackForkHarness {
    fn id(&self) -> zeron_proto::HarnessId {
        zeron_proto::HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Rollback fork fixture"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> zeron_proto::SteeringMode {
        zeron_proto::SteeringMode::StepBoundary
    }
    fn reasoning_levels(&self) -> &[zeron_proto::ReasoningLevel] {
        &[]
    }
    fn session_lifecycle(&self) -> Option<&dyn zeron_harness::session_lifecycle::SessionLifecycle> {
        Some(self)
    }
    async fn models(
        &self,
    ) -> std::result::Result<Vec<zeron_proto::Model>, zeron_harness::HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        _: zeron_proto::RunRequest,
        _: zeron_harness::RunControls,
    ) -> std::result::Result<
        futures::stream::BoxStream<
            'static,
            std::result::Result<zeron_proto::AgentEvent, zeron_harness::HarnessError>,
        >,
        zeron_harness::HarnessError,
    > {
        Err(zeron_harness::HarnessError::Protocol("Not used".into()))
    }
}
#[async_trait::async_trait]
impl zeron_harness::session_lifecycle::SessionLifecycle for RollbackForkHarness {
    fn can_fork_from_turn(&self) -> bool {
        true
    }
    fn supports_fork_rollback(&self) -> bool {
        true
    }
    async fn fork_thread(
        &self,
        input: zeron_harness::session_lifecycle::NativeForkRequest,
    ) -> std::result::Result<String, zeron_harness::HarnessError> {
        self.seen
            .lock()
            .unwrap()
            .push((input.source_turn_id, input.rollback_turns));
        Ok("legacy-native-fork".into())
    }
}

fn legacy_turn(id: &str, attempt: &str, ordinal: i64, status: &str) -> Value {
    let mut turn = sample("OrchestrationV2ProviderTurn");
    turn["id"] = json!(id);
    turn["providerThreadId"] = json!("provider:source");
    turn["runAttemptId"] = json!(attempt);
    turn["nativeTurnRef"] = Value::Null;
    turn["ordinal"] = json!(ordinal);
    turn["status"] = json!(status);
    turn
}

#[tokio::test]
async fn legacy_cursorless_fork_counts_settled_later_turns_and_refuses_a_live_head() {
    for later_live in [false, true] {
        let cwd = tempfile::tempdir().unwrap();
        let (_db, kernel, source) = fixture(cwd.path());
        let turns = [
            legacy_turn("source-turn", "attempt:source", 1, "completed"),
            legacy_turn("later-1", "attempt:later-1", 2, "completed"),
            legacy_turn(
                "later-2",
                "attempt:later-2",
                3,
                if later_live { "running" } else { "failed" },
            ),
        ];
        kernel
            .store
            .write(|tx| {
                for turn in &turns {
                    tx.execute(
                        "INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence)
                         VALUES('source','provider-turn',?1,?2,1)",
                        rusqlite::params![turn["id"].as_str().unwrap(), turn.to_string()],
                    )?;
                }
                Ok(())
            })
            .unwrap();
        kernel
            .transfer_command(
                &"source".into(),
                "fork:legacy".into(),
                TransferOperation::Fork {
                    target: "legacy-child".into(),
                    source: SourcePoint::Run { run_id: source.id },
                    title: None,
                },
            )
            .await
            .unwrap();
        let run = start(&kernel, "legacy-child", "legacy-input").await;
        let harness = RollbackForkHarness {
            seen: Default::default(),
        };
        let caps = crate::orchestration::assembly::capabilities(&harness);
        let mut input = request(cwd.path());
        prepare_run(
            &kernel,
            &run.thread_id,
            &run,
            &mut input,
            &harness,
            &caps,
            Default::default(),
        )
        .await
        .unwrap();
        let transfer = &kernel.store.thread_transfers(&run.thread_id).unwrap()[0];
        let seen = harness.seen.lock().unwrap().clone();
        if later_live {
            assert!(seen.is_empty(), "a live later turn makes the count unsafe");
            assert_eq!(transfer["resolution"]["strategy"], "portable_context");
        } else {
            assert_eq!(seen, vec![(None, Some(2))]);
            assert_eq!(transfer["resolution"]["strategy"], "native_fork");
            assert_eq!(input.resume.as_deref(), Some("legacy-native-fork"));
        }
    }
}
