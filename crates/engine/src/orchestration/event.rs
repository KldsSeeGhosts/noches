use serde::{Deserialize, Serialize};
use zeron_proto::orchestration::{CommandId, EventId, OrchestrationV2DomainEvent, ThreadId};

use super::{Error, Result};

pub const APPLICATION_EVENT_VERSION: i64 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub application_event_version: i64,
    pub sequence: i64,
    pub stream_version: i64,
    pub command_id: Option<CommandId>,
    pub event: OrchestrationV2DomainEvent,
}

pub(crate) fn iso(now_ms: i64) -> Result<String> {
    chrono::DateTime::from_timestamp_millis(now_ms)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or_else(|| Error::Invariant("timestamp outside supported range".into()))
}

/// Construct through the contracts decoder, keeping optional/nullable fields and
/// discriminator spellings identical to the pinned wire contract.
pub(crate) fn make<T: Serialize>(
    event_id: EventId,
    thread: &ThreadId,
    event_type: &str,
    payload: &T,
    now_ms: i64,
) -> Result<OrchestrationV2DomainEvent> {
    let payload = serde_json::to_value(payload)?;
    let mut value = serde_json::json!({
        "id": event_id,
        "threadId": thread,
        "type": event_type,
        "occurredAt": iso(now_ms)?,
        "payload": payload,
    });
    for field in ["runId", "providerInstanceId"] {
        if let Some(field_value) = payload.get(field)
            && !field_value.is_null()
        {
            value[field] = field_value.clone();
        }
    }
    Ok(serde_json::from_value(value)?)
}

/// JS encodeURIComponent, including its unescaped punctuation (not form/URL
/// encoding). Stable IDs are scoped by the MCP credential's provider session.
pub fn encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            write!(&mut encoded, "%{byte:02X}").expect("string formatting");
        }
    }
    encoded
}

pub fn mcp_command_id(session: &str, operation: &str, client_request_id: &str) -> CommandId {
    CommandId(format!(
        "command:mcp:{}:{}:{}",
        encode_component(session),
        encode_component(operation),
        encode_component(client_request_id)
    ))
}
