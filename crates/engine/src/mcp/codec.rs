//! MCP framing and pinned schema boundary. No Rust diagnostic crosses the wire.
use serde_json::{Value, json};

pub fn result(value: Value) -> Value {
    let mut result = json!({"content":[{"type":"text","text":value.to_string()}],"isError":false});
    if value.is_object() {
        result["structuredContent"] = value;
    }
    result
}

pub fn error_text(message: &str) -> Value {
    json!({"content":[{"type":"text","text":message}],"isError":true})
}

pub fn failure(code: &str, message: &str) -> Value {
    json!({"_tag":"OrchestratorMcpFailure","code":code,"message":message})
}

pub fn unavailable() -> Value {
    failure(
        "orchestration_error",
        "The operation could not be completed.",
    )
}

pub fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

pub fn invalid_parameters(name: &str, description: &str) -> Value {
    json!({"_tag":"AiError","module":"Toolkit","method":format!("{name}.handle"),
        "reason":{"_tag":"ToolParameterValidationError","toolName":name,"description":description}})
}

pub fn null_refusal(rows: &Value, value: &Value) -> Option<String> {
    fn visit(value: &Value, path: &[Value], actual: String, description: &str) -> Option<String> {
        let Some((key, rest)) = path.split_first() else {
            return value
                .is_null()
                .then(|| format!("{description}\n  at {actual}"));
        };
        let key = key.as_str()?;
        if key == "*" {
            if let Some(items) = value.as_array() {
                return items
                    .iter()
                    .enumerate()
                    .find_map(|(i, v)| visit(v, rest, format!("{actual}[{i}]"), description));
            }
            return value.as_object()?.iter().find_map(|(key, v)| {
                visit(v, rest, format!("{actual}[{}]", json!(key)), description)
            });
        }
        visit(
            value.get(key)?,
            rest,
            format!("{actual}[{}]", json!(key)),
            description,
        )
    }
    rows.as_array()?.iter().find_map(|row| {
        visit(
            value,
            row["path"].as_array()?,
            String::new(),
            row["description"].as_str()?,
        )
    })
}

pub fn refinements(name: &str, value: &Value) -> Result<(), String> {
    if name != "t3_thread_update" {
        return Ok(());
    }
    if let Some(url) = value["pullRequest"]["url"].as_str()
        && !reqwest::Url::parse(url).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
    {
        return Err(
            "Pull request URL must be a well-formed HTTP(S) URL.\n  at [\"pullRequest\"][\"url\"]"
                .into(),
        );
    }
    let title = value.get("title").is_some();
    let pull_request = value.get("pullRequest").is_some();
    let refusal = match value["action"].as_str() {
        Some("rename") if !title || pull_request => {
            Some("rename requires title and does not accept pullRequest.".into())
        }
        Some("link_pull_request") if !pull_request || title => {
            Some("link_pull_request requires pullRequest and does not accept title.".into())
        }
        Some(action @ ("regenerate_title" | "unlink_pull_request")) if title || pull_request => {
            Some(format!("{action} does not accept title or pullRequest."))
        }
        _ => None,
    };
    refusal.map_or(Ok(()), Err)
}

/// The record shorthand is strict at the tool boundary (unlike persisted
/// model options). Keep the original record/array path in validation errors.
pub fn normalize_target_options(value: Value) -> Result<Value, String> {
    let choice = json!({"anyOf":[{"type":"string","minLength":1},{"type":"boolean"}]});
    let schema = json!({"anyOf":[
        {"type":"array","items":{"type":"object","properties":{
            "id":{"type":"string","minLength":1},"value":choice
        },"required":["id","value"]}},
        {"type":"object","additionalProperties":choice}
    ]});
    let mut value = value;
    decode(&schema, &mut value);
    validate_at(&schema, &schema, &value, "[\"target\"][\"options\"]")?;
    zeron_proto::orchestration::normalize_contract("OrchestratorMcpTargetOptions", value).map_err(
        |_| "Expected a value with a length of at least 1\n  at [\"target\"][\"options\"]".into(),
    )
}

/// Evaluate the JSON Schema vocabulary in the pinned core input inventory.
/// The root owns $defs; recursion never fetches remote references.
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    validate_at(schema, schema, value, "")
}

