//! Exact-target steering and restart-durable per-input acceptance.
//! A running root does not prove that its later steering messages were told.
use rusqlite::{Connection, OptionalExtension, params};
use zeron_proto::orchestration::*;

use super::{
    Result,
    effects::{Effect, EffectRequest},
    projection::ThreadProjection,
    task::records,
};

/// The immutable host admission bound to a live runtime. MCP credential
/// renewal/expiry does not change this identity; a replacement attempt does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeTarget {
    pub run_id: RunId,
    pub attempt_id: RunAttemptId,
    pub root_node_id: NodeId,
    pub provider_thread_id: ProviderThreadId,
    pub runtime_id: Option<String>,
}

impl RuntimeTarget {
    pub(crate) fn for_run(run: &OrchestrationV2Run) -> Option<Self> {
        Some(Self {
            run_id: run.id.clone(),
            attempt_id: run.active_attempt_id.clone()?,
            root_node_id: run.root_node_id.clone()?,
            provider_thread_id: run.provider_thread_id.clone()?,
            runtime_id: None,
        })
    }
}

pub(crate) fn bind_runtime(
    conn: &Connection,
    thread: &ThreadId,
    target: &RuntimeTarget,
    runtime_id: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO orchestration_runtime_targets
         (thread_id,run_id,run_attempt_id,root_node_id,provider_thread_id,runtime_id)
         VALUES(?1,?2,?3,?4,?5,?6)
         ON CONFLICT(thread_id) DO UPDATE SET run_id=excluded.run_id,
            run_attempt_id=excluded.run_attempt_id,root_node_id=excluded.root_node_id,
            provider_thread_id=excluded.provider_thread_id,runtime_id=excluded.runtime_id",
        params![
            thread.0,
            target.run_id.0,
            target.attempt_id.0,
            target.root_node_id.0,
            target.provider_thread_id.0,
            runtime_id
        ],
    )?;
    Ok(())
}

pub(crate) fn runtime_id(
    conn: &Connection,
    thread: &ThreadId,
    target: &RuntimeTarget,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT runtime_id FROM orchestration_runtime_targets
         WHERE thread_id=?1 AND run_id=?2 AND run_attempt_id=?3
           AND root_node_id=?4 AND provider_thread_id=?5",
            params![
                thread.0,
                target.run_id.0,
                target.attempt_id.0,
                target.root_node_id.0,
                target.provider_thread_id.0
            ],
            |r| r.get(0),
        )
        .optional()?)
}

pub(crate) fn admitted_runtime(conn: &Connection, effect_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT runtime_id FROM orchestration_steering_inputs WHERE effect_id=?1",
            [effect_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

/// T3 ProviderTurnControlService.load's recorded execution target, with the
/// additional Noches attempt/root/ordinal fences. Never resolve by chat alone.
pub(crate) fn target<'a>(
    p: &'a ThreadProjection,
    request: &EffectRequest,
) -> Option<(&'a OrchestrationV2Run, &'a serde_json::Value)> {
    let EffectRequest::ProviderTurnSteer {
        provider_session_id,
        provider_thread_id,
        provider_turn_id,
        message_id,
    } = request
    else {
        return None;
    };
    if p.thread.archived_at.is_some() || p.thread.deleted_at.is_some() {
        return None;
    }
    let message = records(p, "message")
        .iter()
        .find(|m| m["id"] == message_id.0)?;
    let turn = records(p, "provider-turn")
        .iter()
        .find(|t| t["id"] == provider_turn_id.0 && t["providerThreadId"] == provider_thread_id.0)?;
    let run = p.runs.iter().find(|r| {
        message["runId"] == r.id.0
            && r.provider_thread_id.as_ref() == Some(provider_thread_id)
            && r.active_attempt_id
                .as_ref()
                .is_some_and(|a| turn["runAttemptId"] == a.0)
            && r.root_node_id
                .as_ref()
                .is_some_and(|n| turn["nodeId"] == n.0 && message["nodeId"] == n.0)
    })?;
    p.attempts.iter().find(|a| {
        Some(&a.id) == run.active_attempt_id.as_ref()
            && a.run_id == run.id
            && Some(&a.root_node_id) == run.root_node_id.as_ref()
            && &a.provider_thread_id == provider_thread_id
            && a.provider_turn_id.as_ref() == Some(provider_turn_id)
    })?;
    records(p, "provider-thread").iter().find(|provider| {
        provider["id"] == provider_thread_id.0
            && provider["providerSessionId"] == provider_session_id.0
            && provider["providerInstanceId"] == run.provider_instance_id.0
            && provider["lastRunOrdinal"] == run.ordinal
    })?;
    Some((run, turn))
}

pub(crate) fn confirmed(conn: &Connection, effect_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM orchestration_steering_acceptances WHERE effect_id=?1)",
        [effect_id],
        |r| r.get(0),
    )?)
}

