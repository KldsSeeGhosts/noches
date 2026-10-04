use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;

use super::wire;
use crate::orchestration::projection::ThreadProjection;
use crate::orchestration::task::{active_run, records};
use crate::orchestration::{Error, Result, Store, projection};

/// `result` is the Unicode-safe desktop projection. MCP uses `wire_value` so a
/// JS slice ending inside an emoji retains the exact escaped surrogate.
#[derive(Debug, Clone)]
pub struct ThreadReadPage {
    pub result: T3ThreadReadResult,
    pub text_units: BTreeMap<usize, Vec<u16>>,
}

impl ThreadReadPage {
    pub fn wire_value(&self) -> Value {
        let mut value = serde_json::to_value(&self.result).expect("thread read contract");
        for (index, units) in &self.text_units {
            value["items"][*index]["text"] = wire::text_value(units);
        }
        value
    }
}

#[derive(Clone)]
pub(crate) struct Row {
    pub position: usize,
    pub source: ThreadId,
    pub id: String,
    pub visibility: &'static str,
    pub item: Value,
}

// Timeline positions count all visible items, not just the selected view.
pub(crate) fn visible(store: &Store, target: &ThreadProjection) -> Result<Vec<Row>> {
    fn local(target: &ThreadProjection) -> Vec<Row> {
        let mut items = records(target, "turn-item").to_vec();
        items.sort_by(|a, b| {
            a["ordinal"]
                .as_i64()
                .cmp(&b["ordinal"].as_i64())
                .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
        });
        items
            .into_iter()
            .map(|item| Row {
                position: 0,
                source: target.thread.id.clone(),
                id: item["id"].as_str().unwrap_or_default().into(),
                visibility: "local",
                item,
            })
            .collect()
    }
    fn superseded(target: &ThreadProjection, item: &Value) -> bool {
        item["type"] == "run_interrupt_result"
            && item["runId"].is_string()
            && item["nodeId"].is_string()
            && target.attempts.iter().any(|a| {
                a.status == OrchestrationV2RunAttemptStatus::Superseded
                    && item["runId"] == a.run_id.0
                    && item["nodeId"] == a.root_node_id.0
            })
            && !records(target, "turn-item")
                .iter()
                .any(|i| i["type"] == "run_interrupt_request" && i["runId"] == item["runId"])
    }
    fn visit(
        store: &Store,
        target: &ThreadProjection,
        seen: &mut BTreeSet<String>,
    ) -> Result<Vec<Row>> {
        let local_rows = local(target);
        let local_visible: Vec<_> = local_rows
            .iter()
            .filter(|row| {
                // Ordinary intake may already have written the user doc entry
                // when first admission snapshots history. Its real run item
                // supersedes the imported, non-executable transcript row.
                if row.item["runId"].is_null()
                    && row.item["messageId"].is_string()
                    && local_rows.iter().any(|other| {
                        other.item["runId"].is_string()
                            && other.item["messageId"] == row.item["messageId"]
                    })
                {
                    return false;
                }
                let status = target
                    .runs
                    .iter()
                    .find(|r| row.item["runId"] == r.id.0)
                    .map(|r| &r.status);
                status != Some(&OrchestrationV2RunStatus::RolledBack)
                    && !(status == Some(&OrchestrationV2RunStatus::Cancelled)
                        && row.item["type"] == "user_message"
                        && row.item["inputIntent"] == "queued_turn")
                    && !superseded(target, &row.item)
            })
            .cloned()
            .collect();
        if !seen.insert(target.thread.id.0.clone()) {
            return Ok(local_visible);
        }
        let mut rows = vec![];
        if let Some(OrchestrationV2AppThreadForkedFrom::Run(fork)) =
            target.thread.forked_from.as_ref()
            && !seen.contains(&fork.thread_id.0)
        {
            let parent = store
                .thread(&fork.thread_id)?
                .ok_or_else(|| Error::Invariant("Fork source thread missing.".into()))?;
            if let Some(run) = parent.runs.iter().find(|r| r.id == fork.run_id) {
                let mut inherited: Vec<_> = visit(store, &parent, seen)?
                    .into_iter()
                    .filter(|r| r.item["threadId"] != fork.thread_id.0 || r.item["type"] == "fork")
                    .collect();
                inherited.extend(local(&parent).into_iter().filter(|r| {
                    !superseded(&parent, &r.item)
                        && (parent
                            .runs
                            .iter()
                            .find(|s| r.item["runId"] == s.id.0)
                            .is_some_and(|s| s.ordinal <= run.ordinal)
                            || (r.item["runId"].is_null()
                                && json!(parent.thread.history_origin) == "v1_import"))
                }));
                for mut row in inherited {
                    row.visibility = "inherited";
                    rows.push(row);
                }
            }
            let id = format!("turn-item:fork:{}", target.thread.id);
            let item = json!({"id":id,"threadId":target.thread.id,"runId":null,"nodeId":null,
                "providerTurnId":null,"nativeItemRef":null,"parentItemId":null,"ordinal":0,"status":"completed",
                "title":"Forked from conversation","startedAt":null,"completedAt":target.thread.created_at,
                "updatedAt":target.thread.created_at,"type":"fork","source":{"type":"run","threadId":fork.thread_id,"runId":fork.run_id},
                "targetThreadId":target.thread.id});
            rows.push(Row {
                position: 0,
                source: fork.thread_id.clone(),
                id,
                visibility: "synthetic",
                item,
            });
        }
        rows.extend(local_visible);
        seen.remove(&target.thread.id.0);
        Ok(rows)
    }
    let mut rows = visit(store, target, &mut BTreeSet::new())?;
    for (position, row) in rows.iter_mut().enumerate() {
        row.position = position;
    }
    Ok(rows)
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap()
}

