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
    codec::result(match result {
        Ok(value) => value,
        Err(error) => serde_json::to_value(error.into_failure()).expect("failure"),
    })
}
