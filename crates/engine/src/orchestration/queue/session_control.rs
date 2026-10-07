//! User-requested detach, separate from archive and ordinary turn interruption.
//! Pin the observed sessions and the runtime run; never clear native history.
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use zeron_proto::orchestration::{CommandId, ProviderSessionId};
use zeron_proto::transfer::{
    DisconnectThreadSessionParams, DisconnectThreadSessionResult, ProviderSessionRef,
    ResetThreadSessionParams, ResetThreadSessionResult,
};
use zeron_rpc::RpcError;

use super::QueueDomain;
use crate::orchestration::{
    Error, ReceiptStatus, Result,
    command::{Command, Plan},
    effects::EffectRequest,
    event::{encode_component, iso},
    projection::ThreadProjection,
    task,
};

pub(crate) fn attached_sessions(
    conn: &Connection,
    p: &ThreadProjection,
) -> Result<Vec<ProviderSessionRef>> {
    let mut ids = vec![];
    for session in task::records(p, "provider-session")
        .iter()
        .filter(|session| !matches!(session["status"].as_str(), Some("stopped" | "error")))
    {
        let Some(id) = session["id"].as_str() else {
            continue;
        };
        if !task::records(p, "provider-thread")
            .iter()
            .any(|provider| provider["providerSessionId"] == id)
        {
            continue;
        }
        let sequence: Option<i64> = conn
            .query_row(
                "SELECT last_sequence FROM orchestration_projection_records
             WHERE kind='provider-session' AND id=?1 AND thread_id=?2",
                rusqlite::params![id, p.thread.id.0],
                |row| row.get(0),
            )
            .optional()?;
        // A separate passive read must never pair an older projection with a
        // newer attachment. Hide it until the next coherent refresh.
        if let Some(attachment_sequence) = sequence.filter(|seq| *seq <= p.through_sequence) {
            ids.push(ProviderSessionRef {
                id: id.into(),
                attachment_sequence,
            });
        }
    }
    ids.sort_by(|a, b| a.id.cmp(&b.id));
    ids.dedup_by(|a, b| a.id == b.id);
    Ok(ids)
}

pub(super) fn plan(
    conn: &Connection,
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    input: &Value,
    now: i64,
) -> Result<()> {
    let ids: Vec<ProviderSessionRef> = serde_json::from_value(input["providerSessions"].clone())?;
    let current = attached_sessions(conn, p)?;
    // Validate the entire selection before emitting anything. Requiring the
    // observed set prevents a stale panel from detaching a new attachment.
    if ids.is_empty()
        || ids != current
        || p.thread.archived_at.is_some()
        || p.runs.iter().any(|run| {
            !crate::orchestration::command::run_terminal(&run.status)
                && run.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Queued
                && !ids.iter().any(|session| {
                    session.id == format!("provider-session:{}", encode_component(&run.id.0))
                })
        })
    {
        return Err(Error::Invariant(
            "The agent session changed. Refresh Details before disconnecting.".into(),
        ));
    }
    for reference in ids {
        let id = reference.id;
        let run = p
            .runs
            .iter()
            .find(|run| format!("provider-session:{}", encode_component(&run.id.0)) == id)
            .ok_or_else(|| Error::Invariant("The attached session has no owning run.".into()))?;
        let attempt = p
            .attempts
            .iter()
            .find(|attempt| Some(&attempt.id) == run.active_attempt_id.as_ref())
            .ok_or_else(|| {
                Error::Invariant("The attached session has no owning attempt.".into())
            })?;
        task::records(p, "provider-thread")
            .iter()
            .find(|provider| {
                Some(provider["id"].as_str().unwrap_or(""))
                    == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
            })
            .ok_or_else(|| {
                Error::Invariant("The attached provider thread was not found.".into())
            })?;
        plan.emit(
            command,
            "provider-session.detached",
            &json!({"providerSessionId":id,"detachedAt":iso(now)?,"reason":"client-requested"}),
            now,
        )?;
        plan.effects.push(EffectRequest::ProviderSessionDisconnect {
            provider_session_id: ProviderSessionId(id),
            run_id: run.id.clone(),
            run_attempt_id: attempt.id.clone(),
            provider_thread_id: run.provider_thread_id.clone().ok_or_else(|| {
                Error::Invariant("The attached run has no provider thread.".into())
            })?,
        });
    }
    Ok(())
}

