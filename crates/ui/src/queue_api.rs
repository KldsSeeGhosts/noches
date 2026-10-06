//! Typed passive queue/question reads and owner-routed lifecycle commands.
use zeron_proto::QueueUiState;
use zeron_proto::orchestration_mcp::T3ThreadOrganizeInputAction;

use crate::state::EngineHandle;

impl EngineHandle {
    pub async fn mutate_queued_run(
        &self,
        request: zeron_proto::MutateQueuedRunParams,
    ) -> Result<zeron_proto::MutateQueuedRunResult, String> {
        let value = self
            .client()
            .call(
                zeron_rpc::methods::MUTATE_QUEUED_RUN,
                serde_json::to_value(request).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_value(value).map_err(|e| e.to_string())
    }

    pub async fn queue_state(&self, chat_id: &str) -> Result<Option<QueueUiState>, String> {
        let value = self
            .client()
            .call(
                zeron_rpc::methods::GET_QUEUE_STATE,
                serde_json::json!({"chatId":chat_id}),
            )
            .await
            .map_err(|e| e.to_string())?;
        if value.is_null() {
            return Ok(None);
        }
        serde_json::from_value(value)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    pub async fn organize_thread(
        &self,
        chat_id: &str,
        action: T3ThreadOrganizeInputAction,
        snoozed_until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<i64, String> {
        let mut input = serde_json::json!({"chatId":chat_id,"action":action});
        if let Some(time) = snoozed_until {
            input["snoozedUntil"] = serde_json::json!(time);
        }
        let value = self
            .client()
            .call(zeron_rpc::methods::ORGANIZE_THREAD, input)
            .await
            .map_err(|e| e.to_string())?;
        value["sequence"]
            .as_i64()
            .ok_or_else(|| "Invalid organization response.".into())
    }

    pub async fn acknowledge_thread_woke(&self, chat_id: &str) -> Result<(), String> {
        self.client()
            .call(
                zeron_rpc::methods::ACKNOWLEDGE_THREAD_WOKE,
                serde_json::json!({"chatId":chat_id}),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lifecycle_accessor_has_the_designer_contract() {
        let mut state = crate::state::AppState::new();
        assert!(state.chat_lifecycle("thread").is_none());
        let lifecycle = zeron_proto::ChatLifecycle {
            woke_at: Some(chrono::Utc::now()),
            ..Default::default()
        };
        state
            .thread_lifecycles
            .insert("thread".into(), lifecycle.clone());
        assert_eq!(state.chat_lifecycle("thread"), Some(&lifecycle));
    }
}
