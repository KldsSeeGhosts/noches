//! User-requested detach, separate from archive and ordinary turn interruption.
//! Pin the observed sessions and the runtime run; never clear native history.
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use zeron_proto::orchestration::{CommandId, ProviderSessionId};
use zeron_proto::transfer::{
    DisconnectThreadSessionParams, DisconnectThreadSessionResult, ProviderSessionRef,
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
                let previous: Option<String> = conn.query_row(
                "SELECT payload_json FROM orchestration_session_user_requests WHERE command_id=?1",
                [&id.0], |row| row.get(0),
            ).optional()?;
                if let Some(previous) = previous {
                    return Ok(previous == payload);
                }
                conn.execute(
                    "INSERT INTO orchestration_session_user_requests VALUES(?1,?2)",
                    rusqlite::params![id.0, payload],
                )?;
                Ok(true)
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
