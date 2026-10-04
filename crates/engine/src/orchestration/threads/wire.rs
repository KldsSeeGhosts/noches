//! JavaScript strings are UTF-16, including lone surrogates produced by slice.
//! Rust strings cannot represent those. The private marker lives only between
//! toolkit framing and HTTP encoding; it is never persisted or published.
use serde_json::{Value, json};

const UNITS: &str = "\0noches.thread.utf16";

pub fn slice(text: &str, offset: usize, count: usize) -> (String, Vec<u16>, bool) {
    let units: Vec<_> = text.encode_utf16().collect();
    let end = offset.saturating_add(count).min(units.len());
    let part = units
        .get(offset.min(units.len())..end)
        .unwrap_or_default()
        .to_vec();
    (
        String::from_utf16_lossy(&part),
        part,
        units.len() > offset.saturating_add(count),
    )
}

pub fn text_value(units: &[u16]) -> Value {
    match String::from_utf16(units) {
        Ok(text) => json!(text),
        Err(_) => json!({UNITS:units}),
    }
}

fn quoted_units(units: &[u16]) -> String {
    // Escape all units, retaining surrogate boundaries exactly.
    let mut result = String::from("\"");
    for unit in units {
        use std::fmt::Write;
        write!(result, "\\u{unit:04x}").expect("string write");
    }
    result.push('"');
    result
}

/// Only the read result's `items[].text` slots accept private markers. Arbitrary
/// tool output, user JSON and other fields can never trigger raw substitution.
pub fn read_json(value: &Value) -> String {
    let mut safe = value.clone();
    let mut replacements = vec![];
    if let Some(items) = safe.get_mut("items").and_then(Value::as_array_mut) {
        for item in items {
            if let Some(units) = item["text"][UNITS].as_array() {
                let units: Vec<_> = units.iter().map(|n| n.as_u64().unwrap() as u16).collect();
                let token = format!("\0utf16:{}:{}", uuid::Uuid::new_v4(), replacements.len());
                item["text"] = json!(token);
                replacements.push((json!(token).to_string(), quoted_units(&units)));
            }
        }
    }
    let mut encoded = safe.to_string();
    for (token, text) in replacements {
        encoded = encoded.replace(&token, &text);
    }
    encoded
}

/// The final response encoder retains structuredContent's lone surrogates.
pub fn response_json(value: &Value) -> String {
    let Some(content) = value.pointer("/result/structuredContent") else {
        return value.to_string();
    };
    if !content.get("items").is_some_and(Value::is_array) {
        return value.to_string();
    }
    let encoded = read_json(content);
    let mut safe = value.clone();
    let token = format!("\0thread-read:{}", uuid::Uuid::new_v4());
    safe["result"]["structuredContent"] = json!(token);
    safe.to_string()
        .replace(&json!(token).to_string(), &encoded)
}

pub fn result(value: Value) -> Value {
    json!({"content":[{"type":"text","text":read_json(&value)}],
        "structuredContent":value,"isError":false})
}
