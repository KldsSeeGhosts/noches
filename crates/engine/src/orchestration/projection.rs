use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use zeron_proto::orchestration::{
    OrchestrationV2AppThread, OrchestrationV2ExecutionNode, OrchestrationV2Run,
    OrchestrationV2RunAttempt, ThreadId,
};

use super::event::Envelope;
use super::{Error, Result};

pub const SCHEMA_VERSION: i64 = 1;
pub(crate) const TABLES: &[&str] = &[
    "orchestration_projection_threads",
    "orchestration_projection_runs",
    "orchestration_projection_attempts",
    "orchestration_projection_nodes",
    "orchestration_projection_records",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadProjection {
    pub schema_version: i64,
    pub through_sequence: i64,
    pub thread: OrchestrationV2AppThread,
    pub runs: Vec<OrchestrationV2Run>,
    pub attempts: Vec<OrchestrationV2RunAttempt>,
    pub nodes: Vec<OrchestrationV2ExecutionNode>,
    /// Lossless auxiliary records participate in the same publication barrier.
    #[serde(default)]
    pub records: std::collections::BTreeMap<String, Vec<Value>>,
}

pub(crate) fn decode<T: DeserializeOwned>(value: &str) -> Result<T> {
    Ok(serde_json::from_str(value)?)
}

pub(crate) fn read_entity<T: DeserializeOwned>(
    conn: &Connection,
    table: &str,
    id: &str,
    thread_id: &str,
) -> Result<Option<T>> {
    // Table names are private constants, never command-supplied SQL.
    let value: Option<String> = conn
        .query_row(
            &format!("SELECT payload_json FROM {table} WHERE id=?1 AND thread_id=?2"),
            params![id, thread_id],
            |row| row.get(0),
        )
        .optional()?;
    value.as_deref().map(decode).transpose()
}

pub(crate) fn read_records(conn: &Connection, thread_id: &str, kind: &str) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT payload_json FROM orchestration_projection_records
         WHERE thread_id=?1 AND kind=?2 ORDER BY last_sequence,id",
    )?;
    stmt.query_map(params![thread_id, kind], |row| row.get::<_, String>(0))?
        .map(|row| decode(&row?))
        .collect()
}

