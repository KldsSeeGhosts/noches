use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use zeron_rpc::{RpcError, RpcReply, RpcService};

struct PrRead;
#[async_trait]
impl RpcService for PrRead {
    async fn handle(&self, method: &str, params: Value) -> Result<RpcReply, RpcError> {
        assert_eq!(method, zeron_rpc::methods::GET_THREAD_PULL_REQUESTS);
        assert_eq!(params, json!({"chatId":"thread"}));
        Ok(RpcReply::Value(json!({
            "threadId":"thread","version":42,
            "pullRequests":[{
                "host":"github.com","repository":"owner/repo","number":124,
                "url":"https://github.com/owner/repo/pull/124","source":"agent",
                "watching":true,"state":"open","checksState":"pending",
                "stack":{"kind":"derived","position":2,"size":2}
            }],
            "chains":[{"kind":"derived","numbers":[123,124]}]
        })))
    }
}

#[tokio::test]
async fn typed_passive_pr_read_preserves_state_and_bottom_to_top_chain() {
    let client = zeron_rpc::memory_client(Arc::new(PrRead));
    let state = client.thread_pull_requests("thread").await.unwrap();
    assert_eq!(state.thread_id, "thread");
    assert_eq!(state.version, 42);
    assert!(state.pull_requests[0].watching);
    assert_eq!(
        state.pull_requests[0].checks_state,
        Some(zeron_proto::orchestration::PullRequestChecksState::Pending)
    );
    assert_eq!(state.chains[0].numbers, [123, 124]);
    assert_eq!(state.pull_requests[0].stack.as_ref().unwrap().position, 2);
}
