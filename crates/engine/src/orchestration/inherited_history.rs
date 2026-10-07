//! Paged, read-only view of the history a fork inherited. Passive: it never
//! touches delivery, handoff context or either conversation's document.
//!
//! The inherited items are frozen at the fork's source run, so the entry list
//! is stable while the parent keeps running; a cursor is the id of the oldest
//! entry the client already holds, never a position. Every part is bounded
//! here (the Details state stays text-only and small), and a page stops early
//! once its byte budget is spent.
use super::{Error, Result, Store};
use serde_json::Value;
use zeron_doc::{MessagePart, MessageRole, SessionMessageEntry, ToolDiffStat};
use zeron_proto::ToolCall;
use zeron_proto::orchestration::ThreadId;
use zeron_proto::transfer::InheritedHistoryPage;

const DEFAULT_PAGE: usize = 24;
const MAX_PAGE: usize = 50;
/// Serialized-text budget for one page; a single oversized entry still ships.
const PAGE_BYTES: usize = 384 * 1024;
const TEXT_CHARS: usize = 10_000;
const REASONING_CHARS: usize = 4_000;
const OUTPUT_CHARS: usize = 6_000;
const COMMAND_CHARS: usize = 2_000;
/// Inline diff sides above this fall back to per-file stats.
const DIFF_SIDE_CHARS: usize = 20_000;
const LISTED_RESULTS: usize = 40;
const REFS_MARKER: &str = "\n\nAttached images (local files";

/// Cut to `limit` chars; reports whether anything was lost.
fn cap(text: &str, limit: usize) -> (String, bool) {
    match text.char_indices().nth(limit) {
        Some((end, _)) => (format!("{}…", &text[..end]), true),
        None => (text.to_owned(), false),
    }
}

/// A user prompt keeps its attachment-ref trailer intact: the refs are what
/// the client turns into thumbnails, so only the prose is shortened.
fn cap_prompt(text: &str) -> (String, bool) {
    match text.find(REFS_MARKER) {
        Some(split) => {
            let (body, cut) = cap(&text[..split], TEXT_CHARS);
            (format!("{body}{}", &text[split..]), cut)
        }
        None => cap(text, TEXT_CHARS),
    }
}

struct Entry {
    entry: SessionMessageEntry,
    shortened: u32,
}

struct Builder<'a> {
    id: String,
    run: Option<&'a str>,
    parts: Vec<MessagePart>,
    shortened: u32,
}

impl<'a> Builder<'a> {
    fn finish(self) -> Option<Entry> {
        (!self.parts.is_empty()).then(|| Entry {
            entry: SessionMessageEntry {
                id: self.id,
                role: MessageRole::Assistant,
                parts: self.parts,
                // Inherited turn-items carry no wall-clock the UI may trust.
                created_at: 0,
                device_id: String::new(),
                status: None,
                continuation_of: None,
            },
            shortened: self.shortened,
        })
    }
}

fn identity(item: &Value) -> Option<String> {
    let source = item["sourceThreadId"]
        .as_str()
        .or_else(|| item["threadId"].as_str())?;
    let id = item["sourceItemId"].as_str().or_else(|| item["id"].as_str())?;
    // JSON tuple encoding prevents collisions between nested source IDs.
    Some(format!(
        "inherited:{}",
        serde_json::to_string(&(source, id)).ok()?
    ))
}

fn failed(item: &Value) -> bool {
    matches!(item["status"].as_str(), Some("failed" | "error"))
        || item["outputIndicatesFailure"] == true
        || item["exitCode"].as_i64().is_some_and(|code| code != 0)
}

fn tool(
    id: String,
    call: ToolCall,
    item: &Value,
    output: Option<String>,
    shortened: &mut u32,
) -> MessagePart {
    let output = output
        .filter(|text| !text.trim().is_empty())
        .map(|text| {
            let (text, cut) = cap(&text, OUTPUT_CHARS);
            *shortened += u32::from(cut);
            text
        });
    MessagePart::Tool {
        id,
        call,
        is_error: failed(item),
        resolved: !matches!(item["status"].as_str(), Some("running" | "pending" | "streaming")),
        output,
        diff: None,
        output_ref: None,
        output_bytes: None,
        diff_ref: None,
        diff_stats: None,
        subagent_ref: None,
        subagent_status: None,
        subagent_tail: None,
    }
}

