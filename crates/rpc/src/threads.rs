//! Typed UI read calls over the existing authenticated owner RPC transport.
use crate::{RpcClient, RpcError, methods};
use zeron_proto::orchestration_threads::*;

impl RpcClient {
    pub async fn thread_summaries(
        &self,
        input: ThreadSummariesRequest,
    ) -> Result<ThreadSummaries, RpcError> {
        self.call_as(
            methods::GET_THREAD_SUMMARIES,
            serde_json::to_value(input).expect("thread summaries"),
        )
        .await
    }
    pub async fn thread_timeline(
        &self,
        input: ThreadTimelineRequest,
    ) -> Result<ThreadTimeline, RpcError> {
        self.call_as(
            methods::GET_THREAD_TIMELINE,
            serde_json::to_value(input).expect("thread timeline"),
        )
        .await
    }
}
