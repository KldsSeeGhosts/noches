//! TODO(merge-threads): use canonical thread intake for auto steering/refusals.
use crate::orchestration::{
    Command, Operation, ReceiptStatus,
    launch::{LaunchOperation, unavailable},
    launch_service::{LaunchThreadIntake, SendFailure},
    runner::RunnerBridge,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use zeron_proto::orchestration::{CommandId, ThreadId};

pub(crate) struct HostLaunchIntake {
    pub bridge: std::sync::Arc<RunnerBridge>,
}

#[async_trait]
impl LaunchThreadIntake for HostLaunchIntake {
    async fn send(
        &self,
        thread: &str,
        message_id: &str,
        text: &str,
        attachments: Vec<Value>,
        queue: bool,
    ) -> Result<Value, SendFailure> {
        let driver = self
            .bridge
            .kernel
            .store
            .thread(&ThreadId(thread.into()))
            .ok()
            .flatten()
            .and_then(|p| {
                crate::orchestration::task::records(&p, "provider-thread")
                    .last()
                    .and_then(|p| p["driver"].as_str())
                    .map(str::to_owned)
            })
            .unwrap_or("unknown".into());
        let receipt = self
            .bridge
            .kernel
            .dispatch(
                &Command {
                    id: CommandId(message_id.into()),
                    thread_id: ThreadId(thread.into()),
                    operation: Operation::Launch(Box::new(LaunchOperation::Send {
                        message_id: message_id.into(),
                        text: text.into(),
                        attachments,
                        queue,
                        driver,
                    })),
                },
                crate::now_ms(),
            )
            .await
            .map_err(|_| SendFailure {
                error: unavailable(),
                uncertain: true,
            })?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(SendFailure {
                error: unavailable(),
                uncertain: false,
            });
        }
        let p = self
            .bridge
            .kernel
            .store
            .thread(&ThreadId(thread.into()))
            .map_err(|_| SendFailure {
                error: unavailable(),
                uncertain: true,
            })?
            .unwrap();
        let run = p
            .runs
            .iter()
            .find(|r| r.user_message_id.0 == message_id)
            .ok_or_else(|| SendFailure {
                error: unavailable(),
                uncertain: true,
            })?;
        Ok(json!({"threadId":thread,"messageId":message_id,"runId":run.id,"status":run.status}))
    }
    async fn detach(&self, thread: &str) -> Result<(), crate::orchestration::service::ToolError> {
        self.bridge
            .sessions
            .interrupt(thread)
            .await
            .map_err(|_| unavailable())?;
        Ok(())
    }
}
