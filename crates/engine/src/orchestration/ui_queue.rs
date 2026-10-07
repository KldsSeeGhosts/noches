//! Passive host/replica read model, never acknowledges questions or tasks.
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use zeron_proto::orchestration::ThreadId;
use zeron_proto::{ChatLifecycle, PendingQuestionUi, QueueUiEntry, QueueUiState, SettleSource};

use super::{Result, projection, queue, task};

pub(crate) fn marker(conn: &Connection, id: &ThreadId) -> Result<Value> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM orchestration_queue_lifecycle WHERE thread_id=?1",
            [&id.0],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(|raw| Ok(serde_json::from_str(&raw)?))
        .unwrap_or_else(|| Ok(json!({})))
}

pub(crate) fn loro_intents(
    conn: &Connection,
    id: &ThreadId,
) -> Result<Vec<zeron_doc::QueuedMessage>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM orchestration_queue_intents WHERE thread_id=?1",
            [&id.0],
            |row| row.get(0),
        )
        .optional()?;
    Ok(raw
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?
        .unwrap_or_default())
}

pub(crate) fn persist(
    conn: &Connection,
    id: &ThreadId,
    lifecycle: Option<&Value>,
    intents: Option<&[zeron_doc::QueuedMessage]>,
) -> Result<()> {
    if let Some(intents) = intents {
        // A cancelled run released its files in its own transaction; a lagging
        // Loro row must not re-claim them. A row without files owns no record.
        let cancelled = cancelled_messages(conn, id)?;
        for intent in intents {
            if cancelled.contains(&intent.id) {
                continue;
            }
            if intent.attachments.is_empty() {
                conn.execute(
                    "DELETE FROM orchestration_queue_attachments WHERE thread_id=?1 AND message_id=?2",
                    params![id.0, intent.id],
                )?;
                continue;
            }
            conn.execute(
                "INSERT INTO orchestration_queue_attachments VALUES(?1,?2,?3)
                 ON CONFLICT(thread_id,message_id) DO UPDATE SET paths_json=excluded.paths_json",
                params![id.0, intent.id, serde_json::to_string(&intent.attachments)?],
            )?;
        }
    }
    for (table, value) in [
        (
            "orchestration_queue_lifecycle",
            lifecycle.map(serde_json::to_value).transpose()?,
        ),
        (
            "orchestration_queue_intents",
            intents.map(serde_json::to_value).transpose()?,
        ),
    ] {
        if let Some(value) = value {
            conn.execute(
                &format!(
                    "INSERT INTO {table} VALUES(?1,?2)
                ON CONFLICT(thread_id) DO UPDATE SET payload_json=excluded.payload_json"
                ),
                params![id.0, serde_json::to_string(&value)?],
            )?;
        }
    }
    Ok(())
}

/// User-message ids of this thread's cancelled runs: queued messages that
/// will never start, whose Loro rows are only waiting to be dropped.
pub(crate) fn cancelled_messages(
    conn: &Connection,
    thread: &ThreadId,
) -> Result<std::collections::HashSet<String>> {
    let mut statement = conn.prepare(
        "SELECT json_extract(payload_json,'$.userMessageId')
         FROM orchestration_projection_runs
         WHERE thread_id=?1 AND json_extract(payload_json,'$.status')='cancelled'",
    )?;
    let rows = statement.query_map([&thread.0], |row| row.get::<_, Option<String>>(0))?;
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        ids.extend(row?);
    }
    Ok(ids)
}

pub(crate) fn attachment_paths(
    conn: &Connection,
    thread: &ThreadId,
    message: &str,
) -> Result<Vec<String>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT paths_json FROM orchestration_queue_attachments
             WHERE thread_id=?1 AND message_id=?2",
            params![thread.0, message],
            |row| row.get(0),
        )
        .optional()?;
    Ok(raw
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?
        .unwrap_or_default())
}

fn time(value: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|t| t.to_utc())
}

/// Shared presentation guard for lifecycle sync and thread list/read. Durable
/// parking metadata must never conceal active, input-needed, or failed work.
pub(crate) fn parking_blocked(p: &projection::ThreadProjection) -> bool {
    p.runs
        .iter()
        .any(|r| !super::command::run_terminal(&r.status))
        || p.runs
            .iter()
            .rev()
            .find(|r| {
                r.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Queued
                    && (r.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Cancelled
                        || r.started_at.is_some())
            })
            .is_some_and(|r| {
                r.status == zeron_proto::orchestration::OrchestrationV2RunStatus::Failed
            })
        || task::records(p, "runtime-request")
            .iter()
            .any(|r| r["status"] == "pending")
        || task::records(p, "subagent")
            .iter()
            .any(|t| !task::terminal(t["status"].as_str().unwrap_or("running")))
        || task::records(p, "provider-thread").iter().any(|t| {
            t["pendingBackgroundTasks"]
                .as_array()
                .is_some_and(|tasks| !tasks.is_empty())
        })
}

const MAX_CONTEXT_REFS: usize = 50;
const MAX_CONTEXT_FIELD_CHARS: usize = 200;