fn validate_at(root: &Value, schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let at = |message: String| {
        if path.is_empty() {
            message
        } else {
            format!("{message}\n  at {path}")
        }
    };
    let expected = |expected: &str| at(format!("Expected {expected}"));
    let invalid = || expected("a valid value");
    if let Some(reference) = schema["$ref"].as_str() {
        let pointer = reference.strip_prefix('#').ok_or_else(invalid)?;
        return validate_at(
            root,
            root.pointer(pointer).ok_or_else(invalid)?,
            value,
            path,
        );
    }
    if let Some(schemas) = schema["anyOf"].as_array() {
        let failures: Vec<_> = schemas
            .iter()
            .map(|s| validate_at(root, s, value, path))
            .collect();
        if !failures.iter().any(Result::is_ok) {
            // Effect collapses primitive unions and selects the failing
            // branch whose structure/discriminator matches the input.
            let matched = schemas
                .iter()
                .zip(&failures)
                .filter(|(s, _)| shape_matches(s, value))
                .filter_map(|(_, error)| error.as_ref().err());
            if let Some(error) = matched.max_by_key(|error| error.matches('[').count()) {
                return Err(error.clone());
            }
            let alternatives: Vec<_> = schemas
                .iter()
                .filter_map(|s| {
                    s["type"].as_str().map(|t| match t {
                        "array" => "ReadonlyArray",
                        "integer" => "an integer",
                        other => other,
                    })
                })
                .collect();
            return Err(if alternatives.len() == schemas.len() {
                expected(&alternatives.join(" | "))
            } else {
                invalid()
            });
        }
    }
    if let Some(values) = schema["enum"].as_array()
        && !values.contains(value)
    {
        return Err(expected(
            &values
                .iter()
                // Effect reports the mode enum arm, not the separate
                // "inherit" literal arm, for this flattened union.
                .skip(usize::from(matches!(
                    values.first().and_then(Value::as_str),
                    Some("inherit" | "idle")
                )))
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(" | "),
        ));
    }
    if let Some(expected) = schema["const"].as_str()
        && value.as_str() != Some(expected)
    {
        return Err(invalid());
    }
    let matches = match schema["type"].as_str() {
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("integer") => value
            .as_f64()
            .is_some_and(|n| n.is_finite() && n.fract() == 0.0),
        Some("boolean") => value.is_boolean(),
        Some("null") => value.is_null(),
        None => true,
        _ => false,
    };
    if !matches {
        return Err(expected(match schema["type"].as_str() {
            Some("integer") => "an integer",
            Some("object") => "object",
            Some("array") => "ReadonlyArray",
            Some("string") => "string",
            Some("number") => "number",
            Some("boolean") => "boolean",
            Some("null") => "null",
            _ => "a valid value",
        }));
    }
    if let Some(text) = value.as_str() {
        let length = text.encode_utf16().count() as u64;
        if let Some(min) = schema["minLength"].as_u64()
            && length < min
        {
            return Err(expected(&format!(
                "a value with a length of at least {min}"
            )));
        }
        if let Some(max) = schema["maxLength"].as_u64()
            && length > max
        {
            return Err(expected(&format!("a value with a length of at most {max}")));
        }
        if let Some(pattern) = schema["pattern"].as_str()
            && !regex::Regex::new(pattern)
                .map_err(|_| invalid())?
                .is_match(text)
        {
            return Err(expected(&format!("a string matching the RegExp {pattern}")));
        }
    }
    if let Some(number) = value.as_f64() {
        for (key, passes) in [
            (
                "minimum",
                schema["minimum"].as_f64().is_none_or(|n| number >= n),
            ),
            (
                "maximum",
                schema["maximum"].as_f64().is_none_or(|n| number <= n),
            ),
            (
                "exclusiveMinimum",
                schema["exclusiveMinimum"]
                    .as_f64()
                    .is_none_or(|n| number > n),
            ),
            (
                "exclusiveMaximum",
                schema["exclusiveMaximum"]
                    .as_f64()
                    .is_none_or(|n| number < n),
            ),
        ] {
            if !passes {
                let boundary = &schema[key];
                let relation = match key {
                    "minimum" => "greater than or equal to",
                    "maximum" => "less than or equal to",
                    "exclusiveMinimum" => "greater than",
                    _ => "less than",
                };
                return Err(expected(&format!("a value {relation} {boundary}")));
            }
        }
    }
    if let Some(items) = value.as_array() {
        if let Some(min) = schema["minItems"].as_u64()
            && (items.len() as u64) < min
        {
            return Err(expected(&format!(
                "a value with a length of at least {min}"
            )));
        }
        if let Some(max) = schema["maxItems"].as_u64()
            && items.len() as u64 > max
        {
            return Err(expected(&format!("a value with a length of at most {max}")));
        }
        if let Some(item_schema) = schema.get("items") {
            for (i, item) in items.iter().enumerate() {
                validate_at(root, item_schema, item, &format!("{path}[{i}]"))?;
            }
        }
    }
    if let Some(map) = value.as_object() {
        for field in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !map.contains_key(field) {
                return Err(format!("Missing key\n  at {path}[\"{field}\"]"));
            }
        }
        for (key, value) in map {
            if let Some(child) = schema["properties"].get(key) {
                validate_at(root, child, value, &format!("{path}[\"{key}\"]"))?;
            } else if let Some(child) = schema.get("additionalProperties") {
                // Effect's ordinary Struct decoder strips unknown keys even
                // when the published JSON Schema says additionalProperties:false.
                if child.is_object() {
                    validate_at(root, child, value, &format!("{path}[{}]", json!(key)))?;
                }
            }
        }
    }
    if let Some(schemas) = schema["allOf"].as_array() {
        for s in schemas {
            validate_at(root, s, value, path)?;
        }
    }
    Ok(())
}