fn pretty_fields(item: &Value, fields: &[&str]) -> String {
    struct Fields<'a>(&'a Value, &'a [&'a str]);
    impl serde::Serialize for Fields<'_> {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            use serde::ser::SerializeMap;
            let mut object = serializer.serialize_map(None)?;
            for key in self.1 {
                // JS JSON.stringify omits undefined, but retains explicit null.
                if let Some(value) = self.0.get(*key) {
                    object.serialize_entry(key, value)?;
                }
            }
            object.end()
        }
    }
    serde_json::to_string_pretty(&Fields(item, fields)).unwrap()
}

pub(crate) fn text(item: &Value) -> Option<String> {
    let string = |key: &str| item[key].as_str().map(str::to_owned);
    let join = |parts: Vec<Option<String>>| {
        Some(parts.into_iter().flatten().collect::<Vec<_>>().join("\n"))
    };
    match item["type"].as_str()? {
        "user_message" | "assistant_message" | "reasoning" => string("text"),
        "notification" => join(vec![string("summary"), string("detail")]),
        "proposed_plan" => string("markdown"),
        "todo_list" => {
            let mut lines = vec![string("explanation")];
            lines.extend(item["steps"].as_array().into_iter().flatten().map(|s| {
                Some(format!(
                    "[{}] {}",
                    s["status"].as_str().unwrap_or_default(),
                    s["text"].as_str().unwrap_or_default()
                ))
            }));
            join(lines)
        }
        "user_input_request" => Some(pretty(&item["questions"])),
        "file_change" => join(vec![
            string("fileName"),
            (item.get("additions").is_some() || item.get("deletions").is_some()).then(|| {
                format!(
                    "+{} -{}",
                    item["additions"].as_i64().unwrap_or(0),
                    item["deletions"].as_i64().unwrap_or(0)
                )
            }),
            string("diffStr").or_else(|| string("newStr")),
        ]),
        "command_execution" => join(vec![
            string("input").map(|s| format!("$ {s}")),
            string("output"),
        ]),
        "file_search" => Some(pretty_fields(item, &["pattern", "results"])),
        "web_search" => Some(pretty_fields(item, &["patterns", "results"])),
        "approval_request" => string("prompt").or_else(|| string("requestKind")),
        "checkpoint" => Some(pretty(&item["files"])),
        "run_interrupt_request" | "run_interrupt_result" | "system_notice" => string("message"),
        "error" => item["failure"]["message"].as_str().map(str::to_owned),
        "compaction" => string("summary"),
        "handoff" => string("summary").or_else(|| {
            Some(format!(
                "{} handoff to {}",
                item["strategy"].as_str().unwrap_or_default(),
                item["toProviderInstanceId"].as_str().unwrap_or_default()
            ))
        }),
        "fork" => Some(format!(
            "Forked to thread {}.",
            item["targetThreadId"].as_str().unwrap_or_default()
        )),
        "thread_created" => Some(format!(
            "Created thread {} with {} ({}).",
            item["targetThreadId"].as_str().unwrap_or_default(),
            item["targetProviderInstanceId"]
                .as_str()
                .unwrap_or_default(),
            item["targetModel"].as_str().unwrap_or_default()
        )),
        "subagent" => string("result")
            .or_else(|| string("progress"))
            .or_else(|| string("prompt")),
        "dynamic_tool" => Some(pretty_fields(item, &["toolName", "input", "output"])),
        _ => None,
    }
}

