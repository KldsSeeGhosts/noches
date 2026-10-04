use std::sync::Arc;

use serde_json::Value;

use crate::mcp::{auth::InvocationScope, codec};
use crate::orchestration::queue_service::QueueService;

pub async fn dispatch(
    service: Option<Arc<dyn QueueService>>,
    scope: &InvocationScope,
    name: &str,
    input: Value,
) -> Value {
    let result = match service {
        Some(service) => service.call(scope.caller.clone(), name, input).await,
        None => Err(super::unavailable()),
    };
    match result {
        Ok(value) if name == "t3_thread_search" => {
            crate::orchestration::threads::wire::result(value)
        }
        Ok(value) => codec::result(value),
        Err(error) => codec::result(serde_json::to_value(error.into_failure()).expect("failure")),
    }
}