/// Forced reconstruction. History, runs and the app conversation are kept; the
/// provider conversations they ran on are closed, so the next turn starts a new
/// provider-thread generation seeded with bounded portable context. Queued runs
/// already bound to a closed conversation move to one fresh generation together.
pub(super) fn plan_reset(
    conn: &Connection,
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    input: &Value,
    now: i64,
) -> Result<()> {
    use zeron_proto::orchestration::OrchestrationV2RunStatus::Queued;
    let latest = p
        .runs
        .iter()
        .filter(|run| run.status != Queued)
        .max_by_key(|run| run.ordinal)
        .map(|run| run.id.0.as_str());
    if p.thread.archived_at.is_some() {
        return Err(Error::Invariant("Archived threads cannot be reset.".into()));
    }
    if p.runs.iter().any(|run| {
        !crate::orchestration::command::run_terminal(&run.status) && run.status != Queued
    }) {
        return Err(Error::Invariant(
            "Stop the current run before resetting the agent session.".into(),
        ));
    }
    if input["observedRunId"].as_str() != latest {
        return Err(Error::Invariant(
            "A newer turn started. Refresh Details before resetting.".into(),
        ));
    }
    let observed: Vec<ProviderSessionRef> =
        serde_json::from_value(input["providerSessions"].clone())?;
    if observed.is_empty() {
        if !attached_sessions(conn, p)?.is_empty() {
            return Err(Error::Invariant(
                "The agent session changed. Refresh Details before resetting.".into(),
            ));
        }
    } else {
        // Live attachments are torn down exactly as Disconnect does.
        self::plan(conn, plan, command, p, input, now)?;
    }
    let started: Vec<&str> = p
        .runs
        .iter()
        .filter(|run| run.status != Queued)
        .filter_map(|run| run.provider_thread_id.as_ref().map(|id| id.0.as_str()))
        .collect();
    let closing: Vec<&Value> = task::records(p, "provider-thread")
        .iter()
        .filter(|provider| {
            provider["id"]
                .as_str()
                .is_some_and(|id| started.contains(&id))
                && !matches!(
                    provider["status"].as_str(),
                    Some("closed" | "archived" | "error")
                )
        })
        .collect();
    if closing.is_empty() {
        return Err(Error::Invariant(
            "There is no agent session to reset yet.".into(),
        ));
    }
    let time = iso(now)?;
    for provider in &closing {
        let mut next = (*provider).clone();
        next["status"] = json!("closed");
        next["updatedAt"] = json!(time);
        plan.emit(command, "provider-thread.updated", &next, now)?;
    }
    // One fresh generation per closed conversation, named after the first queued
    // run that will use it (the id convention `execution_seed` uses).
    for provider in &closing {
        let mut queued = p.runs.iter().filter(|run| {
            run.status == Queued
                && run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                    == provider["id"].as_str()
        });
        let Some(first) = queued.next() else { continue };
        let fresh = format!(
            "provider-thread:app:{}:{}:{}",
            encode_component(&p.thread.id.0),
            encode_component(&first.provider_instance_id.0),
            first.ordinal
        );
        let mut thread = (*provider).clone();
        thread["id"] = json!(fresh);
        thread["status"] = json!("not_loaded");
        for key in [
            "providerSessionId",
            "nativeThreadRef",
            "nativeConversationHeadRef",
            "forkedFrom",
        ] {
            thread[key] = Value::Null;
        }
        thread["ownerNodeId"] = json!(first.root_node_id);
        thread["firstRunOrdinal"] = json!(first.ordinal);
        thread["lastRunOrdinal"] = json!(first.ordinal);
        thread["handoffIds"] = json!([]);
        thread["pendingBackgroundTasks"] = json!([]);
        thread["createdAt"] = json!(time);
        thread["updatedAt"] = json!(time);
        plan.emit(command, "provider-thread.updated", &thread, now)?;
        for run in std::iter::once(first).chain(queued) {
            let mut value = serde_json::to_value(run)?;
            value["providerThreadId"] = json!(fresh);
            plan.emit(command, "run.updated", &value, now)?;
            for attempt in p.attempts.iter().filter(|a| a.run_id == run.id) {
                let mut value = serde_json::to_value(attempt)?;
                value["providerThreadId"] = json!(fresh);
                plan.emit(command, "run-attempt.updated", &value, now)?;
            }
            for node in p
                .nodes
                .iter()
                .filter(|n| n.run_id.as_ref() == Some(&run.id))
            {
                let mut value = serde_json::to_value(node)?;
                value["providerThreadId"] = json!(fresh);
                plan.emit(command, "node.updated", &value, now)?;
            }
        }
    }
    Ok(())
}

/// A run cancelled while still queued: it never reached the provider, so it
/// proves nothing about the conversation it was bound to.
pub(crate) fn never_started(run: &zeron_proto::orchestration::OrchestrationV2Run) -> bool {
    run.status == zeron_proto::orchestration::OrchestrationV2RunStatus::Cancelled
        && run.started_at.is_none()
}