fn shape_matches(schema: &Value, value: &Value) -> bool {
    if let Some(arms) = schema["anyOf"].as_array() {
        return arms.iter().any(|arm| shape_matches(arm, value));
    }
    match schema["type"].as_str() {
        Some("object") => {
            value.is_object()
                && schema["properties"].as_object().is_none_or(|properties| {
                    properties.iter().all(|(key, child)| {
                        child["enum"].as_array().is_none_or(|choices| {
                            value.get(key).map_or_else(
                                || {
                                    !schema["required"]
                                        .as_array()
                                        .is_some_and(|required| required.contains(&json!(key)))
                                },
                                |v| choices.contains(v),
                            )
                        })
                    })
                })
        }
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("boolean") => value.is_boolean(),
        Some("number" | "integer") => value.is_number(),
        Some("null") => value.is_null(),
        _ => false,
    }
}

/// Trimming precedes length/pattern checks. Actual null rejection is handled
/// from the executed AST boundary, not inferred from JSON's Undefined encoding.
pub fn decode(schema: &Value, value: &mut Value) {
    if let Some(arms) = schema["anyOf"].as_array() {
        let mut fallback = None;
        for arm in arms {
            let mut candidate = value.clone();
            decode(arm, &mut candidate);
            if validate(arm, &candidate).is_ok() {
                *value = candidate;
                return;
            }
            if fallback.is_none() && shape_matches(arm, value) {
                fallback = Some(candidate);
            }
        }
        if let Some(candidate) = fallback {
            *value = candidate;
        }
    }
    if schema.get("minLength").is_some()
        && let Some(text) = value.as_str()
    {
        // ECMAScript WhiteSpace + LineTerminator, not Rust Unicode whitespace.
        *value = json!(text.trim_matches(|c: char| matches!(c,
            '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' |
            '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' |
            '\u{205F}' | '\u{3000}' | '\u{FEFF}')));
    }
    if let Some(map) = value.as_object_mut() {
        let required = schema["required"].as_array();
        if let Some(properties) = schema["properties"].as_object() {
            for (key, child) in properties {
                let is_required = required.is_some_and(|r| r.contains(&json!(key)));
                if !is_required
                    && map.get(key).is_some_and(Value::is_null)
                    && validate(child, &Value::Null).is_err()
                {
                    map.remove(key);
                }
                if let Some(value) = map.get_mut(key) {
                    decode(child, value);
                }
            }
            map.retain(|key, _| properties.contains_key(key));
        } else if schema["additionalProperties"] == false {
            map.clear();
        } else if let Some(child) = schema.get("additionalProperties")
            && child.is_object()
        {
            for value in map.values_mut() {
                decode(child, value);
            }
        }
    }
    if let Some(items) = value.as_array_mut()
        && let Some(child) = schema.get("items")
    {
        for item in items {
            decode(child, item);
        }
    }
}
