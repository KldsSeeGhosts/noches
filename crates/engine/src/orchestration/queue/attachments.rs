//! Host-owned attachments of queued messages: edit planning, release when a
//! queued run is cancelled, and the guarded cleanup effect. A queued message's
//! files are claimed uploads (`message.attachments`, MCP) and committed upload
//! paths (`orchestration_queue_attachments`, desktop). Files are deleted only
//! by the durable cleanup effect, and only when nothing references them.
use std::collections::BTreeSet;

use rusqlite::Connection;
use serde_json::Value;
use zeron_proto::orchestration::*;
use zeron_proto::{QueueAttachmentEdit, queue_attachment_fingerprint};

use crate::orchestration::command::Plan;
use crate::orchestration::effects::{EffectOutcome, EffectRequest};
use crate::orchestration::projection::ThreadProjection;
use crate::orchestration::runner::RunnerBridge;
use crate::orchestration::{Error, Result, task, ui_queue};

/// T3's per-message attachment limit (`HostLaunchService::claim`).
pub(crate) const MAX_ATTACHMENTS: usize = 8;

fn refuse(message: impl Into<String>) -> Error {
    Error::Invariant(message.into())
}

fn claim_ids(message: &Value) -> Vec<String> {
    message["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|attachment| attachment["id"].as_str().map(str::to_owned))
        .collect()
}

/// Claimed ids in `ids` that no *other* message in the thread references: a
/// claim can be shared by messages (`send_attachments` reuses thread-owned ids).
fn unshared_claims(p: &ThreadProjection, message_id: &str, ids: &[String]) -> Vec<String> {
    let others: BTreeSet<String> = task::records(p, "message")
        .iter()
        .filter(|message| message["id"] != message_id)
        .flat_map(claim_ids)
        .collect();
    ids.iter()
        .filter(|id| !others.contains(*id))
        .cloned()
        .collect()
}

pub(super) fn cleanup_effects(plan: &mut Plan, paths: Vec<String>, claims: Vec<String>) {
    if !paths.is_empty() {
        plan.effects
            .push(EffectRequest::QueuedAttachmentCleanup { paths });
    }
    if !claims.is_empty() {
        plan.effects.push(EffectRequest::AttachmentCleanup {
            attachment_ids: claims,
        });
    }
}

/// A queued run that will never start gives up its files. Callers pass only
/// runs they are cancelling; promotion and delivery keep theirs.
pub(super) fn release_run(
    conn: &Connection,
    plan: &mut Plan,
    p: &ThreadProjection,
    run: &OrchestrationV2Run,
) -> Result<()> {
    let message_id = run.user_message_id.0.clone();
    let paths = ui_queue::attachment_paths(conn, &p.thread.id, &message_id)?;
    let claims = task::records(p, "message")
        .iter()
        .find(|message| message["id"] == message_id.as_str())
        .map(|message| unshared_claims(p, &message_id, &claim_ids(message)))
        .unwrap_or_default();
    if !paths.is_empty() {
        plan.queue_attachments.push((message_id, Vec::new()));
    }
    cleanup_effects(plan, paths, claims);
    Ok(())
}

/// Apply an attachment edit to the planned message update. Returns whether the
/// message record changed (callers mirror it onto the user turn item).
pub(super) fn plan_edit(
    conn: &Connection,
    plan: &mut Plan,
    p: &ThreadProjection,
    message: &mut Value,
    edit: &Value,
) -> Result<bool> {
    let edit: QueueAttachmentEdit = serde_json::from_value(edit.clone())
        .map_err(|_| refuse("The attachment edit is malformed."))?;
    let message_id = message["id"].as_str().unwrap_or("").to_owned();
    let claims = message["attachments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let current = ui_queue::attachment_paths(conn, &p.thread.id, &message_id)?;
    if queue_attachment_fingerprint(&claims, &current) != edit.expected {
        return Err(refuse(
            "This queued message's attachments changed; your edit was not applied.",
        ));
    }
    let unique: BTreeSet<&String> = edit.paths.iter().collect();
    if unique.len() != edit.paths.len() || edit.paths.iter().any(|path| path.trim().is_empty()) {
        return Err(refuse("Attachment paths must be distinct and nonempty."));
    }
    let known: BTreeSet<String> = claim_ids(message).into_iter().collect();
    if let Some(unknown) = edit.remove_ids.iter().find(|id| !known.contains(*id)) {
        return Err(refuse(format!(
            "Attachment {unknown} does not belong to this queued message."
        )));
    }
    let kept: Vec<Value> = claims
        .iter()
        .filter(|claim| {
            !claim["id"]
                .as_str()
                .is_some_and(|id| edit.remove_ids.iter().any(|removed| removed == id))
        })
        .cloned()
        .collect();
    if kept.len() + edit.paths.len() > MAX_ATTACHMENTS {
        return Err(refuse(format!(
            "A message can carry up to {MAX_ATTACHMENTS} attachments."
        )));
    }
    let removed_paths: Vec<String> = current
        .iter()
        .filter(|path| !edit.paths.contains(path))
        .cloned()
        .collect();
    let removed_claims = unshared_claims(p, &message_id, &edit.remove_ids);
    let changed = kept.len() != claims.len() || edit.paths != current;
    if !changed {
        return Ok(false);
    }
    if kept.len() != claims.len() {
        message["attachments"] = Value::Array(kept);
        // Context records bound to a dropped attachment would dangle.
        if let Some(records) = message
            .get_mut("context")
            .and_then(|context| context.get_mut("records"))
            .and_then(Value::as_array_mut)
        {
            records.retain(|record| {
                !record["attachmentId"]
                    .as_str()
                    .is_some_and(|id| edit.remove_ids.iter().any(|removed| removed == id))
            });
        }
    }
    if edit.paths != current {
        plan.queue_attachments.push((message_id, edit.paths));
    }
    cleanup_effects(plan, removed_paths, removed_claims);
    Ok(true)
}

/// Upload files the host can safely drop: under the uploads root, and not
/// named by any queue row (another message, another thread, a live intent).
fn unreferenced(conn: &Connection, path: &str) -> Result<bool> {
    let mut statement = conn.prepare(
        "SELECT paths_json FROM orchestration_queue_attachments
         UNION ALL SELECT payload_json FROM orchestration_queue_intents",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let needle = serde_json::to_string(path)?;
    for raw in rows {
        if raw?.contains(&needle) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) async fn execute_cleanup(
    bridge: &RunnerBridge,
    paths: &[String],
) -> Result<EffectOutcome> {
    // Without an uploads store nothing here was ever a committed upload.
    if let Some(uploads) = bridge.doc_host.uploads() {
        remove_unreferenced(&bridge.kernel.store, uploads, paths)?;
    }
    Ok(EffectOutcome::Succeeded)
}

/// Delete each owned upload no queue row names. Idempotent: a replayed
/// cleanup finds the files gone, and a path adopted since planning survives.
pub(crate) fn remove_unreferenced(
    store: &crate::orchestration::Store,
    uploads: &crate::uploads::Uploads,
    paths: &[String],
) -> Result<()> {
    for path in paths {
        if !uploads.owns(std::path::Path::new(path))
            || !store.read(|conn| unreferenced(conn, path))?
        {
            continue;
        }
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::Invariant(error.to_string())),
        }
    }
    Ok(())
}

/// Uploads the host accepts as new queue attachments: committed by this host,
/// present, and not already another row's file.
pub(crate) fn validate_new_paths(
    conn: &Connection,
    uploads: &crate::uploads::Uploads,
    thread: &ThreadId,
    message_id: &str,
    requested: &[String],
) -> Result<std::result::Result<(), String>> {
    let current = ui_queue::attachment_paths(conn, thread, message_id)?;
    for path in requested.iter().filter(|path| !current.contains(path)) {
        let local = std::path::Path::new(path);
        if !uploads.owns(local) || !local.is_file() {
            return Ok(Err(
                "A new attachment was not found on the chat host; upload it again.".into(),
            ));
        }
        if !unreferenced(conn, path)? {
            return Ok(Err(
                "That attachment already belongs to another queued message.".into(),
            ));
        }
    }
    Ok(Ok(()))
}