fn settlement(target: &ThreadProjection) -> (bool, Value) {
    let thread = serde_json::to_value(&target.thread).unwrap();
    let settled = thread["settledOverride"] == "settled"
        && !crate::orchestration::ui_queue::parking_blocked(target);
    (
        settled,
        if settled {
            thread["settledAt"].clone()
        } else {
            Value::Null
        },
    )
}

pub(crate) fn summary(target: &ThreadProjection, item_count: usize) -> Value {
    let (settled, settled_at) = settlement(target);
    let latest = target.runs.iter().max_by_key(|r| r.ordinal);
    let active = active_run(target);
    json!({
        "threadId":target.thread.id,"title":target.thread.title,
        "createdBy":target.thread.created_by,"creationSource":target.thread.creation_source,
        "status":active.or(latest).map(|r| json!(r.status)).unwrap_or(json!("idle")),
        "latestRunId":latest.map(|r| &r.id),"providerInstanceId":target.thread.model_selection.instance_id,
        "model":target.thread.model_selection.model,"runtimeMode":target.thread.runtime_mode,
        "interactionMode":target.thread.interaction_mode,"linkedPullRequest":target.thread.linked_pull_request,
        "settled":settled,"settledAt":settled_at,"parentThreadId":target.thread.lineage.parent_thread_id,
        "relationshipToParent":target.thread.lineage.relationship_to_parent,"itemCount":item_count,
        "createdAt":target.thread.created_at,"updatedAt":target.thread.updated_at,
    })
}

