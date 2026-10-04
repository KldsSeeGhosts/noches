//! Toolkit seam: validation remains at the pinned MCP schema boundary.
use serde_json::Value;
use std::sync::{Arc, PoisonError, RwLock};
use zeron_proto::orchestration_mcp::OrchestrationToolInput;

use crate::mcp::{auth::InvocationScope, codec};
use crate::orchestration::thread_service::ThreadService;

pub async fn dispatch(
    services: &RwLock<Option<Arc<dyn ThreadService>>>,
    scope: &InvocationScope,
    input: OrchestrationToolInput,
) -> Value {
    let service = services
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let Some(service) = service else {
        return codec::result(codec::unavailable());
    };
    macro_rules! call {
        ($method:ident,$input:expr) => {
            match service.$method(scope.caller.clone(), *$input).await {
                Ok(result) => codec::result(serde_json::to_value(result).expect("thread result")),
                Err(error) => codec::result(
                    serde_json::to_value(error.into_failure()).expect("thread failure"),
                ),
            }
        };
    }
    match input {
        OrchestrationToolInput::T3ThreadList(i) => call!(list, i),
        OrchestrationToolInput::T3ThreadRead(i) => {
            match service.read(scope.caller.clone(), *i).await {
                Ok(result) => super::wire::result(result.wire_value()),
                Err(error) => codec::result(
                    serde_json::to_value(error.into_failure()).expect("thread failure"),
                ),
            }
        }
        OrchestrationToolInput::T3ThreadSend(i) => call!(send, i),
        OrchestrationToolInput::T3ThreadWait(i) => call!(wait, i),
        OrchestrationToolInput::T3ThreadInterrupt(i) => call!(interrupt, i),
        OrchestrationToolInput::T3ThreadConfiguration(i) => call!(configuration, i),
        OrchestrationToolInput::T3ThreadConfigure(i) => call!(configure, i),
        OrchestrationToolInput::CreateThreads(i) => call!(create, i),
        _ => codec::result(codec::unavailable()),
    }
}