/// Reduce one turn item to a transcript part. Approvals, input requests,
/// handoff/fork markers and checkpoints are provenance or live controls, not
/// conversation, so they never render in an inherited view.
fn part(item: &Value, shortened: &mut u32) -> Option<MessagePart> {
    let id = item["id"].as_str()?.to_owned();
    let text = |key: &str| item[key].as_str().filter(|text| !text.trim().is_empty());
    Some(match item["type"].as_str()? {
        "assistant_message" => {
            let (text, cut) = cap(text("text")?, TEXT_CHARS);
            *shortened += u32::from(cut);
            MessagePart::Text { id, text }
        }
        "proposed_plan" => {
            let (text, cut) = cap(text("markdown")?, TEXT_CHARS);
            *shortened += u32::from(cut);
            MessagePart::Text { id, text }
        }
        "reasoning" => {
            let (text, cut) = cap(text("text")?, REASONING_CHARS);
            *shortened += u32::from(cut);
            MessagePart::Reasoning { id, text }
        }
        "error" => MessagePart::Error {
            id,
            message: item["failure"]["message"].as_str()?.to_owned(),
        },
        "run_interrupt_result" => MessagePart::Text {
            id,
            text: text("message")?.to_owned(),
        },
        "command_execution" => {
            let (command, cut) = cap(text("input")?, COMMAND_CHARS);
            *shortened += u32::from(cut);
            let output = item["output"].as_str().map(str::to_owned);
            tool(id, ToolCall::Exec { command }, item, output, shortened)
        }
        "file_change" => {
            let path = text("fileName")?.to_owned();
            let stat = ToolDiffStat {
                path: path.clone(),
                additions: item["additions"].as_u64().unwrap_or(0),
                deletions: item["deletions"].as_u64().unwrap_or(0),
            };
            let mut part = tool(
                id,
                ToolCall::EditFile {
                    path: path.clone(),
                    old_string: None,
                    new_string: None,
                },
                item,
                None,
                shortened,
            );
            if let MessagePart::Tool {
                diff, diff_stats, ..
            } = &mut part
            {
                let old = item["oldStr"].as_str();
                let new = item["newStr"].as_str();
                match new {
                    Some(new)
                        if new.chars().count() <= DIFF_SIDE_CHARS
                            && old.is_none_or(|old| old.chars().count() <= DIFF_SIDE_CHARS) =>
                    {
                        *diff = Some(zeron_proto::ToolDiff {
                            path,
                            old_text: old.map(str::to_owned),
                            new_text: new.to_owned(),
                        });
                    }
                    _ => {
                        *shortened += u32::from(new.is_some());
                        *diff_stats = Some(vec![stat]);
                    }
                }
            }
            part
        }
        "file_search" => {
            let listed: Vec<String> = item["results"]
                .as_array()
                .map(|results| {
                    results
                        .iter()
                        .take(LISTED_RESULTS)
                        .filter_map(|r| {
                            let file = r["fileName"].as_str()?;
                            Some(match r["line"].as_u64() {
                                Some(line) => format!("{file}:{line}"),
                                None => file.to_owned(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let more = item["results"].as_array().map_or(0, Vec::len).saturating_sub(LISTED_RESULTS);
            *shortened += u32::from(more > 0);
            let mut output = listed.join("\n");
            if more > 0 {
                output.push_str(&format!("\n… {more} more"));
            }
            tool(
                id,
                ToolCall::Search {
                    pattern: item["pattern"].as_str().unwrap_or_default().to_owned(),
                    path: None,
                },
                item,
                Some(output),
                shortened,
            )
        }
        "web_search" => {
            let query = item["patterns"]
                .as_array()
                .map(|p| {
                    p.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" · ")
                })
                .unwrap_or_default();
            let output = item["results"].as_array().map(|results| {
                results
                    .iter()
                    .take(LISTED_RESULTS)
                    .filter_map(|r| {
                        let title = r["title"].as_str().or_else(|| r["url"].as_str())?;
                        Some(match r["url"].as_str().filter(|url| *url != title) {
                            Some(url) => format!("{title}\n{url}"),
                            None => title.to_owned(),
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            tool(id, ToolCall::WebSearch { query }, item, output, shortened)
        }
        "todo_list" => {
            let items = item["steps"]
                .as_array()?
                .iter()
                .filter_map(|step| {
                    Some(zeron_proto::TodoItem::new(
                        step["text"].as_str()?,
                        zeron_proto::TodoStatus::parse(step["status"].as_str().unwrap_or("")),
                    ))
                })
                .collect();
            tool(id, ToolCall::Todo { items }, item, None, shortened)
        }
        "dynamic_tool" => {
            let name = item["toolName"].as_str().unwrap_or("tool").to_owned();
            if let Some(path) = item["viewedImagePath"].as_str() {
                let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned();
                return Some(MessagePart::Image {
                    id,
                    path: path.to_owned(),
                    name,
                    // The client verifies the real type while decoding.
                    mime_type: "image/png".into(),
                });
            }
            let output = match &item["output"] {
                Value::Null => None,
                Value::String(text) => Some(text.clone()),
                other => Some(other.to_string()),
            };
            tool(
                id,
                ToolCall::Unknown {
                    name,
                    input: (!item["input"].is_null()).then(|| item["input"].clone()),
                },
                item,
                output,
                shortened,
            )
        }
        // Named so it cannot pass for a live spawn chip (no doc ref follows).
        "subagent" => {
            let output = text("result").or_else(|| text("progress")).map(str::to_owned);
            tool(
                id,
                ToolCall::Unknown {
                    name: "Subagent task".into(),
                    input: text("prompt").map(|p| serde_json::json!({"description": cap(p, 200).0})),
                },
                item,
                output,
                shortened,
            )
        }
        _ => return None,
    })
}

/// Fold the flat, ordered item list into the entries the transcript renders:
/// one per user prompt, and one per run of consecutive agent work.
fn entries(items: &[Value]) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut open: Option<Builder> = None;
    for item in items {
        let Some(kind) = item["type"].as_str() else {
            continue;
        };
        if kind == "user_message" {
            if let Some(entry) = open.take().and_then(Builder::finish) {
                out.push(entry);
            }
            let (Some(id), Some(text)) = (identity(item), item["text"].as_str()) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            let (text, cut) = cap_prompt(text);
            out.push(Entry {
                entry: SessionMessageEntry {
                    id,
                    role: MessageRole::User,
                    parts: vec![MessagePart::Text {
                        id: "text".into(),
                        text,
                    }],
                    created_at: 0,
                    device_id: String::new(),
                    status: None,
                    continuation_of: None,
                },
                shortened: u32::from(cut),
            });
            continue;
        }
        let run = item["runId"].as_str();
        if open.as_ref().is_some_and(|b| b.run != run) {
            if let Some(entry) = open.take().and_then(Builder::finish) {
                out.push(entry);
            }
        }
        let mut shortened = 0;
        let Some(part) = part(item, &mut shortened) else {
            continue;
        };
        let Some(id) = identity(item) else {
            continue;
        };
        let builder = open.get_or_insert_with(|| Builder {
            id,
            run,
            parts: Vec::new(),
            shortened: 0,
        });
        builder.parts.push(part);
        builder.shortened += shortened;
    }
    if let Some(entry) = open.and_then(Builder::finish) {
        out.push(entry);
    }
    out
}

impl Store {
    /// The page of `entries` that ends just before `before` (or at the newest
    /// entry), oldest first. An unknown cursor is refused rather than guessed,
    /// so the client restarts from the newest page instead of skipping history.
    pub fn inherited_history_page(
        &self,
        id: &ThreadId,
        before: Option<&str>,
        limit: Option<u32>,
    ) -> Result<InheritedHistoryPage> {
        let projection = self
            .thread(id)?
            .ok_or_else(|| Error::Invariant("The thread was not found.".into()))?;
        let all = entries(&super::transfer::inherited_items(self, &projection)?);
        let end = match before {
            None => all.len(),
            Some(cursor) => all
                .iter()
                .position(|e| e.entry.id == cursor)
                .ok_or_else(|| {
                    Error::Invariant("The inherited-history cursor is no longer valid.".into())
                })?,
        };
        let limit = limit.map_or(DEFAULT_PAGE, |n| (n as usize).clamp(1, MAX_PAGE));
        let mut start = end;
        let mut bytes = 0usize;
        while start > 0 && end - start < limit {
            let cost = serde_json::to_string(&all[start - 1].entry).map_or(0, |s| s.len());
            if start < end && bytes + cost > PAGE_BYTES {
                break;
            }
            bytes += cost;
            start -= 1;
        }
        let page = &all[start..end];
        Ok(InheritedHistoryPage {
            thread_id: id.0.clone(),
            entries: page
                .iter()
                .map(|e| serde_json::to_value(&e.entry))
                .collect::<std::result::Result<_, _>>()?,
            next_before: (start > 0).then(|| all[start].entry.id.clone()),
            remaining: start as u64,
            total: all.len() as u64,
            shortened: page.iter().map(|e| e.shortened).sum(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(kind: &str, id: &str, run: &str, extra: Value) -> Value {
        let mut base = json!({
            "type": kind, "id": id, "threadId": "parent", "runId": run,
            "sourceThreadId": "parent", "sourceItemId": id, "status": "completed"
        });
        for (key, value) in extra.as_object().unwrap() {
            base[key] = value.clone();
        }
        base
    }

    fn sample() -> Vec<Value> {
        vec![
            item("user_message", "u1", "r1", json!({"text": "first"})),
            item("reasoning", "t1", "r1", json!({"text": "thinking"})),
            item("command_execution", "c1", "r1", json!({"input": "ls", "output": "a\nb", "exitCode": 0})),
            item("assistant_message", "a1", "r1", json!({"text": "done"})),
            item("user_message", "u2", "r2", json!({"text": "second"})),
            item("file_change", "f1", "r2", json!({"fileName": "x.rs", "additions": 3, "deletions": 1, "newStr": "fn x() {}"})),
            item("approval_request", "p1", "r2", json!({})),
            item("assistant_message", "a2", "r2", json!({"text": "again"})),
        ]
    }

    #[test]
    fn folds_agent_work_between_prompts_and_drops_controls() {
        let folded = entries(&sample());
        assert_eq!(folded.len(), 4);
        assert!(matches!(folded[0].entry.role, MessageRole::User));
        let parts = &folded[1].entry.parts;
        assert!(matches!(parts[0], MessagePart::Reasoning { .. }));
        assert!(matches!(&parts[1], MessagePart::Tool { call: ToolCall::Exec { command }, output: Some(out), .. } if command == "ls" && out == "a\nb"));
        assert!(matches!(parts[2], MessagePart::Text { .. }));
        // The approval request is a live control: absent, not a part.
        assert_eq!(folded[3].entry.parts.len(), 2);
        assert!(matches!(&folded[3].entry.parts[0], MessagePart::Tool { diff: Some(_), .. }));
    }

    #[test]
    fn a_new_run_never_merges_into_the_previous_agent_entry() {
        let folded = entries(&[
            item("assistant_message", "a1", "r1", json!({"text": "one"})),
            item("assistant_message", "a2", "r2", json!({"text": "two"})),
        ]);
        assert_eq!(folded.len(), 2);
    }

    #[test]
    fn prompt_cap_keeps_the_attachment_trailer_whole() {
        let body = "x".repeat(TEXT_CHARS + 50);
        let text = format!("{body}\n\nAttached images (local files — open them to view):\n- /tmp/a.png");
        let (capped, cut) = cap_prompt(&text);
        assert!(cut);
        assert!(capped.ends_with("):\n- /tmp/a.png"));
        assert!(capped.chars().count() < text.chars().count());
    }

    #[test]
    fn oversized_tool_output_is_shortened_and_counted() {
        let folded = entries(&[item(
            "command_execution",
            "c1",
            "r1",
            json!({"input": "cat big", "output": "y".repeat(OUTPUT_CHARS * 2), "exitCode": 1}),
        )]);
        assert_eq!(folded[0].shortened, 1);
        let MessagePart::Tool { output, is_error, .. } = &folded[0].entry.parts[0] else {
            panic!("tool part");
        };
        assert!(*is_error);
        assert_eq!(output.as_ref().unwrap().chars().count(), OUTPUT_CHARS + 1);
    }

    #[test]
    fn viewed_image_becomes_an_image_part() {
        let folded = entries(&[item(
            "dynamic_tool",
            "d1",
            "r1",
            json!({"toolName": "view_image", "viewedImagePath": "/work/shot.png", "input": {}}),
        )]);
        assert!(matches!(&folded[0].entry.parts[0], MessagePart::Image { name, .. } if name == "shot.png"));
    }
}
