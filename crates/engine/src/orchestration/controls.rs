//! Durable exact-process admission for Stop and interrupt/restart.
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use zeron_proto::orchestration::*;

use super::{
    Result,
    effects::{Effect, EffectOutcome, EffectRequest},
    projection::ThreadProjection,
    steering::RuntimeTarget,
    task::records,
};

pub(crate) async fn interrupt_for_user(
    bridge: &super::runner::RunnerBridge,
    thread: &ThreadId,
    command_id: &str,
) -> Result<Option<bool>> {
    let _guards = bridge.kernel.locks.acquire([thread.clone()]).await;
    let id = CommandId(format!(
        "command:user-stop:{}:{}",
        super::event::encode_component(&thread.0),
        super::event::encode_component(command_id)
    ));
    if let Some(receipt) = bridge.kernel.store.receipt(&id)? {
        return if receipt.status == super::ReceiptStatus::Accepted {
            Ok(Some(true))
        } else {
            Err(super::Error::Invariant(receipt.error.unwrap_or_default()))
        };
    }
    let Some(p) = bridge.kernel.store.thread(thread)? else {
        return Ok(None);
    };
    if p.thread.deleted_at.is_some() || p.thread.archived_at.is_some() {
        return Err(super::Error::Invariant(
            "Thread is not interruptible.".into(),
        ));
    }
    let Some(run) = super::background::interruptible_run(&p) else {
        return Ok(Some(false));
    };
    let receipt = bridge.kernel.store.dispatch(
        &super::Command {
            id,
            thread_id: thread.clone(),
            operation: super::Operation::Thread(Box::new(
                super::threads::planner::ThreadOperation::Interrupt {
                    run_id: run.id.clone(),
                    reason: None,
                },
            )),
        },
        crate::now_ms(),
    )?;
    if receipt.status == super::ReceiptStatus::Rejected {
        return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
    }
    Ok(Some(true))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ControlTarget {
    runtime: RuntimeTarget,
    /// Restart has already committed a new attempt before the effect runs.
    replacement_attempt_id: Option<RunAttemptId>,
}

fn target(p: &ThreadProjection, request: &EffectRequest) -> Option<ControlTarget> {
    let (run, attempt, replacement_attempt_id) = match request {
        EffectRequest::ManagedRunInterrupt { run_id } => {
            let run = p.runs.iter().find(|r| &r.id == run_id)?;
            let attempt = p
                .attempts
                .iter()
                .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())?;
            (run, attempt, None)
        }
        EffectRequest::ProviderTurnInterrupt {
            provider_session_id,
            provider_thread_id,
            provider_turn_id,
        }
        | EffectRequest::ProviderTurnRestart {
            provider_session_id,
            provider_thread_id,
            provider_turn_id,
            ..
        } => {
            let provider = records(p, "provider-thread").iter().find(|r| {
                r["id"] == provider_thread_id.0 && r["providerSessionId"] == provider_session_id.0
            })?;
            let turn = records(p, "provider-turn").iter().find(|t| {
                t["id"] == provider_turn_id.0 && t["providerThreadId"] == provider_thread_id.0
            })?;
            let attempt = p.attempts.iter().find(|a| {
                turn["runAttemptId"] == a.id.0
                    && turn["nodeId"] == a.root_node_id.0
                    && &a.provider_thread_id == provider_thread_id
                    && a.provider_turn_id.as_ref() == Some(provider_turn_id)
            })?;
            let run = p.runs.iter().find(|r| {
                r.id == attempt.run_id
                    && r.provider_thread_id.as_ref() == Some(provider_thread_id)
                    && provider["providerInstanceId"] == r.provider_instance_id.0
                    && provider["lastRunOrdinal"] == r.ordinal
            })?;
            let replacement = if let EffectRequest::ProviderTurnRestart {
                interrupted_attempt_id,
                run_id,
                ..
            } = request
            {
                if &attempt.id != interrupted_attempt_id || &run.id != run_id {
                    return None;
                }
                let next = p.attempts.iter().find(|a| {
                    Some(&a.id) == run.active_attempt_id.as_ref()
                        && a.run_id == run.id
                        && Some(&a.root_node_id) == run.root_node_id.as_ref()
                        && a.reason == OrchestrationV2RunAttemptReason::SteeringRestart
                        && a.attempt_ordinal == attempt.attempt_ordinal + 1
                })?;
                Some(next.id.clone())
            } else {
                if run.active_attempt_id.as_ref() != Some(&attempt.id)
                    || run.root_node_id.as_ref() != Some(&attempt.root_node_id)
                {
                    return None;
                }
                None
            };
            (run, attempt, replacement)
        }
        _ => return None,
    };
    Some(ControlTarget {
        runtime: RuntimeTarget {
            run_id: run.id.clone(),
            attempt_id: attempt.id.clone(),
            root_node_id: attempt.root_node_id.clone(),
            provider_thread_id: attempt.provider_thread_id.clone(),
            runtime_id: None,
        },
        replacement_attempt_id,
    })
}

