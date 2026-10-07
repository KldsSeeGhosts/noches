//! Pi tool/result records → Noches [`AgentEvent`] payloads.
//!
//! Shapes were recorded from a live Pi 1.0.4 (`write {path, content}`,
//! `edit {path, edits: [{oldText, newText}]}`, `bash {command}`, `read {path}`,
//! `grep {pattern, path}`, `find`/`ls`); unknown or extension tools stay
//! `Unknown`/`Mcp` rather than being guessed into a built-in shape.

use serde_json::Value;
use zeron_proto::{TodoItem, TodoStatus, ToolCall, ToolDiff};

/// Cap on tool output crossing the event stream (the doc fold applies its own
/// byte cap before anything persists).
pub(crate) const OUTPUT_CAP: usize = 16 * 1024;
/// Cap on each side of an inline diff.
pub(crate) const DIFF_TEXT_CAP: usize = 64 * 1024;

fn cap_text(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_owned();
    }
    let mut end = cap;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… [truncated]", &text[..end])
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn path_of(args: &Value) -> Option<String> {
    ["path", "file_path", "filePath"]
        .iter()
        .find_map(|key| string(args, key))
        .map(str::to_owned)
}

/// `(old, new)` text of an edit. The current schema carries `edits: [..]`;
/// older Pi versions put `oldText`/`newText` at the top level.
fn edit_texts(args: &Value) -> (Option<String>, Option<String>) {
    let mut olds = Vec::new();
    let mut news = Vec::new();
    let flat = std::iter::once(args);
    let nested = args
        .get("edits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for edit in flat.chain(nested) {
        if let Some(old) = edit.get("oldText").and_then(Value::as_str) {
            olds.push(old);
        }
        if let Some(new) = edit.get("newText").and_then(Value::as_str) {
            news.push(new);
        }
    }
    let join = |parts: Vec<&str>| (!parts.is_empty()).then(|| cap_text(&parts.join("\n"), DIFF_TEXT_CAP));
    (join(olds), join(news))
}

/// Split Pi's extension tool names: the Noches MCP bridge registers
/// `mcp__<server>__<tool>`.
fn mcp_parts(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    (!server.is_empty() && !tool.is_empty()).then_some((server, tool))
}

pub(crate) fn tool_call(name: &str, args: &Value) -> ToolCall {
    let text = |key: &str| string(args, key).map(str::to_owned);
    match name {
        "bash" => ToolCall::Exec {
            command: text("command").unwrap_or_default(),
        },
        "read" => ToolCall::ReadFile {
            path: path_of(args).unwrap_or_default(),
        },
        "write" => ToolCall::WriteFile {
            path: path_of(args).unwrap_or_default(),
            content: text("content"),
        },
        "edit" => {
            let (old_string, new_string) = edit_texts(args);
            ToolCall::EditFile {
                path: path_of(args).unwrap_or_default(),
                old_string,
                new_string,
            }
        }
        "grep" => ToolCall::Search {
            pattern: text("pattern")
                .or_else(|| text("query"))
                .unwrap_or_default(),
            path: text("path"),
        },
        "find" | "glob" => ToolCall::Glob {
            pattern: text("pattern")
                .or_else(|| text("glob"))
                .or_else(|| text("path"))
                .unwrap_or_default(),
        },
        "ls" => ToolCall::Glob {
            pattern: text("path").unwrap_or_else(|| ".".into()),
        },
        "web_search" | "websearch" | "search_web" => ToolCall::WebSearch {
            query: text("query")
                .or_else(|| text("q"))
                .or_else(|| text("searchTerm"))
                .unwrap_or_default(),
        },
        "fetch" | "web_fetch" | "webfetch" => ToolCall::WebFetch {
            url: text("url").unwrap_or_default(),
            prompt: text("prompt"),
        },
        "todo" | "todowrite" | "todo_write" => ToolCall::Todo {
            items: todo_items(args),
        },
        // The official Pi subagent extension spawns children; name the chip
        // after the task like every other driver's spawn call.
        "subagent" => ToolCall::Unknown {
            name: text("description")
                .or_else(|| text("agent"))
                .map(|task| format!("Agent: {task}"))
                .unwrap_or_else(|| "Agent".into()),
            input: Some(args.clone()),
        },
        _ => match mcp_parts(name) {
            Some((server, tool)) => ToolCall::Mcp {
                server: server.to_owned(),
                tool: tool.to_owned(),
                input: Some(args.clone()),
            },
            None => ToolCall::Unknown {
                name: name.to_owned(),
                input: Some(args.clone()),
            },
        },
    }
}

fn todo_items(args: &Value) -> Vec<TodoItem> {
    args.get("todos")
        .or_else(|| args.get("items"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let text = ["content", "text", "task"]
                .iter()
                .find_map(|key| entry.get(*key).and_then(Value::as_str))?;
            let status = if entry.get("done").and_then(Value::as_bool) == Some(true) {
                TodoStatus::Completed
            } else {
                TodoStatus::parse(entry.get("status").and_then(Value::as_str).unwrap_or(""))
            };
            Some(TodoItem::new(text, status))
        })
        .collect()
}