/// True for a run that begins a new provider-thread generation after a user
/// reset: it is bound to a closed conversation, or is bound to a fresh one that
/// has not yet established its own native conversation while a closed sibling
/// exists. Later runs resume that generation once it has.
pub(crate) fn fresh_after_reset(
    p: &ThreadProjection,
    run: &zeron_proto::orchestration::OrchestrationV2Run,
) -> bool {
    let closed = |thread: &Value| thread["status"] == "closed";
    let Some(own) = task::records(p, "provider-thread").iter().find(|thread| {
        Some(thread["id"].as_str().unwrap_or(""))
            == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
    }) else {
        return false;
    };
    closed(own) || fresh_generation(p, run)
}

/// The fresh generation a reset created, until a turn on it is accepted. Keyed
/// on the generation's own evidence, never on whether an earlier run shares it:
/// a cancelled or startup-failed first run must not turn the next start back
/// into a resume of the closed conversation.
pub(crate) fn fresh_generation(
    p: &ThreadProjection,
    run: &zeron_proto::orchestration::OrchestrationV2Run,
) -> bool {
    use zeron_proto::orchestration::OrchestrationV2RunStatus::{Completed, Waiting};
    let threads = task::records(p, "provider-thread");
    let Some(own) = threads.iter().find(|thread| {
        Some(thread["id"].as_str().unwrap_or(""))
            == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
    }) else {
        return false;
    };
    if own["status"] == "closed"
        || !threads.iter().any(|sibling| {
            sibling["status"] == "closed"
                && sibling["id"] != own["id"]
                && sibling["providerInstanceId"] == own["providerInstanceId"]
        })
    {
        return false;
    }
    let native = own["nativeThreadRef"]["nativeId"]
        .as_str()
        .is_some_and(|id| !id.is_empty())
        || own["providerSessionId"].as_str().is_some_and(|id| !id.is_empty());
    let accepted = p.runs.iter().any(|earlier| {
        earlier.id != run.id
            && earlier.ordinal < run.ordinal
            && earlier.provider_thread_id == run.provider_thread_id
            && !never_started(earlier)
            && (matches!(earlier.status, Completed | Waiting)
                || p.attempts.iter().any(|attempt| {
                    attempt.run_id == earlier.id && attempt.native_thread_id.as_ref().is_some()
                }))
    });
    !native && !accepted
}

pub(super) fn target_still_disconnected(p: &ThreadProjection, request: &EffectRequest) -> bool {
    let EffectRequest::ProviderSessionDisconnect {
        provider_session_id,
        run_id,
        run_attempt_id,
        provider_thread_id,
    } = request
    else {
        return false;
    };
    p.thread.deleted_at.is_none()
        && format!("provider-session:{}", encode_component(&run_id.0)) == provider_session_id.0
        && p.runs.iter().any(|run| &run.id == run_id && run.active_attempt_id.as_ref() == Some(run_attempt_id))
        && !p.runs.iter().any(|run| &run.id != run_id
            && !crate::orchestration::command::run_terminal(&run.status)
            && run.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Queued)
        // The native conversation generation is part of this stable ID, not
        // a fabricated numeric field on T3's ProviderThread contract.
        && task::records(p, "provider-thread").iter().any(|provider|
            provider["id"] == provider_thread_id.0)
        // Acceptance removed this attachment. Its reappearance is a newly
        // started runtime, even on the same logical attempt.
        && !task::records(p, "provider-session").iter()
            .any(|session| session["id"] == provider_session_id.0)
}

