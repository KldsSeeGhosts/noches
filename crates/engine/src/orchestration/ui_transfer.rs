//! Passive lineage, provenance and files-only timeline. Never acknowledges a task.
use super::{Error, Result, Store};
use serde_json::{Value, json};
use zeron_proto::orchestration::ThreadId;
use zeron_proto::transfer::ThreadTransferState;
use zeron_rpc::{RpcError, RpcReply, methods};

impl Store {
    pub fn transfer_ui_state(&self, id: &ThreadId) -> Result<ThreadTransferState> {
        let projection = self
            .thread(id)?
            .ok_or_else(|| Error::Invariant("The thread was not found.".into()))?;
        let inherited = super::transfer::inherited_items(self, &projection)?;
        let transfers = self.thread_transfers(id)?;
        let handoffs = super::task::records(&projection, "context-handoff")
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
            version: projection.through_sequence,
            lineage: json!(projection.thread.lineage),
            forked_from: projection.thread.forked_from.as_ref().map(|f| json!(f)),
            transfers,
            handoffs,
            checkpoints: self.checkpoint_timeline(id)?,
            inherited_items: inherited,
        })
    }
}

pub(crate) async fn rpc(
    store: Option<&Store>,
    method: &str,
    params: Value,
    sessions: &crate::SessionsEngine,
) -> std::result::Result<RpcReply, RpcError> {
    let store = store.ok_or_else(|| RpcError::Failed("Transfer service is unavailable.".into()))?;
    let service = super::checkpoint::FileCheckpointService {
        kernel: super::Kernel::from_store(store.clone()),
    };
    let error = |e: Error| RpcError::Failed(e.to_string());
    match method {
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