/// The inline diff for a finished `edit`/`write`, reconstructed from the call
/// itself (Pi's own result carries a rendered patch, not before/after text).
pub(crate) fn tool_diff(name: &str, args: &Value) -> Option<ToolDiff> {
    let path = path_of(args)?;
    match name {
        "write" => Some(ToolDiff {
            path,
            old_text: None,
            new_text: cap_text(string(args, "content")?, DIFF_TEXT_CAP),
        }),
        "edit" => {
            let (old, new) = edit_texts(args);
            Some(ToolDiff {
                path,
                old_text: Some(old.unwrap_or_default()),
                new_text: new?,
            })
        }
        _ => None,
    }
}

/// Text blocks of a Pi content array, joined and capped; `None` when empty.
pub(crate) fn content_text(content: &Value) -> Option<String> {
    let text: String = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => return None,
    };
    (!text.is_empty()).then(|| cap_text(&text, OUTPUT_CAP))
}

/// Output of a `tool_execution_end`/`update` result object.
pub(crate) fn result_output(result: &Value) -> Option<String> {
    content_text(result.get("content")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builtin_tools_type_to_their_chips() {
        assert_eq!(
            tool_call("bash", &json!({"command":"ls"})),
            ToolCall::Exec { command: "ls".into() }
        );
        assert_eq!(
            tool_call("read", &json!({"path":"a.rs"})),
            ToolCall::ReadFile { path: "a.rs".into() }
        );
        assert_eq!(
            tool_call("grep", &json!({"pattern":"bye","path":"hello.txt"})),
            ToolCall::Search {
                pattern: "bye".into(),
                path: Some("hello.txt".into())
            }
        );
        assert_eq!(
            tool_call("ls", &json!({})),
            ToolCall::Glob { pattern: ".".into() }
        );
        assert_eq!(
            tool_call("find", &json!({"pattern":"*.rs"})),
            ToolCall::Glob { pattern: "*.rs".into() }
        );
    }

    #[test]
    fn edit_reads_the_current_and_legacy_schemas() {
        // Recorded live: Pi 1.0.4.
        let current = json!({"path":"hello.txt","edits":[{"newText":"bye","oldText":"hi"}]});
        assert_eq!(
            tool_call("edit", &current),
            ToolCall::EditFile {
                path: "hello.txt".into(),
                old_string: Some("hi".into()),
                new_string: Some("bye".into())
            }
        );
        let legacy = json!({"path":"a","oldText":"x","newText":"y"});
        assert_eq!(
            tool_call("edit", &legacy),
            ToolCall::EditFile {
                path: "a".into(),
                old_string: Some("x".into()),
                new_string: Some("y".into())
            }
        );
        let diff = tool_diff("edit", &current).unwrap();
        assert_eq!(diff.old_text.as_deref(), Some("hi"));
        assert_eq!(diff.new_text, "bye");
        let write = tool_diff("write", &json!({"path":"hello.txt","content":"hi there"})).unwrap();
        assert!(write.old_text.is_none());
        assert_eq!(write.new_text, "hi there");
        assert!(tool_diff("bash", &json!({"command":"ls"})).is_none());
        assert!(tool_diff("write", &json!({"content":"no path"})).is_none());
    }

    #[test]
    fn extension_tools_keep_their_identity() {
        assert_eq!(
            tool_call("mcp__t3-code__task_status", &json!({"a":1})),
            ToolCall::Mcp {
                server: "t3-code".into(),
                tool: "task_status".into(),
                input: Some(json!({"a":1}))
            }
        );
        assert!(matches!(
            tool_call("ask_user_question", &json!({})),
            ToolCall::Unknown { name, .. } if name == "ask_user_question"
        ));
        assert!(matches!(
            tool_call("mcp__bad", &json!({})),
            ToolCall::Unknown { .. }
        ));
        assert!(tool_call("subagent", &json!({"description":"scan"})).is_subagent_spawn());
        assert!(matches!(
            tool_call("todo", &json!({"todos":[{"content":"a","status":"completed"}]})),
            ToolCall::Todo { items } if items.len() == 1
        ));
    }

    #[test]
    fn output_joins_text_blocks_and_caps_long_results() {
        assert_eq!(
            content_text(&json!([{"type":"text","text":"a"},{"type":"image"},{"type":"text","text":"b"}])),
            Some("ab".into())
        );
        assert_eq!(content_text(&json!([])), None);
        assert_eq!(content_text(&json!("plain")), Some("plain".into()));
        let long = "é".repeat(OUTPUT_CAP);
        let capped = content_text(&json!(long)).unwrap();
        assert!(capped.ends_with("… [truncated]"));
        assert!(capped.len() <= OUTPUT_CAP + 20);
        assert_eq!(
            result_output(&json!({"content":[{"type":"text","text":"Command aborted"}],"details":{}})),
            Some("Command aborted".into())
        );
    }
}
