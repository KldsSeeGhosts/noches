//! T3 EffectWorker's late-steer recovery: preserve the accepted message as a
//! separately receipted follow-up, never drop it or reopen a delegated task.
use super::planner::{Send, ThreadOperation};
use crate::orchestration::command::{Command, Operation};
use crate::orchestration::effects::Effect;
use crate::orchestration::task::records;
use crate::orchestration::{Error, Kernel, ReceiptStatus, Result};
use zeron_proto::orchestration::MessageId;
use zeron_proto::orchestration_mcp::T3ThreadSendInputMode;

pub(crate) async fn late_steer(kernel: &Kernel, effect: &Effect, id: &MessageId) -> Result<()> {
    let projection = kernel
        .store
        .thread(&effect.thread_id)?
        .ok_or_else(|| Error::Invariant("Steer follow-up thread missing.".into()))?;
    let message = records(&projection, "message")
        .iter()
        .find(|m| m["id"] == id.0)
        .ok_or_else(|| Error::Invariant("Steer follow-up message missing.".into()))?;
    let driver = records(&projection, "provider-thread")
        .iter()
        .find(|p| p["providerInstanceId"] == projection.thread.provider_instance_id.0)
        .and_then(|p| p["driver"].as_str())
        .ok_or_else(|| Error::Invariant("Steer follow-up provider missing.".into()))?;
    let receipt = kernel
        .dispatch(
            &Command {
                id: format!("command:steer-follow-up:{}", effect.id).into(),
                thread_id: effect.thread_id.clone(),
                operation: Operation::Thread(Box::new(ThreadOperation::Send(Send {
                    message_id: id.clone(),
                    text: message["text"].as_str().unwrap_or_default().into(),
                    mode: T3ThreadSendInputMode::Auto,
                    driver: driver.into(),
                    sender: message["senderThreadId"]
                        .as_str()
                        .unwrap_or(&effect.thread_id.0)
                        .into(),
                    target_run: None,
                }))),
            },
            crate::now_ms(),
        )
        .await?;
    if receipt.status == ReceiptStatus::Rejected {
        return Err(Error::Invariant(receipt.error.unwrap_or_default()));
    }
    Ok(())
}
