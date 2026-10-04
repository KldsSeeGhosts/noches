//! Typed UI API without a gpui dependency.
use zeron_proto::pull_requests::ThreadPullRequestsUi;
impl crate::RpcClient {
    pub async fn thread_pull_requests(
        &self,
        chat_id: &str,
    ) -> Result<ThreadPullRequestsUi, crate::RpcError> {
        self.call_as(
            crate::methods::GET_THREAD_PULL_REQUESTS,
            serde_json::json!({"chatId":chat_id}),
        )
        .await
    }
}
