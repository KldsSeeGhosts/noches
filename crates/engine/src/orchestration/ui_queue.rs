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
        for intent in intents {
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
                attachment_paths: intent.map(|i| i.attachments.clone()).unwrap_or_default(),
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
    Ok(QueueUiState {
        schema_version: 1,
        thread_id: id.0.clone(),
        version: p.through_sequence,
        queue,
        pending_questions,
        lifecycle,
        active_run_id: task::active_run(&p).map(|run| run.id.0.clone()),
        can_promote_to_steer: queue::can_promote_to_steer(&p),
    })
}

impl super::Store {
    pub fn queue_ui_state(&self, id: &ThreadId) -> Result<QueueUiState> {
        self.read(|conn| state(conn, id))
    }
}