pub(crate) fn page(
    store: &Store,
    target: &ThreadProjection,
    input: &T3ThreadReadInput,
) -> Result<(ThreadReadPage, Vec<Row>)> {
    let all = visible(store, target)?;
    let matching: Vec<_> = all
        .iter()
        .filter(|row| {
            if let Some(item) = input.item_id.as_ref() {
                return row.id == *item;
            }
            row.position as i64 > input.after_position.as_ref().copied().unwrap_or(-1)
                && (input
                    .view
                    .as_ref()
                    .is_some_and(|v| *v == T3ThreadReadInputView::Activity)
                    || matches!(
                        row.item["type"].as_str(),
                        Some("user_message" | "assistant_message" | "proposed_plan")
                    ))
        })
        .cloned()
        .collect();
    let limit = input.limit.as_ref().copied().unwrap_or(50) as usize;
    let selected: Vec<_> = matching.iter().take(limit).cloned().collect();
    let max = input.max_chars_per_item.as_ref().copied().unwrap_or(20_000) as usize;
    let offset = if input.item_id.as_ref().is_some() {
        input.text_offset.as_ref().copied().unwrap_or(0) as usize
    } else {
        0
    };
    let mut units = BTreeMap::new();
    let mut items = vec![];
    for (index, row) in selected.iter().enumerate() {
        let (text, truncated) = if let Some(text) = text(&row.item) {
            let (text, raw, truncated) = wire::slice(&text, offset, max);
            units.insert(index, raw);
            (json!(text), truncated)
        } else {
            (Value::Null, false)
        };
        let message_id = matches!(
            row.item["type"].as_str(),
            Some("user_message" | "assistant_message")
        )
        .then(|| row.item["messageId"].clone())
        .unwrap_or(Value::Null);
        let source = store.thread(&row.source)?;
        let message = source
            .as_ref()
            .and_then(|p| records(p, "message").iter().find(|m| m["id"] == message_id));
        items.push(json!({"position":row.position,"visibility":row.visibility,"sourceThreadId":row.source,
            "itemId":row.id,"runId":row.item["runId"],"messageId":message_id,
            "createdBy":message.map(|m| &m["createdBy"]),"creationSource":message.map(|m| &m["creationSource"]),
            "type":row.item["type"],"status":row.item["status"],"title":row.item["title"],
            "text":text,"textTruncated":truncated,"nextTextOffset":truncated.then_some(offset.saturating_add(max)),
            "updatedAt":row.item["updatedAt"]}));
    }
    let mut detail = summary(target, all.len());
    detail["projectId"] = json!(target.thread.project_id);
    detail["activeRunId"] = json!(active_run(target).map(|r| &r.id));
    detail["titleRegeneration"] = json!(target.thread.title_regeneration);
    detail["branch"] = json!(target.thread.branch);
    detail["worktreePath"] = json!(target.thread.worktree_path);
    detail["runCount"] = json!(target.runs.len());
    detail["pendingRequestCount"] = json!(
        records(target, "runtime-request")
            .iter()
            .filter(|r| r["status"] == "pending")
            .count()
    );
    detail["archived"] = json!(target.thread.archived_at.is_some());
    let mut runs = target.runs.clone();
    runs.sort_by_key(|r| std::cmp::Reverse(r.ordinal));
    let recent: Vec<_> = runs
        .iter()
        .take(input.run_limit.as_ref().copied().unwrap_or(10) as usize)
        .map(|r| {
            json!({"runId":r.id,"ordinal":r.ordinal,"status":r.status,
            "providerInstanceId":r.model_selection.instance_id,"model":r.model_selection.model,
            "requestedAt":r.requested_at,"startedAt":r.started_at,"completedAt":r.completed_at})
        })
        .collect();
    let result = serde_json::from_value(json!({"thread":detail,"recentRuns":recent,"items":items,
        "nextPosition":selected.last().map(|r| r.position),"hasMore":matching.len() > selected.len()}))?;
    Ok((
        ThreadReadPage {
            result,
            text_units: units,
        },
        selected,
    ))
}

impl Store {
    pub fn thread_summaries(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<OrchestratorMcpThreadListItem>> {
        self.admit_project(project)?;
        let threads: Vec<ThreadProjection> = self.read(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM orchestration_projection_threads")?;
            let ids = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.into_iter()
                .map(|id| {
                    projection::read_thread(conn, &ThreadId(id))?
                        .ok_or_else(|| Error::Invariant("Thread disappeared.".into()))
                })
                .collect()
        })?;
        let mut threads: Vec<_> = threads
            .into_iter()
            .filter(|t| t.thread.project_id == *project && t.thread.deleted_at.is_none())
            .collect();
        threads.sort_by(|a, b| {
            b.thread
                .updated_at
                .cmp(&a.thread.updated_at)
                .then_with(|| b.thread.id.0.cmp(&a.thread.id.0))
        });
        threads
            .iter()
            .map(|t| Ok(serde_json::from_value(summary(t, visible(self, t)?.len()))?))
            .collect()
    }
}