pub(crate) fn admit(
    conn: &Connection,
    id: &str,
    thread: &ThreadId,
    request: &EffectRequest,
) -> Result<()> {
    if !matches!(
        request,
        EffectRequest::ManagedRunInterrupt { .. }
            | EffectRequest::ProviderTurnInterrupt { .. }
            | EffectRequest::ProviderTurnRestart { .. }
    ) {
        return Ok(());
    }
    let mut target = super::projection::read_thread(conn, thread)?
        .as_ref()
        .and_then(|p| target(p, request));
    if let Some(target) = &mut target {
        target.runtime.runtime_id = super::steering::runtime_id(conn, thread, &target.runtime)?;
    }
    conn.execute(
        "INSERT INTO orchestration_control_targets(effect_id,target_json) VALUES(?1,?2)",
        params![id, target.as_ref().map(serde_json::to_string).transpose()?],
    )?;
    Ok(())
}

fn saved_target(conn: &Connection, id: &str) -> Result<Option<ControlTarget>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT target_json FROM orchestration_control_targets WHERE effect_id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    json.map(|json| super::projection::decode(&json))
        .transpose()
}

/// True when admission recorded a row but could not freeze a target.
fn target_unresolved(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT target_json IS NULL FROM orchestration_control_targets WHERE effect_id=?1",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .optional()?
        .unwrap_or(false))
}

/// The final logical repair is checked in its write transaction, not between
/// a preflight read and an independently admitted synthetic terminal event.
pub(crate) fn plan_settlement(
    conn: &Connection,
    p: &ThreadProjection,
    command: &super::Command,
    plan: &mut super::command::Plan,
    effect_id: &str,
    now: i64,
) -> Result<()> {
    let Some(effect) = super::effects::get(conn, effect_id)? else {
        return Ok(());
    };
    if effect.thread_id != p.thread.id
        || !matches!(
            effect.status,
            super::effects::EffectStatus::Pending | super::effects::EffectStatus::Running
        )
        || matches!(effect.request, EffectRequest::ProviderTurnRestart { .. })
    {
        return Ok(());
    }
    let Some(saved) = saved_target(conn, effect_id)? else {
        return Ok(());
    };
    let mut logical = saved.clone();
    logical.runtime.runtime_id = None;
    if target(p, &effect.request).as_ref() != Some(&logical)
        || super::steering::runtime_id(conn, &p.thread.id, &saved.runtime)?
            != saved.runtime.runtime_id
    {
        return Ok(());
    }
    super::runner::plan_event(
        conn,
        p,
        command,
        plan,
        &saved.runtime.run_id,
        &saved.runtime.attempt_id,
        &zeron_proto::AgentEvent::Done {
            status: zeron_proto::DoneStatus::Interrupted,
            result: None,
            error: None,
            session_id: None,
        },
        None,
        now,
    )?;
    let run = p
        .runs
        .iter()
        .find(|r| r.id == saved.runtime.run_id)
        .unwrap();
    super::background::settle(p, command, plan, run, effect_id, now)
}