pub(crate) fn read_thread(
    conn: &Connection,
    thread_id: &ThreadId,
) -> Result<Option<ThreadProjection>> {
    let Some(thread) = read_entity(conn, TABLES[0], &thread_id.0, &thread_id.0)? else {
        return Ok(None);
    };
    fn rows<T: DeserializeOwned>(
        conn: &Connection,
        table: &str,
        thread_id: &str,
    ) -> Result<Vec<T>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT payload_json FROM {table} WHERE thread_id=?1 ORDER BY ordinal,id"
        ))?;
        stmt.query_map([thread_id], |row| row.get::<_, String>(0))?
            .map(|row| decode(&row?))
            .collect()
    }
    let through_sequence = conn.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM orchestration_events WHERE stream_id=?1",
        [&thread_id.0],
        |row| row.get(0),
    )?;
    Ok(Some(ThreadProjection {
        schema_version: SCHEMA_VERSION,
        through_sequence,
        thread,
        runs: rows(conn, TABLES[1], &thread_id.0)?,
        attempts: rows(conn, TABLES[2], &thread_id.0)?,
        nodes: rows(conn, TABLES[3], &thread_id.0)?,
        records: {
            let mut records = std::collections::BTreeMap::new();
            for kind in [
                "subagent",
                "message",
                "turn-item",
                "context-transfer",
                "context-handoff",
                "provider-thread",
                "provider-session",
                "provider-turn",
                "runtime-request",
            ] {
                records.insert(kind.to_string(), read_records(conn, &thread_id.0, kind)?);
            }
            records
        },
    }))
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Same full-row replacement reducer is used at commit and at rebuild. Guarded
/// ownership columns remain queryable independently of the lossless JSON rows.
pub(crate) fn apply_checked(
    conn: &Connection,
    stored: &Envelope,
    check: &impl Fn(super::store::WriteBoundary) -> Result<()>,
) -> Result<()> {
    use super::store::WriteBoundary;
    let event = serde_json::to_value(&stored.event)?;
    let event_type = string(&event, "type").expect("typed event discriminator");
    let thread_id = string(&event, "threadId").expect("typed thread id");
    let mut payload = event["payload"].clone();
    if event_type == "run.background-work-cancelled" {
        let mut run = read_entity::<Value>(
            conn,
            TABLES[1],
            string(&payload, "runId").unwrap(),
            thread_id,
        )?
        .ok_or_else(|| Error::Invariant("background roster run missing".into()))?;
        run["restartCancelledBackgroundWork"] = payload["restartCancelledBackgroundWork"].clone();
        payload = run;
    }
    if matches!(event_type, "run.created" | "run.updated")
        && let Some(current) =
            read_entity::<Value>(conn, TABLES[1], string(&payload, "id").unwrap(), thread_id)?
    {
        // T3 preserveRunRecordedFields: omitted update fields must not erase
        // already recorded cohort/roster state.
        for key in ["delegatedCompletion", "restartCancelledBackgroundWork"] {
            if payload.get(key).is_none()
                && let Some(value) = current.get(key)
            {
                payload[key] = value.clone();
            }
        }
    }
    if event_type == "provider-session.detached" {
        conn.execute(
            "DELETE FROM orchestration_projection_records
             WHERE kind='provider-session' AND id=?1 AND thread_id=?2",
            params![string(&payload, "providerSessionId"), thread_id],
        )?;
        check(WriteBoundary::ProjectionRow)?;
    } else {
        let (table, kind) = if event_type.starts_with("thread.") {
            (TABLES[0], None)
        } else {
            match event_type {
                "run.created" | "run.updated" | "run.background-work-cancelled" => {
                    (TABLES[1], None)
                }
                "run-attempt.created" | "run-attempt.updated" => (TABLES[2], None),
                "node.updated" => (TABLES[3], None),
                "provider-thread.updated" => (TABLES[4], Some("provider-thread")),
                "provider-session.attached" | "provider-session.updated" => {
                    (TABLES[4], Some("provider-session"))
                }
                "provider-turn.updated" => (TABLES[4], Some("provider-turn")),
                "runtime-request.updated" => (TABLES[4], Some("runtime-request")),
                "message.updated" => (TABLES[4], Some("message")),
                "subagent.updated" => (TABLES[4], Some("subagent")),
                "turn-item.updated" => (TABLES[4], Some("turn-item")),
                "context-transfer.created" | "context-transfer.updated" => {
                    (TABLES[4], Some("context-transfer"))
                }
                "context-handoff.updated" => (TABLES[4], Some("context-handoff")),
                _ => {
                    return Err(Error::Invariant(format!(
                        "event outside kernel slice: {event_type}"
                    )));
                }
            }
        };
        let id = string(&payload, "id")
            .ok_or_else(|| Error::Invariant("projection payload missing id".into()))?;
        let existing_owner: Option<String> = if let Some(kind) = kind {
            conn.query_row(
                "SELECT thread_id FROM orchestration_projection_records WHERE id=?1 AND kind=?2 LIMIT 1",
                params![id, kind], |row| row.get(0),
            ).optional()?
        } else {
            conn.query_row(
                &format!("SELECT thread_id FROM {table} WHERE id=?1"),
                [id],
                |row| row.get(0),
            )
            .optional()?
        };
        if existing_owner
            .as_deref()
            .is_some_and(|owner| owner != thread_id)
            && kind != Some("provider-session")
        {
            return Err(Error::Invariant(
                "entity cannot move between threads".into(),
            ));
        }
        let run_id = if table == TABLES[1] {
            Some(id)
        } else {
            string(&payload, "runId")
        };
        let columns = "id,thread_id,run_id,status,provider_instance_id,provider_thread_id,
                       provider_session_id,active_attempt_id,ordinal,last_sequence,payload_json";
        let conflict = if kind.is_some() {
            "kind,id,thread_id"
        } else {
            "id"
        };
        let sql = format!(
            "INSERT INTO {table} ({} {columns}) VALUES ({} ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT({conflict}) DO UPDATE SET
                run_id=excluded.run_id,status=excluded.status,
                provider_instance_id=excluded.provider_instance_id,
                provider_thread_id=excluded.provider_thread_id,
                provider_session_id=excluded.provider_session_id,
                active_attempt_id=excluded.active_attempt_id,ordinal=excluded.ordinal,
                last_sequence=excluded.last_sequence,payload_json=excluded.payload_json",
            if kind.is_some() { "kind," } else { "" },
            kind.map(|kind| format!("'{kind}',")).unwrap_or_default()
        );
        conn.execute(
            &sql,
            params![
                id,
                thread_id,
                run_id,
                string(&payload, "status"),
                string(&payload, "providerInstanceId"),
                string(&payload, "providerThreadId"),
                string(&payload, "providerSessionId"),
                string(&payload, "activeAttemptId"),
                payload
                    .get("ordinal")
                    .or_else(|| payload.get("attemptOrdinal"))
                    .and_then(Value::as_i64),
                stored.sequence,
                serde_json::to_string(&payload)?,
            ],
        )?;
        check(WriteBoundary::ProjectionRow)?;
        // Imported real native bindings become active; queued placeholders do
        // not. This derived thread patch is replayed, not written out-of-band.
        if event_type == "provider-thread.updated"
            && string(&payload, "appThreadId") == Some(thread_id)
            && !(payload["status"] == "not_loaded"
                && payload["firstRunOrdinal"].is_null()
                && payload["nativeThreadRef"].is_null()
                && payload["providerSessionId"].is_null())
        {
            let mut thread: OrchestrationV2AppThread =
                read_entity(conn, TABLES[0], thread_id, thread_id)?
                    .ok_or_else(|| Error::Invariant("binding before thread".into()))?;
            thread.active_provider_thread_id =
                Some(zeron_proto::orchestration::ProviderThreadId(id.to_string()));
            conn.execute(
                "UPDATE orchestration_projection_threads SET payload_json=?1,last_sequence=?2 WHERE id=?3",
                params![serde_json::to_string(&thread)?, stored.sequence, thread_id],
            )?;
            check(WriteBoundary::BindingActivated)?;
        }
    }
    // T3's base reducer advances thread activity for execution events as well.
    if !event_type.starts_with("thread.") {
        let mut thread: OrchestrationV2AppThread =
            read_entity(conn, TABLES[0], thread_id, thread_id)?
                .ok_or_else(|| Error::Invariant("execution event before thread".into()))?;
        thread.updated_at = event["occurredAt"].as_str().unwrap().into();
        conn.execute(
            "UPDATE orchestration_projection_threads SET payload_json=?1,last_sequence=?2 WHERE id=?3",
            params![serde_json::to_string(&thread)?, stored.sequence, thread_id],
        )?;
        check(WriteBoundary::ActivityUpdated)?;
    }
    conn.execute(
        "UPDATE orchestration_projection_metadata SET schema_version=?1,last_sequence=?2 WHERE singleton=1",
        params![SCHEMA_VERSION, stored.sequence],
    )?;
    check(WriteBoundary::FrontierUpdated)?;
    Ok(())
}

pub(crate) fn rebuild_checked(
    conn: &Connection,
    events: &[Envelope],
    check: &impl Fn(super::store::WriteBoundary) -> Result<()>,
) -> Result<()> {
    for table in TABLES {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
        check(super::store::WriteBoundary::ProjectionCleared)?;
    }
    conn.execute(
        "UPDATE orchestration_projection_metadata SET schema_version=?1,last_sequence=0 WHERE singleton=1",
        [SCHEMA_VERSION],
    )?;
    check(super::store::WriteBoundary::ProjectionCleared)?;
    for event in events {
        if event.application_event_version != super::event::APPLICATION_EVENT_VERSION {
            return Err(Error::Invariant(
                "unsupported application event version".into(),
            ));
        }
        apply_checked(conn, event, check)?;
    }
    Ok(())
}