impl QueueDomain {
    pub(crate) async fn disconnect_for_user(
        &self,
        docs: &crate::DocHost,
        mut request: DisconnectThreadSessionParams,
    ) -> std::result::Result<DisconnectThreadSessionResult, RpcError> {
        let fail = |error: Error| RpcError::Failed(error.to_string());
        if request.provider_sessions.is_empty() || request.provider_sessions.len() > 64 {
            return Err(RpcError::BadParams(
                "Select between 1 and 64 attached sessions.".into(),
            ));
        }
        for id in [&request.chat_id, &request.client_request_id]
            .into_iter()
            .chain(request.provider_sessions.iter().map(|session| &session.id))
        {
            if id.trim().is_empty() || id.len() > 512 {
                return Err(RpcError::BadParams(
                    "Session identities must be nonempty and at most 512 bytes.".into(),
                ));
            }
        }
        if !docs.is_host(&request.chat_id) {
            return Err(RpcError::Failed(
                "Session disconnect requires the owning host.".into(),
            ));
        }
        request.provider_sessions.sort_by(|a, b| a.id.cmp(&b.id));
        if request
            .provider_sessions
            .windows(2)
            .any(|pair| pair[0].id == pair[1].id)
            || request
                .provider_sessions
                .iter()
                .any(|session| session.attachment_sequence <= 0)
        {
            return Err(RpcError::BadParams(
                "Session revisions must be positive and identities unique.".into(),
            ));
        }
        let thread = request.chat_id.clone().into();
        let handle = docs
            .open(&request.chat_id)
            .map_err(|e| RpcError::Failed(e.to_string()))?;
        let _queue_guard = handle.orchestration_queue_lock().await;
        self.kernel
            .store
            .thread(&thread)
            .map_err(fail)?
            .filter(|p| p.thread.deleted_at.is_none())
            .ok_or_else(|| RpcError::Failed("The thread was not found.".into()))?;
        let id = CommandId(format!(
            "ui:disconnect:{}:{}",
            encode_component(&request.chat_id),
            encode_component(&request.client_request_id)
        ));
        let input = json!({"providerSessions":request.provider_sessions});
        let payload = input.to_string();
        let same = self
            .kernel
            .store
            .write(|conn| {
                super::reserve_request(conn, super::SESSION_USER_REQUESTS, &id.0, &payload)
            })
            .map_err(fail)?;
        if !same {
            return Err(RpcError::BadParams(
                "This disconnect request already belongs to different sessions.".into(),
            ));
        }
        let receipt = self
            .mutate(
                None,
                thread,
                "host.disconnect_provider_sessions",
                input,
                id,
                crate::now_ms(),
            )
            .await
            .map_err(fail)?;
        Ok(DisconnectThreadSessionResult {
            sequence: receipt.result_sequence,
            refusal: (receipt.status == ReceiptStatus::Rejected).then(|| {
                receipt
                    .error
                    .unwrap_or_else(|| "Session disconnect was refused.".into())
            }),
        })
    }
}

impl QueueDomain {
    pub(crate) async fn reset_for_user(
        &self,
        docs: &crate::DocHost,
        mut request: ResetThreadSessionParams,
    ) -> std::result::Result<ResetThreadSessionResult, RpcError> {
        let fail = |error: Error| RpcError::Failed(error.to_string());
        if request.provider_sessions.len() > 64 {
            return Err(RpcError::BadParams(
                "Select at most 64 attached sessions.".into(),
            ));
        }
        for id in [&request.chat_id, &request.client_request_id]
            .into_iter()
            .chain(request.provider_sessions.iter().map(|session| &session.id))
            .chain(request.observed_run_id.as_ref())
        {
            if id.trim().is_empty() || id.len() > 512 {
                return Err(RpcError::BadParams(
                    "Session identities must be nonempty and at most 512 bytes.".into(),
                ));
            }
        }
        if !docs.is_host(&request.chat_id) {
            return Err(RpcError::Failed(
                "Session reset requires the owning host.".into(),
            ));
        }
        request.provider_sessions.sort_by(|a, b| a.id.cmp(&b.id));
        if request
            .provider_sessions
            .windows(2)
            .any(|pair| pair[0].id == pair[1].id)
            || request
                .provider_sessions
                .iter()
                .any(|session| session.attachment_sequence <= 0)
        {
            return Err(RpcError::BadParams(
                "Session revisions must be positive and identities unique.".into(),
            ));
        }
        let thread = request.chat_id.clone().into();
        let handle = docs
            .open(&request.chat_id)
            .map_err(|e| RpcError::Failed(e.to_string()))?;
        let _queue_guard = handle.orchestration_queue_lock().await;
        self.kernel
            .store
            .thread(&thread)
            .map_err(fail)?
            .filter(|p| p.thread.deleted_at.is_none())
            .ok_or_else(|| RpcError::Failed("The thread was not found.".into()))?;
        let id = CommandId(format!(
            "ui:reset-session:{}:{}",
            encode_component(&request.chat_id),
            encode_component(&request.client_request_id)
        ));
        let input = json!({
            "observedRunId":request.observed_run_id,
            "providerSessions":request.provider_sessions
        });
        let payload = input.to_string();
        let same = self
            .kernel
            .store
            .write(|conn| {
                super::reserve_request(conn, super::SESSION_USER_REQUESTS, &id.0, &payload)
            })
            .map_err(fail)?;
        if !same {
            return Err(RpcError::BadParams(
                "This reset request already belongs to a different session state.".into(),
            ));
        }
        let receipt = self
            .mutate(
                None,
                thread,
                "host.reset_provider_session",
                input,
                id,
                crate::now_ms(),
            )
            .await
            .map_err(fail)?;
        Ok(ResetThreadSessionResult {
            sequence: receipt.result_sequence,
            refusal: (receipt.status == ReceiptStatus::Rejected).then(|| {
                receipt
                    .error
                    .unwrap_or_else(|| "Session reset was refused.".into())
            }),
        })
    }
}