pub(crate) async fn execute(
    bridge: &super::runner::RunnerBridge,
    effect: &Effect,
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<EffectOutcome> {
    let guards = bridge
        .kernel
        .locks
        .acquire([effect.thread_id.clone()])
        .await;
    let (saved, unresolved) = bridge.kernel.store.read(|conn| {
        if super::effects::get(conn, &effect.id)?.is_none_or(|e| {
            !matches!(
                e.status,
                super::effects::EffectStatus::Pending | super::effects::EffectStatus::Running
            )
        }) {
            return Ok((None, false));
        }
        Ok((
            saved_target(conn, &effect.id)?,
            target_unresolved(conn, &effect.id)?,
        ))
    })?;
    // Legacy rows (no targets row) cannot acquire authority over a new process.
    // An admitted row with no frozen target means the planner and `target()`
    // disagreed: nothing will be stopped, so never report that as success.
    let Some(saved) = saved else {
        if unresolved {
            tracing::warn!(effect = %effect.id, request = ?effect.request, "control effect was admitted without a resolvable target");
            return Ok(EffectOutcome::Failed);
        }
        return Ok(EffectOutcome::Succeeded);
    };
    let Some(p) = bridge.kernel.store.thread(&effect.thread_id)? else {
        return Ok(EffectOutcome::Succeeded);
    };
    let Some(current) = target(&p, &effect.request) else {
        return Ok(EffectOutcome::Succeeded);
    };
    let mut logical_saved = saved.clone();
    logical_saved.runtime.runtime_id = None;
    if current != logical_saved {
        return Ok(EffectOutcome::Succeeded);
    }
    let run = p
        .runs
        .iter()
        .find(|r| r.id == saved.runtime.run_id)
        .unwrap()
        .clone();
    let requested = bridge
        .sessions
        .request_canonical_interrupt(&effect.thread_id.0, &saved.runtime);
    if requested == crate::sessions::CanonicalInterruptOutcome::Replaced {
        return Ok(EffectOutcome::Succeeded);
    }
    // Never wait while holding the ingestion lane: the observer needs it to
    // record the native terminal event and settle mail/background work.
    drop(guards);
    let had_runtime = matches!(
        requested,
        crate::sessions::CanonicalInterruptOutcome::Requested(_)
    );
    if let crate::sessions::CanonicalInterruptOutcome::Requested(runtime_id) = requested {
        if !bridge
            .sessions
            .await_runtime_retirement(&effect.thread_id.0, &runtime_id)
            .await
        {
            return Ok(EffectOutcome::Retry);
        }
    }
    if cancellation.is_cancelled() {
        return Ok(EffectOutcome::Succeeded);
    }
    if had_runtime && !matches!(effect.request, EffectRequest::ProviderTurnRestart { .. }) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            if bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .is_none_or(|p| {
                    p.runs.iter().find(|r| r.id == run.id).is_none_or(|r| {
                        super::command::run_terminal(&r.status)
                            || r.active_attempt_id != run.active_attempt_id
                    })
                })
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    // Process replacement can race the bounded wait. Neither synthetic
    // terminal ingestion nor a restart may repair against that new admission.
    let guards = bridge
        .kernel
        .locks
        .acquire([effect.thread_id.clone()])
        .await;
    let same_runtime = bridge.kernel.store.read(|conn| {
        Ok(
            super::steering::runtime_id(conn, &effect.thread_id, &saved.runtime)?
                == saved.runtime.runtime_id,
        )
    })?;
    let still_targeted = bridge
        .kernel
        .store
        .thread(&effect.thread_id)?
        .as_ref()
        .and_then(|p| target(p, &effect.request))
        .is_some_and(|current| current == logical_saved);
    if !same_runtime || !still_targeted {
        return Ok(EffectOutcome::Succeeded);
    }
    if matches!(effect.request, EffectRequest::ProviderTurnRestart { .. }) {
        drop(guards);
        // start() refuses a busy replacement; its run/attempt admission also
        // prevents a late control from reopening an already-started attempt.
        return bridge.start(effect, &run.id, cancellation).await;
    }
    // Prefer the observer's native terminal result; synthesize only if the
    // exact attempt still needs settlement after its process has retired.
    // Hold the runtime map empty across the synchronous repair transaction.
    // An unbound ordinary-session replacement is fenced too, even if it has
    // not updated the durable canonical runtime binding yet.
    bridge
        .sessions
        .with_retired_runtime(&effect.thread_id.0, || {
            let receipt = bridge.kernel.store.dispatch(
                &super::Command {
                    id: CommandId(format!(
                        "command:control-settlement:{}",
                        super::event::encode_component(&effect.id)
                    )),
                    thread_id: effect.thread_id.clone(),
                    operation: super::Operation::Task(Box::new(
                        super::task::TaskOperation::ControlSettlement {
                            effect_id: effect.id.clone(),
                        },
                    )),
                },
                crate::now_ms(),
            )?;
            if receipt.status == super::ReceiptStatus::Rejected {
                return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
            }
            Ok(())
        })
        .transpose()?;
    drop(guards);
    bridge.settle(&effect.thread_id, &run).await?;
    Ok(EffectOutcome::Succeeded)
}
