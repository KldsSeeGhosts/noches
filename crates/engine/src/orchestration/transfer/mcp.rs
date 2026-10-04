use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use zeron_proto::orchestration::{CommandId, Optional, ThreadId};
use zeron_proto::orchestration_mcp::*;

use super::{SourcePoint, TransferOperation};
use crate::orchestration::service::{CallerScope, ToolError};
use crate::orchestration::transfer_service::TransferService;
use crate::orchestration::{Kernel, ReceiptStatus};

pub struct EngineTransferService {
    pub kernel: Kernel,
}

fn unavailable(_: impl std::fmt::Display) -> ToolError {
    ToolError::new(
        OrchestratorMcpFailureCode::OrchestrationError,
        "The operation could not be completed.",
    )
}

impl EngineTransferService {
    fn read(
        &self,
        caller: &CallerScope,
        target: &ThreadId,
        writable: bool,
    ) -> Result<(), ToolError> {
        let parent = self
            .kernel
            .store
            .thread(&caller.thread_id)
            .map_err(unavailable)?
            .filter(|p| p.thread.deleted_at.is_none())
            .ok_or_else(|| {
                ToolError::new(
                    OrchestratorMcpFailureCode::ThreadNotFound,
                    "The calling thread was not found.",
                )
            })?;
        let target = self
            .kernel
            .store
            .thread(target)
            .map_err(unavailable)?
            .filter(|p| {
                p.thread.deleted_at.is_none() && p.thread.project_id == parent.thread.project_id
            })
            .ok_or_else(|| {
                ToolError::new(
                    OrchestratorMcpFailureCode::ThreadNotFound,
                    "The thread was not found in the calling project.",
                )
            })?;
        if writable {
            // T3 checks current ownership, not credential run-id equality.
            if parent.thread.archived_at.is_some()
                || !parent.runs.iter().any(|r| {
                    matches!(
                        r.status,
                        zeron_proto::orchestration::OrchestrationV2RunStatus::Preparing
                            | zeron_proto::orchestration::OrchestrationV2RunStatus::Starting
                            | zeron_proto::orchestration::OrchestrationV2RunStatus::Running
                            | zeron_proto::orchestration::OrchestrationV2RunStatus::Waiting
                    )
                })
                || parent.thread.provider_instance_id != caller.provider_instance_id
            {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::ParentNotActive,
                    "The calling provider no longer owns an active thread run.",
                ));
            }
            if !parent
                .thread
                .runtime_mode
                .permits(target.thread.runtime_mode)
            {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::RuntimeModeEscalationDenied,
                    format!(
                        "Child runtime mode {} is broader than parent mode {}.",
                        json!(target.thread.runtime_mode).as_str().unwrap(),
                        json!(parent.thread.runtime_mode).as_str().unwrap()
                    ),
                ));
            }
            if !parent
                .thread
                .interaction_mode
                .permits(target.thread.interaction_mode)
            {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::InteractionModeEscalationDenied,
                    format!(
                        "Child interaction mode {} is broader than parent mode {}.",
                        json!(target.thread.interaction_mode).as_str().unwrap(),
                        json!(parent.thread.interaction_mode).as_str().unwrap()
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TransferService for EngineTransferService {
    async fn fork(
        &self,
        caller: CallerScope,
        input: T3ThreadForkInput,
    ) -> Result<T3ThreadForkResult, ToolError> {
        self.read(&caller, &caller.thread_id, true)?;
        let id = CommandId(format!("mcp:{}", uuid::Uuid::new_v4()));
        let target = ThreadId(format!("{}:fork", id.0));
        let source: SourcePoint =
            serde_json::from_value(json!(input.source_point)).map_err(unavailable)?;
        let title = match input.title {
            Optional::Present(s) => Some(s),
            Optional::Absent => None,
        };
        let receipt = self
            .kernel
            .transfer_command(
                &caller.thread_id,
                id,
                TransferOperation::Fork {
                    target: target.clone(),
                    source,
                    title,
                },
            )
            .await
            .map_err(unavailable)?;
        if receipt.status != ReceiptStatus::Accepted {
            return Err(unavailable(receipt.error.unwrap_or_default()));
        }
        Ok(T3ThreadForkResult {
            sequence: receipt.result_sequence,
            target_thread_id: target,
        })
    }
    async fn merge_back(
        &self,
        caller: CallerScope,
        input: T3ThreadMergeBackInput,
    ) -> Result<T3ThreadMergeBackResult, ToolError> {
        self.read(&caller, &input.target_thread_id.clone().into(), true)?;
        let source = serde_json::from_value(json!(input.source_point)).map_err(unavailable)?;
        let receipt = self
            .kernel
            .transfer_command(
                &caller.thread_id,
                CommandId(format!("mcp:{}", uuid::Uuid::new_v4())),
                TransferOperation::MergeBack {
                    target: input.target_thread_id.clone().into(),
                    source,
                },
            )
            .await
            .map_err(unavailable)?;
        if receipt.status != ReceiptStatus::Accepted {
            return Err(unavailable(receipt.error.unwrap_or_default()));
        }
        Ok(T3ThreadMergeBackResult {
            sequence: receipt.result_sequence,
            target_thread_id: input.target_thread_id.into(),
        })
    }
    async fn transfers(
        &self,
        caller: CallerScope,
        input: T3ThreadTransfersInput,
    ) -> Result<T3ThreadTransfersResult, ToolError> {
        let target = match input.thread_id {
            Optional::Present(s) => ThreadId(s),
            Optional::Absent => caller.thread_id.clone(),
        };
        self.read(&caller, &target, false)?;
        let rows = self
            .kernel
            .store
            .thread_transfers(&target)
            .map_err(unavailable)?;
        serde_json::from_value(json!({"transfers":rows.into_iter().map(|t| json!({
            "id":t["id"],"sourceThreadId":t["sourceThreadId"],"targetThreadId":t["targetThreadId"],"status":t["status"]
        })).collect::<Vec<_>>()})).map_err(unavailable)
    }
}

pub async fn dispatch(
    service: Option<Arc<dyn TransferService>>,
    caller: CallerScope,
    input: OrchestrationToolInput,
) -> Value {
    let Some(service) = service else {
        return json!({"_tag":"OrchestratorMcpFailure","code":"orchestration_error","message":"The operation could not be completed."});
    };
    let result = match input {
        OrchestrationToolInput::T3ThreadFork(input) => {
            service.fork(caller, *input).await.map(|v| json!(v))
        }
        OrchestrationToolInput::T3ThreadMergeBack(input) => {
            service.merge_back(caller, *input).await.map(|v| json!(v))
        }
        OrchestrationToolInput::T3ThreadTransfers(input) => {
            service.transfers(caller, *input).await.map(|v| json!(v))
        }
        _ => unreachable!("transfer dispatcher only"),
    };
    result.unwrap_or_else(|e| json!(e.into_failure()))
}