/// Only an exact adapter receipt may call this. Record the proof even when a
/// terminal event raced the acknowledgement, but never for another admission.
pub(crate) fn confirm(
    conn: &Connection,
    p: &ThreadProjection,
    effect: &Effect,
    now: i64,
) -> Result<bool> {
    let Some((run, _)) = target(p, &effect.request) else {
        return Ok(false);
    };
    if !conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM orchestration_steering_inputs WHERE effect_id=?1)",
        [&effect.id],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(false);
    }
    if let Some(expected) = admitted_runtime(conn, &effect.id)?
        && runtime_id(
            conn,
            &effect.thread_id,
            &RuntimeTarget::for_run(run).unwrap(),
        )?
        .as_ref()
            != Some(&expected)
    {
        return Ok(false);
    }
    let EffectRequest::ProviderTurnSteer {
        message_id,
        provider_session_id,
        ..
    } = &effect.request
    else {
        return Ok(false);
    };
    conn.execute(
        "INSERT OR IGNORE INTO orchestration_steering_acceptances
         (effect_id,thread_id,message_id,run_attempt_id,provider_session_id,accepted_at)
         VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            effect.id,
            effect.thread_id.0,
            message_id.0,
            run.active_attempt_id.as_ref().unwrap().0,
            provider_session_id.0,
            now
        ],
    )?;
    // An ACK may arrive after a worker timeout/lease loss. Release only this
    // uncertainty barrier; never steal a running worker's lease or resurrect
    // owner-cancelled work.
    conn.execute(
        "UPDATE orchestration_effect_outbox SET status='succeeded',completed_at=?2,last_error=NULL
         WHERE effect_id=?1 AND status='uncertain'",
        params![effect.id, now],
    )?;
    Ok(true)
}

pub(crate) fn accept_input(
    conn: &Connection,
    p: &ThreadProjection,
    run_id: &RunId,
    attempt_id: &RunAttemptId,
    message_id: &str,
    now: i64,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT effect_id FROM orchestration_effect_outbox
         WHERE thread_id=?1 AND effect_type='provider-turn.steer' AND dispatch_started=1
         AND json_extract(payload_json,'$.messageId')=?2",
    )?;
    let ids = stmt
        .query_map(params![p.thread.id.0, message_id], |r| {
            r.get::<_, String>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for id in ids {
        let effect = super::effects::get(conn, &id)?.expect("selected steering effect");
        if target(p, &effect.request).is_some_and(|(run, _)| {
            &run.id == run_id && run.active_attempt_id.as_ref() == Some(attempt_id)
        }) {
            confirm(conn, p, &effect, now)?;
        }
    }
    Ok(())
}

/// Recovery supplies only untold steering messages, not an accepted root's
/// entire transcript. A legacy succeeded effect has no receipt contract and
/// is deliberately not reinterpreted as rejected input.
pub(crate) fn unconfirmed_messages(
    conn: &Connection,
    p: &ThreadProjection,
    run: &OrchestrationV2Run,
) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(e.payload_json,'$.messageId')
         FROM orchestration_effect_outbox e
         JOIN orchestration_steering_inputs i ON i.effect_id=e.effect_id
         WHERE e.thread_id=?1 AND e.effect_type='provider-turn.steer'
         AND json_extract(e.payload_json,'$.providerThreadId')=?2
         AND e.status IN ('failed','cancelled','uncertain')
         AND NOT EXISTS(SELECT 1 FROM orchestration_steering_acceptances a
                        WHERE a.effect_id=e.effect_id)",
    )?;
    let candidates = stmt
        .query_map(
            params![
                p.thread.id.0,
                run.provider_thread_id.as_ref().map(|id| &id.0)
            ],
            |r| r.get::<_, String>(0),
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(candidates
        .into_iter()
        .filter(|id| {
            let Some(message) = records(p, "message").iter().find(|m| m["id"] == *id) else {
                return false;
            };
            p.runs.iter().any(|source| {
                message["runId"] == source.id.0
                    && source.ordinal < run.ordinal
                    && source.provider_thread_id == run.provider_thread_id
                    && super::command::run_terminal(&source.status)
                    && source.status != OrchestrationV2RunStatus::RolledBack
            })
        })
        .collect())
}

