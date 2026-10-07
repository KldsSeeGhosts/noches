//! Paged, read-only view of the history a fork inherited. Passive: it never
//! touches delivery, handoff context or either conversation's document.
//!
//! The canonical turn items fix the boundary and order and carry the prompts
//! and reply text, but the runner projects only text into them. Tool calls,
//! results, diffs and media live in the source chat's session document, so
//! each inherited run's agent work is read from there (user entry id == the
//! run's user message id; its agent entries follow until the next prompt) and
//! the turn items stand in only when a document no longer has the run.
//!
//! The inherited items are frozen at the fork's source run, so the entry list
//! is stable while the parent keeps running; a cursor is the id of the oldest
//! entry the client already holds, never a position. Every part is bounded
//! here (the Details state stays text-only and small), and a page stops early
//! once its byte budget is spent.
use super::{Error, Result, Store, task::records};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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
const MAX_PARTS: usize = 400;
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

/// Only rasters the client decodes; anything else stays an ordinary tool row.
fn image_mime(path: &str) -> Option<&'static str> {
    let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    })
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
            if let Some((path, mime_type)) = item["viewedImagePath"]
                .as_str()
                .and_then(|path| Some((path, image_mime(path)?)))
            {
                let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned();
                return Some(MessagePart::Image {
                    id,
                    path: path.to_owned(),
                    name,
                    mime_type: mime_type.into(),
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

/// Doc parts that are live controls or private to the source conversation
/// never render in an inherited view.
fn bound_part(part: &MessagePart, shortened: &mut u32) -> Option<MessagePart> {
    fn cut(text: &str, limit: usize, shortened: &mut u32) -> String {
        let (text, was_cut) = cap(text, limit);
        *shortened += u32::from(was_cut);
        text
    }
    Some(match part {
        MessagePart::Text { id, text } => MessagePart::Text {
            id: id.clone(),
            text: cut(text, TEXT_CHARS, shortened),
        },
        MessagePart::Reasoning { id, text } => MessagePart::Reasoning {
            id: id.clone(),
            text: cut(text, REASONING_CHARS, shortened),
        },
        MessagePart::Tool {
            id,
            call,
            is_error,
            resolved,
            output,
            diff,
            output_ref,
            output_bytes,
            diff_ref,
            diff_stats,
            ..
        } => {
            let mut call = call.clone();
            match &mut call {
                ToolCall::Exec { command } => *command = cut(command, COMMAND_CHARS, shortened),
                ToolCall::WriteFile { content, .. } => *content = None,
                ToolCall::EditFile {
                    old_string,
                    new_string,
                    ..
                } => {
                    *old_string = None;
                    *new_string = None;
                }
                ToolCall::Mcp { input, .. } | ToolCall::Unknown { input, .. } => {
                    if input.as_ref().is_some_and(|v| v.to_string().len() > COMMAND_CHARS) {
                        *input = None;
                        *shortened += 1;
                    }
                }
                _ => {}
            }
            let (diff, stats) = match diff {
                Some(d)
                    if d.new_text.chars().count() <= DIFF_SIDE_CHARS
                        && d.old_text
                            .as_ref()
                            .is_none_or(|o| o.chars().count() <= DIFF_SIDE_CHARS) =>
                {
                    (Some(d.clone()), diff_stats.clone())
                }
                Some(d) => {
                    *shortened += 1;
                    (None, Some(vec![zeron_doc::diff_stat(d)]))
                }
                None => (None, diff_stats.clone()),
            };
            MessagePart::Tool {
                id: id.clone(),
                call,
                is_error: *is_error,
                resolved: *resolved,
                output: output.as_deref().map(|text| cut(text, OUTPUT_CHARS, shortened)),
                diff,
                output_ref: output_ref.clone(),
                output_bytes: *output_bytes,
                diff_ref: diff_ref.clone(),
                diff_stats: stats,
                // A spawn chip would link a live subagent doc.
                subagent_ref: None,
                subagent_status: None,
                subagent_tail: None,
            }
        }
        MessagePart::Image { .. } | MessagePart::Error { .. } => part.clone(),
        MessagePart::Input { .. } | MessagePart::Permission { .. } => return None,
    })
}

/// Agent entries per (source thread, run), read from the source chat's doc.
type RichRuns = HashMap<(String, String), Vec<SessionMessageEntry>>;

/// Source of a chat's joined session-doc entries; `None` when unavailable.
pub(crate) type DocEntries<'a> = &'a dyn Fn(&str) -> Option<Vec<SessionMessageEntry>>;

fn rich_runs(store: &Store, items: &[Value], docs: DocEntries) -> Result<RichRuns> {
    let mut threads: Vec<&str> = items
        .iter()
        .filter_map(|item| item["sourceThreadId"].as_str())
        .collect();
    threads.sort_unstable();
    threads.dedup();
    let mut out = RichRuns::new();
    for thread in threads {
        let (Some(entries), Some(projection)) =
            (docs(thread), store.thread(&ThreadId(thread.to_owned()))?)
        else {
            continue;
        };
        // A run owns every user message recorded against it: a steer adds one,
        // and a restart-steer moves `user_message_id` onto the steer message.
        let owner: HashMap<&str, &str> = records(&projection, "message")
            .iter()
            .filter_map(|m| Some((m["id"].as_str()?, m["runId"].as_str()?)))
            .collect();
        for run in &projection.runs {
            let owned = |entry: &SessionMessageEntry| {
                entry.id == run.user_message_id.0
                    || owner.get(entry.id.as_str()).is_some_and(|id| *id == run.id.0)
            };
            let agent = run_agent_entries(&entries, owned);
            if agent.iter().any(|e| !e.parts.is_empty()) {
                out.insert((thread.to_owned(), run.id.0.clone()), agent);
            }
        }
    }
    Ok(out)
}

/// A run's agent entries from its chat doc: everything after its first user
/// message up to the next user entry that belongs to a different run (steer
/// messages the run owns do not end it).
fn run_agent_entries(
    entries: &[SessionMessageEntry],
    owned: impl Fn(&SessionMessageEntry) -> bool,
) -> Vec<SessionMessageEntry> {
    let Some(at) = entries
        .iter()
        .position(|e| e.role == MessageRole::User && owned(e))
    else {
        return Vec::new();
    };
    entries[at + 1..]
        .iter()
        .take_while(|e| e.role != MessageRole::User || owned(e))
        .filter(|e| e.role != MessageRole::User)
        .cloned()
        .collect()
}

fn rich_entries(thread: &str, doc: &[SessionMessageEntry]) -> Vec<Entry> {
    doc.iter()
        .filter_map(|entry| {
            let mut shortened = 0;
            let parts: Vec<_> = entry
                .parts
                .iter()
                .take(MAX_PARTS)
                .filter_map(|part| bound_part(part, &mut shortened))
                .collect();
            (!parts.is_empty()).then(|| Entry {
                entry: SessionMessageEntry {
                    id: format!(
                        "inherited:{}",
                        serde_json::to_string(&(thread, &entry.id)).unwrap_or_default()
                    ),
                    role: MessageRole::Assistant,
                    parts,
                    created_at: 0,
                    device_id: String::new(),
                    status: None,
                    continuation_of: None,
                },
                shortened,
            })
        })
        .collect()
}

/// Fold the flat, ordered item list into the entries the transcript renders:
/// one per user prompt, and one per run of consecutive agent work.
fn entries(items: &[Value], rich: &RichRuns) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut open: Option<Builder> = None;
    let mut emitted: HashSet<(&str, &str)> = HashSet::new();
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
        let thread = item["sourceThreadId"].as_str().unwrap_or_default();
        if let Some(doc) = run.and_then(|run| rich.get(&(thread.to_owned(), run.to_owned()))) {
            // The document has this run's real agent work: emit it once, in
            // place of the text-only items, and skip the rest of them.
            if let Some(entry) = open.take().and_then(Builder::finish) {
                out.push(entry);
            }
            if emitted.insert((thread, run.unwrap_or_default())) {
                out.extend(rich_entries(thread, doc));
            }
            continue;
        }
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
        docs: DocEntries,
    ) -> Result<InheritedHistoryPage> {
        let projection = self
            .thread(id)?
            .ok_or_else(|| Error::Invariant("The thread was not found.".into()))?;
        let items = super::transfer::inherited_items(self, &projection)?;
        let rich = rich_runs(self, &items, docs)?;
        let all = entries(&items, &rich);
        let end = match before {
            None => all.len(),
            Some(cursor) => all
                .iter()
                .position(|e| e.entry.id == cursor)
                .ok_or_else(|| {
                    Error::Invariant(format!(
                        "{}: The inherited-history cursor is no longer valid.",
                        zeron_proto::transfer::INHERITED_CURSOR_EXPIRED
                    ))
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

    fn fold(items: &[Value]) -> Vec<Entry> {
        entries(items, &RichRuns::new())
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
        let folded = fold(&sample());
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
        let folded = fold(&[
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
        let folded = fold(&[item(
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
        let folded = fold(&[item(
            "dynamic_tool",
            "d1",
            "r1",
            json!({"toolName": "view_image", "viewedImagePath": "/work/shot.png", "input": {}}),
        )]);
        assert!(matches!(&folded[0].entry.parts[0], MessagePart::Image { name, .. } if name == "shot.png"));
    }

    #[test]
    fn document_parts_drop_live_links_controls_and_oversized_diffs() {
        let mut cut = 0;
        let spawn = MessagePart::Tool {
            id: "s".into(),
            call: ToolCall::Unknown { name: "Agent".into(), input: None },
            is_error: false,
            resolved: true,
            output: None,
            diff: None,
            output_ref: None,
            output_bytes: None,
            diff_ref: None,
            diff_stats: None,
            subagent_ref: Some("sub-doc".into()),
            subagent_status: Some(zeron_doc::SubagentStatus::Running),
            subagent_tail: Some("working".into()),
        };
        let Some(MessagePart::Tool { subagent_ref, subagent_status, subagent_tail, .. }) =
            bound_part(&spawn, &mut cut)
        else {
            panic!("tool part");
        };
        assert!(subagent_ref.is_none() && subagent_status.is_none() && subagent_tail.is_none());
        let huge = MessagePart::Tool {
            id: "e".into(),
            call: ToolCall::EditFile { path: "a".into(), old_string: Some("x".into()), new_string: Some("y".into()) },
            is_error: false,
            resolved: true,
            output: Some("z".repeat(OUTPUT_CHARS + 10)),
            diff: Some(zeron_proto::ToolDiff { path: "a".into(), old_text: None, new_text: "n\n".repeat(DIFF_SIDE_CHARS) }),
            output_ref: None,
            output_bytes: None,
            diff_ref: None,
            diff_stats: None,
            subagent_ref: None,
            subagent_status: None,
            subagent_tail: None,
        };
        let Some(MessagePart::Tool { call, diff, diff_stats, output, .. }) = bound_part(&huge, &mut cut) else {
            panic!("tool part");
        };
        assert!(diff.is_none() && diff_stats.is_some_and(|s| s.len() == 1));
        assert!(matches!(call, ToolCall::EditFile { old_string: None, new_string: None, .. }));
        assert_eq!(output.unwrap().chars().count(), OUTPUT_CHARS + 1);
        assert_eq!(cut, 2);
        let input = MessagePart::Input { id: "i".into(), request_id: "r".into(), questions: vec![], resolved: false };
        assert!(bound_part(&input, &mut cut).is_none());
    }

    fn doc_entry(id: &str, role: MessageRole) -> SessionMessageEntry {
        SessionMessageEntry {
            id: id.into(),
            role,
            parts: vec![MessagePart::Text { id: "t".into(), text: id.into() }],
            created_at: 0,
            device_id: String::new(),
            status: None,
            continuation_of: None,
        }
    }

    #[test]
    fn steered_runs_keep_work_before_and_after_their_steer_messages() {
        use MessageRole::{Assistant as A, User as U};
        let doc = [
            doc_entry("u1", U),
            doc_entry("before", A),
            doc_entry("steer", U),
            doc_entry("after", A),
            doc_entry("u2", U),
            doc_entry("next", A),
        ];
        let ids = |entries: Vec<SessionMessageEntry>| -> Vec<String> {
            entries.into_iter().map(|e| e.id).collect()
        };
        // Active steer: the run keeps its first message and gains the steer.
        let active = |e: &SessionMessageEntry| matches!(e.id.as_str(), "u1" | "steer");
        assert_eq!(ids(run_agent_entries(&doc, active)), ["before", "after"]);
        // Restart-steer: `user_message_id` is now the steer message, but the
        // original message is still recorded against the run.
        let restarted = |e: &SessionMessageEntry| matches!(e.id.as_str(), "steer" | "u1");
        assert_eq!(ids(run_agent_entries(&doc, restarted)), ["before", "after"]);
        // The next run is cut at its own prompt and never inherits the steer's.
        let next = |e: &SessionMessageEntry| e.id == "u2";
        assert_eq!(ids(run_agent_entries(&doc, next)), ["next"]);
        // A user entry nobody owns ends the run rather than being swallowed.
        let strict = |e: &SessionMessageEntry| e.id == "u1";
        assert_eq!(ids(run_agent_entries(&doc, strict)), ["before"]);
    }
}