/// Bounded presentation of a message's context records. Every known kind
/// names its identifying field; unknown kinds keep only their label.
fn context_refs(message: &Value) -> Vec<zeron_proto::QueueContextRef> {
    fn field(record: &Value, key: &str) -> String {
        record[key]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(MAX_CONTEXT_FIELD_CHARS)
            .collect()
    }
    let joined = |parts: &[String]| {
        parts
            .iter()
            .filter(|part| !part.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    };
    message["context"]["records"]
        .as_array()
        .into_iter()
        .flatten()
        .take(MAX_CONTEXT_REFS)
        .filter_map(|record| {
            let kind = record["kind"].as_str().filter(|kind| !kind.is_empty())?;
            let detail = match kind {
                "image" | "file" => field(record, "name"),
                "terminal" => joined(&[
                    field(record, "terminalLabel"),
                    format!(
                        "L{}-{}",
                        record["lineStart"].as_u64().unwrap_or(0),
                        record["lineEnd"].as_u64().unwrap_or(0)
                    ),
                ]),
                "element" => joined(&[field(record, "tagName"), field(record, "selector")]),
                "preview-annotation" => {
                    let summary = field(record, "targetSummary");
                    if summary.is_empty() {
                        field(record, "pageTitle")
                    } else {
                        summary
                    }
                }
                "review-comment" => {
                    joined(&[field(record, "filePath"), field(record, "rangeLabel")])
                }
                "mention" => field(record, "path"),
                "skill" => field(record, "name"),
                "thread" => field(record, "title"),
                _ => String::new(),
            };
            Some(zeron_proto::QueueContextRef {
                context_id: field(record, "contextId"),
                kind: kind.chars().take(40).collect(),
                label: field(record, "label"),
                detail,
            })
        })
        .collect()
}

pub(crate) fn state(conn: &Connection, id: &ThreadId) -> Result<QueueUiState> {
    let Some(p) = projection::read_thread(conn, id)? else {
        return Ok(QueueUiState {
            thread_id: id.0.clone(),
            ..Default::default()
        });
    };
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM orchestration_queue_intents WHERE thread_id=?1",
            [&id.0],
            |row| row.get(0),
        )
        .optional()?;
    let intents: Vec<zeron_doc::QueuedMessage> = raw
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?
        .unwrap_or_default();
    let queue = queue::queued(&p)
        .into_iter()
        .filter_map(|run| {
            let message = task::records(&p, "message")
                .iter()
                .find(|m| m["id"] == run.user_message_id.0)?;
            let intent = intents.iter().find(|row| row.id == run.user_message_id.0);
            Some(QueueUiEntry {
                queued_run_id: run.id.0.clone(),
                message_id: run.user_message_id.0.clone(),
                text: message["text"].as_str().unwrap_or("").into(),
                attachments: message["attachments"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                // SQL-only rows keep edited uploads in the host attachment table.
                context: context_refs(message),
                attachment_paths: match intent {
                    Some(intent) => intent.attachments.clone(),
                    None => attachment_paths(conn, id, &run.user_message_id.0).unwrap_or_default(),
                },
                held: run.queue_held.as_ref().copied().unwrap_or(false)
                    || intent.is_some_and(|i| i.hold_for_turn_end),
                delivery_gate: intent
                    .and_then(|i| i.delivery_gate.as_ref())
                    .map(serde_json::to_value)
                    .transpose()
                    .ok()
                    .flatten(),
                automatic: message.get("delegatedCompletion").is_some()
                    || message.get("notification").is_some(),
                document_backed: intent.is_some(),
            })
        })
        .collect();
    let pending_questions = task::records(&p, "runtime-request")
        .iter()
        .filter_map(|request| {
            let id = request["id"].as_str()?;
            let (_, item) = queue::question(&p, id)?;
            let response_type = request["responseCapability"]["type"]
                .as_str()
                .unwrap_or("not_resumable");
            Some(PendingQuestionUi {
                request_id: id.into(),
                questions: item["questions"].as_array().cloned().unwrap_or_default(),
                response_type: response_type.into(),
                answerable: matches!(response_type, "live" | "message"),
            })
        })
        .collect();
    let thread = serde_json::to_value(&p.thread)?;
    let marker = marker(conn, id)?;
    // Durable parked fields remain in the source; presentation never conceals
    // active/blocked/error work, including post-terminal background nodes.
    let blocked = parking_blocked(&p);
    let lifecycle = ChatLifecycle {
        pinned_at: time(&thread["pinnedAt"]),
        snoozed_until: (!blocked).then(|| time(&thread["snoozedUntil"])).flatten(),
        settled_at: (!blocked).then(|| time(&thread["settledAt"])).flatten(),
        settled_by: if blocked || thread["settledAt"].is_null() {
            None
        } else {
            match marker["settledBy"].as_str() {
                Some("Auto") => Some(SettleSource::Auto),
                Some("User") => Some(SettleSource::User),
                _ => None,
            }
        },
        woke_at: time(&marker["wokeAt"]),
    };
    let promotion = queue::promotion_hint(&p);
    Ok(QueueUiState {
        schema_version: 1,
        thread_id: id.0.clone(),
        version: p.through_sequence,
        queue,
        pending_questions,
        lifecycle,
        active_run_id: task::active_run(&p).map(|run| run.id.0.clone()),
        background_run_id: super::background::settled_run(&p).map(|run| run.id.0.clone()),
        can_promote_to_steer: promotion.mode
            == Some(zeron_proto::QueuePromotionMode::ActiveSteering),
        promotion_mode: promotion.mode,
        promotion_selection: promotion.selection,
        promotion_selection_deferred: promotion.deferred,
        promotion_blocked: promotion.blocked,
    })
}

impl super::Store {
    pub fn queue_ui_state(&self, id: &ThreadId) -> Result<QueueUiState> {
        self.read(|conn| state(conn, id))
    }
}
