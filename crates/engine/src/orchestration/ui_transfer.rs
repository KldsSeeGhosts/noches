//! Passive lineage, provenance and files-only timeline. Never acknowledges a task.
use super::{Error, Result, Store};
use serde_json::{Value, json};
use zeron_proto::orchestration::ThreadId;
use zeron_proto::transfer::ThreadTransferState;
use zeron_rpc::{RpcError, RpcReply, methods};

/// The Details preview shows only the most recent text messages, so the wire
/// state carries just those (delivery reads `inherited_items` in full).
const PREVIEW_MESSAGES: usize = 100;
const PREVIEW_CHARS: usize = 10_000;

fn preview_items(items: Vec<Value>) -> Vec<Value> {
    let mut messages: Vec<Value> = items
        .into_iter()
        .filter(|item| {
            matches!(
                item["type"].as_str(),
                Some("user_message" | "assistant_message")
            ) && item["text"].as_str().is_some_and(|text| !text.is_empty())
        })
        .collect();
    messages.drain(..messages.len().saturating_sub(PREVIEW_MESSAGES));
    for item in &mut messages {
        // One char past the limit, so the client can still tell it was cut.
        if let Some(text) = item["text"].as_str()
            && text.chars().count() > PREVIEW_CHARS + 1
        {
            item["text"] = json!(text.chars().take(PREVIEW_CHARS + 1).collect::<String>());
        }
    }
    messages
}

impl Store {
    pub fn transfer_ui_state(&self, id: &ThreadId) -> Result<ThreadTransferState> {
        let projection = self
            .thread(id)?
            .ok_or_else(|| Error::Invariant("The thread was not found.".into()))?;
        let inherited = preview_items(super::transfer::inherited_items(self, &projection)?);
        let transfers = self.thread_transfers(id)?;
        // A transfer is visible on both ends, but its acceptance receipt lives
        // on the target. Source Details must not label delivered child history
        // "Prepared" just because the receipt is in another conversation.
        let targets: std::collections::BTreeSet<_> = transfers
            .iter()
            .filter_map(|transfer| transfer["targetThreadId"].as_str())
            .collect();
        let (target_handoffs, version) = self.read(|conn| {
            let mut handoffs = super::task::records(&projection, "context-handoff").to_vec();
            let mut version = projection.through_sequence;
            for target in targets.iter().filter(|target| **target != id.0) {
                let thread: Option<zeron_proto::orchestration::OrchestrationV2AppThread> =
                    super::projection::read_entity(
                        conn,
                        super::projection::TABLES[0],
                        target,
                        target,
                    )?;
                if !thread.is_some_and(|thread| thread.project_id == projection.thread.project_id) {
                    continue;
                }
                for handoff in super::projection::read_records(conn, target, "context-handoff")? {
                    if transfers
                        .iter()
                        .any(|transfer| transfer["id"] == handoff["transferId"])
                    {
                        let sequence: i64 = conn.query_row(
                            "SELECT last_sequence FROM orchestration_projection_records
                             WHERE thread_id=?1 AND kind='context-handoff' AND id=?2",
                            rusqlite::params![target, handoff["id"].as_str()],
                            |row| row.get(0),
                        )?;
                        version = version.max(sequence);
                        handoffs.push(handoff);
                    }
                }
            }
            Ok((handoffs, version))
        })?;
        let handoffs = target_handoffs
            .iter()
            .map(|h| {
                json!({
                    "id":h["id"],"transferId":h["transferId"],"threadId":h["threadId"],
                    "targetRunId":h["targetRunId"],"strategy":h["strategy"],"status":h["status"],
                    "coveredRunOrdinals":h["coveredRunOrdinals"],"summaryText":"",
                    "coverage":h["history"]["coverage"],"omittedItems":h["history"]["omittedItems"],
                    "omittedItemIds":h["history"]["omittedItemIds"],
                    "deliveryStatus":h["delivery"]["status"]
                })
            })
            .collect();
        Ok(ThreadTransferState {
            thread_id: id.clone(),
            version,
            lineage: json!(projection.thread.lineage),
            forked_from: projection.thread.forked_from.as_ref().map(|f| json!(f)),
            transfers,
            handoffs,
            checkpoints: self.checkpoint_timeline(id)?,
            inherited_items: inherited,
            latest_forkable_run_id: projection
                .runs
                .iter()
                .rev()
                .find(|run| super::transfer::forkable(&run.status))
                .map(|r| r.id.0.clone()),
            latest_mergeable_run_id: projection
                .runs
                .iter()
                .rev()
                .find(|r| {
                    matches!(
                        r.status,
                        zeron_proto::orchestration::OrchestrationV2RunStatus::Completed
                            | zeron_proto::orchestration::OrchestrationV2RunStatus::Waiting
                    )
                })
                .map(|r| r.id.0.clone()),
            latest_started_run_id: projection
                .runs
                .iter()
                .filter(|r| r.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Queued)
                .max_by_key(|r| r.ordinal)
                .map(|r| r.id.0.clone()),
            attached_provider_sessions: self
                .read(|conn| super::queue::session_control::attached_sessions(conn, &projection))?,
        })
    }
}

