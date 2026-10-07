//! T3 EffectWorker's late-steer recovery: preserve the accepted message as a
//! separately receipted follow-up, never drop it or reopen a delegated task.
use super::planner::ThreadOperation;
use crate::orchestration::command::{Command, Operation};
use crate::orchestration::effects::Effect;
use crate::orchestration::{Error, Kernel, ReceiptStatus, Result};
use zeron_proto::orchestration::MessageId;

pub(crate) async fn late_steer(kernel: &Kernel, effect: &Effect, id: &MessageId) -> Result<()> {
    let receipt = kernel
        .dispatch(
            &Command {
                id: format!("command:steer-follow-up:{}", effect.id).into(),
                thread_id: effect.thread_id.clone(),
                operation: Operation::Thread(Box::new(ThreadOperation::SteerFollowUp {
                    effect: Box::new(effect.clone()),
                    message_id: id.clone(),
                })),
            },
            crate::now_ms(),
        )
        .await?;
    if receipt.status == ReceiptStatus::Rejected {
        return Err(Error::Invariant(receipt.error.unwrap_or_default()));
    }
    Ok(())
}