/// A new, explicit turn may proceed after its predecessor stopped. Retire
/// that predecessor's steering uncertainty barrier, not its user input or its
/// missing receipt. Recovery context still sees the cancelled, unconfirmed
/// message; no effect is automatically replayed.
pub(crate) fn retire_for_start(
    conn: &Connection,
    thread: &ThreadId,
    run_id: &RunId,
    now: i64,
) -> Result<()> {
    let Some(p) = super::projection::read_thread(conn, thread)? else {
        return Ok(());
    };
    let Some(run) = p.runs.iter().find(|r| &r.id == run_id) else {
        return Ok(());
    };
    let mut stmt = conn.prepare(
        "SELECT effect_id,json_extract(payload_json,'$.messageId')
         FROM orchestration_effect_outbox
         WHERE thread_id=?1 AND effect_type='provider-turn.steer' AND status='uncertain'",
    )?;
    let candidates = stmt
        .query_map([&thread.0], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (effect_id, message_id) in candidates {
        let previous = records(&p, "message")
            .iter()
            .find(|m| m["id"] == message_id)
            .and_then(|m| p.runs.iter().find(|r| m["runId"] == r.id.0));
        if !previous.is_some_and(|previous| {
            previous.ordinal < run.ordinal && super::command::run_terminal(&previous.status)
        }) {
            continue;
        }
        conn.execute(
            "UPDATE orchestration_effect_outbox
             SET status='cancelled',completed_at=?2,last_error='Unconfirmed steering retained for explicit follow-up.'
             WHERE effect_id=?1 AND status='uncertain'",
            params![effect_id, now],
        )?;
    }
    Ok(())
}

/// Shared ordinary-send and queue-promotion seam. Promotion deliberately has
/// no late follow-up; both paths require an exact live admission and receipt.
pub(crate) async fn execute(
    bridge: &super::runner::RunnerBridge,
    effect: &Effect,
    allow_follow_up: bool,
) -> Result<super::effects::EffectOutcome> {
    use super::effects::EffectOutcome;
    use crate::sessions::CanonicalSteerOutcome;
    let guards = bridge
        .kernel
        .locks
        .acquire([effect.thread_id.clone()])
        .await;
    if bridge
        .kernel
        .store
        .read(|conn| confirmed(conn, &effect.id))?
    {
        return Ok(EffectOutcome::Succeeded);
    }
    let p = bridge
        .kernel
        .store
        .thread(&effect.thread_id)?
        .ok_or_else(|| super::Error::Invariant("Steering thread missing.".into()))?;
    let Some((run, turn)) = target(&p, &effect.request) else {
        return Ok(EffectOutcome::Failed);
    };
    let EffectRequest::ProviderTurnSteer {
        message_id,
        provider_session_id,
        ..
    } = &effect.request
    else {
        unreachable!("validated steering target");
    };
    if turn["status"] == "completed" && allow_follow_up {
        drop(guards);
        super::threads::runner::late_steer(&bridge.kernel, effect, message_id).await?;
        return Ok(EffectOutcome::Succeeded);
    }
    if run.status != OrchestrationV2RunStatus::Running
        || turn["status"] != "running"
        || !records(&p, "provider-session").iter().any(|s| {
            s["id"] == provider_session_id.0
                && s["status"] == "running"
                && s["capabilities"]["turns"]["supportsActiveSteering"] == true
        })
    {
        return Ok(EffectOutcome::Failed);
    }
    let message = records(&p, "message")
        .iter()
        .find(|m| m["id"] == message_id.0)
        .unwrap();
    let mut text = message["text"].as_str().unwrap_or_default().to_owned();
    let paths = bridge
        .kernel
        .store
        .read(|conn| super::ui_queue::attachment_paths(conn, &effect.thread_id, &message_id.0))?;
    if !paths.is_empty() {
        text.push_str("\n\nAttachments:\n");
        text.push_str(
            &paths
                .iter()
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    for attachment in message["attachments"].as_array().into_iter().flatten() {
        if let Some(path) = bridge
            .kernel
            .store
            .launch_attachment_path(attachment["id"].as_str().unwrap_or(""), &effect.thread_id.0)?
        {
            if !paths.contains(&path) {
                text.push_str(&format!(
                    "\nAttached {} (local file): {path}",
                    attachment["type"].as_str().unwrap_or("file")
                ));
            }
        }
    }
    let mut expected = RuntimeTarget::for_run(run).unwrap();
    expected.runtime_id = bridge
        .kernel
        .store
        .read(|conn| admitted_runtime(conn, &effect.id))?;
    if expected.runtime_id.is_none() {
        return Ok(EffectOutcome::Failed);
    }
    let outcome = bridge
        .sessions
        .steer_canonical(&effect.thread_id.0, &expected, &text, message_id.0.clone())
        .await
        .map_err(|e| super::Error::Invariant(e.to_string()))?;
    match outcome {
        CanonicalSteerOutcome::Accepted => {
            let p = bridge.kernel.store.thread(&effect.thread_id)?.unwrap();
            match bridge
                .kernel
                .store
                .write(|conn| confirm(conn, &p, effect, crate::now_ms()))
            {
                Ok(true) => Ok(EffectOutcome::Succeeded),
                Ok(false) => Ok(EffectOutcome::Uncertain),
                Err(error) => {
                    tracing::warn!(effect = %effect.id, %error, "accepted steer receipt persistence uncertain");
                    Ok(EffectOutcome::Uncertain)
                }
            }
        }
        CanonicalSteerOutcome::Rejected => {
            // The adapter can reject while the exact target completes. Let
            // provider observation commit before rechecking in the follow-up
            // planner; never reinterpret failure/Stop/missing runtime as Done.
            drop(guards);
            if allow_follow_up {
                tokio::task::yield_now().await;
                let current = bridge.kernel.store.thread(&effect.thread_id)?.unwrap();
                if target(&current, &effect.request)
                    .is_some_and(|(_, t)| t["status"] == "completed")
                {
                    super::threads::runner::late_steer(&bridge.kernel, effect, message_id).await?;
                    return Ok(EffectOutcome::Succeeded);
                }
            }
            Ok(EffectOutcome::Failed)
        }
        CanonicalSteerOutcome::Uncertain => Ok(EffectOutcome::Uncertain),
    }
}