pub(crate) async fn rpc(
    store: Option<&Store>,
    method: &str,
    params: Value,
    sessions: &crate::SessionsEngine,
    workspace: &crate::WorkspaceHost,
    registry: &crate::HarnessRegistry,
) -> std::result::Result<RpcReply, RpcError> {
    let store = store.ok_or_else(|| RpcError::Failed("Transfer service is unavailable.".into()))?;
    let service = super::checkpoint::FileCheckpointService {
        kernel: super::Kernel::from_store(store.clone()),
    };
    let error = |e: Error| RpcError::Failed(e.to_string());
    match method {
        methods::FORK_THREAD | methods::MERGE_THREAD_BACK => {
            let _admission = sessions
                .admit_work()
                .map_err(|e| RpcError::Failed(e.to_string()))?;
            let (chat, key, target, source, title) = if method == methods::FORK_THREAD {
                let p: zeron_proto::transfer::ForkThreadParams = serde_json::from_value(params)
                    .map_err(|e| RpcError::BadParams(e.to_string()))?;
                (
                    p.chat_id,
                    p.command_id,
                    p.target_chat_id,
                    p.source_point,
                    p.title,
                )
            } else {
                let p: zeron_proto::transfer::MergeThreadBackParams =
                    serde_json::from_value(params)
                        .map_err(|e| RpcError::BadParams(e.to_string()))?;
                (
                    p.chat_id,
                    p.command_id,
                    p.target_chat_id,
                    p.source_point,
                    None,
                )
            };
            for id in [&chat, &key, &target] {
                if id.trim().is_empty() || id.len() > 512 {
                    return Err(RpcError::BadParams(
                        "Transfer identities must be nonempty and at most 512 bytes.".into(),
                    ));
                }
            }
            let owner = workspace
                .chat(&chat)
                .map_err(|e| RpcError::Failed(e.to_string()))?
                .filter(|c| c.device_id == workspace.device_id())
                .ok_or_else(|| {
                    RpcError::Failed("Conversation transfers require the owning host.".into())
                })?;
            let projection = store
                .thread(&chat.clone().into())
                .map_err(error)?
                .filter(|p| p.thread.deleted_at.is_none())
                .ok_or_else(|| RpcError::Failed("The source thread was not found.".into()))?;
            let merging = method == methods::MERGE_THREAD_BACK;
            if merging {
                if !workspace
                    .chat(&target)
                    .map_err(|e| RpcError::Failed(e.to_string()))?
                    .is_some_and(|c| c.device_id == owner.device_id)
                {
                    return Err(RpcError::Failed(
                        "Merge-back requires a target on the same owning host.".into(),
                    ));
                }
                let target_projection = store
                    .thread_in_project(&target.clone().into(), Some(&projection.thread.project_id))
                    .map_err(error)?
                    .filter(|p| {
                        p.thread.deleted_at.is_none()
                            && p.thread.archived_at.is_none()
                            && p.thread.project_id == projection.thread.project_id
                    })
                    .ok_or_else(|| {
                        RpcError::Failed(
                            "The target thread was not found in the source project.".into(),
                        )
                    })?;
                if target_projection.thread.lineage.relationship_to_parent.as_ref()
                    == Some(&zeron_proto::orchestration::OrchestrationV2AppThreadLineageRelationshipToParent::Subagent) {
                    return Err(RpcError::Failed("A delegated child cannot receive a user merge-back.".into()));
                }
            } else if let Some(existing) = workspace
                .chat(&target)
                .map_err(|e| RpcError::Failed(e.to_string()))?
                && existing.device_id != owner.device_id
            {
                return Err(RpcError::Failed(
                    "The target identity belongs to another host.".into(),
                ));
            }
            let operation = if merging {
                super::transfer::TransferOperation::MergeBack {
                    target: target.clone().into(),
                    source: serde_json::from_value(json!(source))
                        .map_err(|e| RpcError::BadParams(e.to_string()))?,
                }
            } else {
                super::transfer::TransferOperation::Fork {
                    target: target.clone().into(),
                    source: serde_json::from_value(json!(source))
                        .map_err(|e| RpcError::BadParams(e.to_string()))?,
                    title: title.filter(|s| !s.trim().is_empty()),
                }
            };
            // Scope command identities by source and operation. A retry uses
            // the same transfer/target, even after a response was lost.
            let command = zeron_proto::orchestration::CommandId(format!(
                "ui:{}:{}:{}",
                if merging { "merge-back" } else { "fork" },
                super::event::encode_component(&chat),
                super::event::encode_component(&key)
            ));
            let prior = store.receipt(&command).map_err(error)?;
            // A stored refusal replays as the same refusal, never as a transport
            // failure that would pin the client's retry forever.
            if let Some(prior) = prior
                .as_ref()
                .filter(|r| r.status != super::ReceiptStatus::Accepted)
            {
                return RpcReply::value(&zeron_proto::transfer::ThreadTransferResult {
                    target_chat_id: target,
                    sequence: prior.result_sequence,
                    refusal: Some(prior.error.clone().unwrap_or_else(|| "Transfer refused.".into())),
                    chat: None,
                });
            }
            let replay = prior.is_some();
            if !merging
                && !replay
                && workspace
                    .chat(&target)
                    .map_err(|e| RpcError::Failed(e.to_string()))?
                    .is_some()
            {
                return Err(RpcError::Failed("The target thread already exists.".into()));
            }
            if replay
                && !store
                    .thread_transfers(&target.clone().into())
                    .map_err(error)?
                    .iter()
                    .any(|t| {
                        t["id"]
                            == format!("transfer:{}", super::event::encode_component(&command.0))
                            && t["sourceThreadId"] == chat
                            && t["targetThreadId"] == target
                    })
            {
                return Err(RpcError::Failed(
                    "This request identity already belongs to a different transfer.".into(),
                ));
            }
            let receipt = service
                .kernel
                .transfer_command(
                    &chat.into(),
                    command,
                    super::transfer::TransferOperation::User(Box::new(operation)),
                )
                .await
                .map_err(error)?;
            if receipt.status != super::ReceiptStatus::Accepted {
                return RpcReply::value(&zeron_proto::transfer::ThreadTransferResult {
                    target_chat_id: target,
                    sequence: receipt.result_sequence,
                    refusal: Some(receipt.error.unwrap_or_else(|| "Transfer refused.".into())),
                    chat: None,
                });
            }
            if !merging {
                let target_projection = store
                    .thread(&target.clone().into())
                    .map_err(error)?
                    .ok_or_else(|| {
                        RpcError::Failed("Accepted fork is not yet available.".into())
                    })?;
                super::assembly::materialize_thread(workspace, registry, &target_projection.thread)
                    .map_err(error)?;
            }
            RpcReply::value(&zeron_proto::transfer::ThreadTransferResult {
                chat: workspace
                    .chat(&target)
                    .map_err(|e| RpcError::Failed(e.to_string()))?,
                target_chat_id: target,
                sequence: receipt.result_sequence,
                refusal: None,
            })
        }
        methods::GET_THREAD_TRANSFER_STATE => {
            let p: zeron_proto::transfer::TransferStateParams =
                serde_json::from_value(params).map_err(|e| RpcError::Failed(e.to_string()))?;
            RpcReply::value(&store.transfer_ui_state(&p.chat_id.into()).map_err(error)?)
        }
        methods::PREVIEW_FILE_CHECKPOINT_RESTORE => {
            let p: zeron_proto::transfer::CheckpointPreviewParams =
                serde_json::from_value(params).map_err(|e| RpcError::Failed(e.to_string()))?;
            RpcReply::value(
                &service
                    .preview(&p.chat_id.into(), &p.checkpoint_id.into())
                    .await
                    .map_err(error)?,
            )
        }
        methods::RESTORE_FILE_CHECKPOINT => {
            let _admission = sessions
                .admit_work()
                .map_err(|e| RpcError::Failed(e.to_string()))?;
            let p: zeron_proto::transfer::CheckpointRestoreParams =
                serde_json::from_value(params).map_err(|e| RpcError::Failed(e.to_string()))?;
            if sessions.turn_in_flight(&p.chat_id) {
                return Err(RpcError::Failed(
                    "File restore requires an idle thread.".into(),
                ));
            }
            RpcReply::value(
                &service
                    .restore(
                        &p.chat_id.into(),
                        &p.checkpoint_id.into(),
                        p.expected_head_sha.as_deref(),
                        &p.expected_checksum,
                    )
                    .await
                    .map_err(error)?,
            )
        }
        _ => unreachable!("transfer RPC route"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_details_preview_carries_only_recent_capped_text_messages() {
        let mut items = vec![json!({"type":"reasoning","text":"private"})];
        for n in 0..150 {
            items.push(json!({"type":"user_message","id":format!("m{n}"),"text":format!("hello {n}")}));
            items.push(json!({"type":"tool_call","id":format!("t{n}"),"text":"ls"}));
        }
        items.push(json!({"type":"assistant_message","id":"empty","text":""}));
        items.push(json!({"type":"assistant_message","id":"huge","text":"x".repeat(50_000)}));
        let preview = preview_items(items);
        assert_eq!(preview.len(), PREVIEW_MESSAGES);
        assert!(
            preview
                .iter()
                .all(|i| matches!(i["type"].as_str(), Some("user_message" | "assistant_message")))
        );
        assert_eq!(preview[0]["id"], "m51");
        let huge = preview.last().unwrap();
        assert_eq!(huge["id"], "huge");
        // One past the client's own 10k cut, so it still shows "shortened".
        assert_eq!(huge["text"].as_str().unwrap().chars().count(), PREVIEW_CHARS + 1);
    }
}
