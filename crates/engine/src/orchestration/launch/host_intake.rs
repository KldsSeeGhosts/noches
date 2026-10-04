//! Attachment intake uses the same commands as ordinary thread sends.
use crate::orchestration::{
    ReceiptStatus,
    launch::unavailable,
    launch_service::{LaunchThreadIntake, SendFailure},
    runner::RunnerBridge,
    thread_service::{ThreadSendRequest, ThreadService},
};
use async_trait::async_trait;
use serde_json::Value;

pub(crate) struct HostLaunchIntake {
    pub bridge: std::sync::Arc<RunnerBridge>,
    pub threads: std::sync::Arc<dyn ThreadService>,
}

#[async_trait]
impl LaunchThreadIntake for HostLaunchIntake {
    async fn send(&self, input: ThreadSendRequest) -> Result<Value, SendFailure> {
        let command_id = input.command_id.clone();
        let result = self.threads.send_to_thread(input).await;
        match result {
            Ok(value) => serde_json::to_value(value).map_err(|_| SendFailure {
                error: unavailable(),
                uncertain: true,
            }),
            Err(error) => {
                let uncertain = self
                    .bridge
                    .kernel
                    .store
                    .receipt(&command_id)
                    .map(|receipt| receipt.is_some_and(|r| r.status == ReceiptStatus::Accepted))
                    .unwrap_or(true);
                Err(SendFailure { error, uncertain })
            }
        }
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
