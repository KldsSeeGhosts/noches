//! Fork/merge-back planning under the kernel transaction. Provider effects are
//! deferred until a target run starts, just as in T3 (fork creates an idle thread).
pub mod context;
pub mod delivery;
pub mod mcp;
mod retry;
#[cfg(test)]
pub(crate) mod tests;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

use super::command::{Command, Plan};
use super::event::{encode_component, iso};
use super::projection::{self, ThreadProjection};
use super::{Error, Kernel, Operation, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourcePoint {
    LatestStable,
    Run {
        #[serde(rename = "runId")]
        run_id: RunId,
    },
    Checkpoint {
        #[serde(rename = "checkpointId")]
        checkpoint_id: CheckpointId,
    },
}

#[derive(Debug, Clone)]
pub enum TransferOperation {
    /// Desktop user authority does not impersonate an active provider session.
    User(Box<TransferOperation>),
    Fork {
        target: ThreadId,
        source: SourcePoint,
        title: Option<String>,
    },
    MergeBack {
        target: ThreadId,
        source: SourcePoint,
    },
    Update {
        transfer: Box<Value>,
        handoff: Option<Box<Value>>,
    },
    CreateHandoff {
        transfer: Box<Value>,
        handoff: Box<Value>,
    },
    Checkpoint {
        record: Box<zeron_proto::transfer::FileCheckpoint>,
    },
}

impl TransferOperation {
    pub fn command_type(&self) -> &'static str {
        match self {
            Self::User(operation) => operation.command_type(),
            Self::Fork { .. } => "thread.fork",
            Self::MergeBack { .. } => "thread.merge_back",
            Self::Update { .. } => "kernel.transfer.update",
            Self::CreateHandoff { .. } => "kernel.transfer.handoff",
            Self::Checkpoint { .. } => "kernel.checkpoint.capture",
        }
    }
    pub fn lock_threads(&self) -> Vec<ThreadId> {
        match self {
            Self::User(operation) => operation.lock_threads(),
            Self::Fork { target, .. } | Self::MergeBack { target, .. } => vec![target.clone()],
            Self::Update { transfer, .. } | Self::CreateHandoff { transfer, .. } => {
                ["sourceThreadId", "targetThreadId"]
                    .into_iter()
                    .filter_map(|key| transfer[key].as_str().map(|s| ThreadId(s.into())))
                    .collect()
            }
            Self::Checkpoint { .. } => vec![],
        }
    }
}

pub fn forkable(status: &OrchestrationV2RunStatus) -> bool {
    use OrchestrationV2RunStatus::*;
    matches!(
        status,
        Completed | Waiting | Failed | Interrupted | Cancelled
    )
}

pub(crate) fn source_run<'a>(
    conn: &Connection,
    projection: &'a ThreadProjection,
    point: &SourcePoint,
) -> Result<Option<&'a OrchestrationV2Run>> {
    Ok(match point {
        SourcePoint::LatestStable => projection
            .runs
            .iter()
            .filter(|r| {
                r.status == OrchestrationV2RunStatus::Completed && r.checkpoint_id.is_some()
            })
            .max_by_key(|r| r.ordinal),
        SourcePoint::Run { run_id } => projection.runs.iter().find(|r| &r.id == run_id),
        SourcePoint::Checkpoint { checkpoint_id } => {
            let checkpoint = super::checkpoint::record(conn, checkpoint_id)?;
            let run_id = checkpoint
                .filter(|c| c.checkpoint.thread_id == projection.thread.id)
                .and_then(|c| c.checkpoint.run_id)
                .or_else(|| {
                    super::task::records(projection, "checkpoint")
                        .iter()
                        .find(|c| c["id"] == checkpoint_id.0)
                        .and_then(|c| c["runId"].as_str().map(|id| RunId(id.into())))
                });
            run_id.and_then(|id| projection.runs.iter().find(|r| r.id == id))
        }
    })
}

pub(crate) fn provider_for_run(
    projection: &ThreadProjection,
    run: &OrchestrationV2Run,
) -> Option<Value> {
    super::task::records(projection, "provider-thread")
        .iter()
        .find(|p| p["id"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str()))
        .cloned()
}

pub(crate) fn canonical_point(projection: &ThreadProjection, run: &OrchestrationV2Run) -> Value {
    let mut point = json!({"threadId":projection.thread.id,"runId":run.id});
    if let Some(id) = &run.checkpoint_id {
        point["checkpointId"] = json!(id);
    }
    if let Some(provider) = provider_for_run(projection, run)
        && !provider["nativeThreadRef"].is_null()
    {
        point["providerThreadRef"] = provider["nativeThreadRef"].clone();
    }
    let turn = super::task::records(projection, "provider-turn")
        .iter()
        .find(|turn| {
            run.active_attempt_id
                .as_ref()
                .is_some_and(|id| turn["runAttemptId"] == id.0)
        })
        .or_else(|| {
            let attempt = projection
                .attempts
                .iter()
                .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())?;
            super::task::records(projection, "provider-turn")
                .iter()
                .find(|turn| {
                    turn["id"].as_str() == attempt.provider_turn_id.as_ref().map(|id| id.0.as_str())
                })
        });
    if let Some(turn) = turn
        && !turn["nativeTurnRef"].is_null()
    {
        point["providerTurnRef"] = turn["nativeTurnRef"].clone();
    }
    point
}

pub(crate) fn transfers(conn: &Connection, thread: &ThreadId) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT payload_json FROM orchestration_projection_records WHERE kind='context-transfer'
         AND (json_extract(payload_json,'$.sourceThreadId')=?1 OR json_extract(payload_json,'$.targetThreadId')=?1)
         ORDER BY last_sequence,id")?;
    stmt.query_map([&thread.0], |row| row.get::<_, String>(0))?
        .map(|v| Ok(serde_json::from_str(&v?)?))
        .collect()
}

fn require_thread(conn: &Connection, id: &ThreadId) -> Result<ThreadProjection> {
    projection::read_thread(conn, id)?
        .filter(|p| p.thread.deleted_at.is_none())
        .ok_or_else(|| Error::Invariant(format!("Thread {id} was not found.")))
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    operation: &TransferOperation,
    now: i64,
) -> Result<Plan> {
    let (operation, user) = match operation {
        TransferOperation::User(operation) => (operation.as_ref(), true),
        operation => (operation, false),
    };
    let mut plan = Plan::default();
    let source = require_thread(conn, &command.thread_id)?;
    match operation {
        TransferOperation::Fork {
            target,
            source: point,
            ..
        }
        | TransferOperation::MergeBack {
            target,
            source: point,
        } => {
            let merge = matches!(operation, TransferOperation::MergeBack { .. });
            let target_projection = if merge {
                Some(require_thread(conn, target)?)
            } else {
                if projection::read_thread(conn, target)?.is_some() {
                    return Err(Error::Invariant(format!("Thread {target} already exists.")));
                }
                None
            };
            if merge
                && (source.thread.lineage.relationship_to_parent.as_ref()
                    != Some(&OrchestrationV2AppThreadLineageRelationshipToParent::Fork)
                    || source.thread.lineage.parent_thread_id.as_ref() != Some(target))
            {
                return Err(Error::Invariant(format!(
                    "Thread {} is not a fork of {target}.",
                    command.thread_id
                )));
            }
            if let Some(target) = &target_projection
                && target.thread.project_id != source.thread.project_id
            {
                return Err(Error::Invariant(
                    "The thread was not found in the calling project.".into(),
                ));
            }
            let label = if merge { "merge-back" } else { "fork" };
            let point_type = serde_json::to_value(point)?["type"]
                .as_str()
                .unwrap()
                .to_owned();
            let run = source_run(conn, &source, point)?.ok_or_else(|| {
                Error::Invariant(format!(
                    "No stable source run was found for {label} source {point_type}."
                ))
            })?;
            let status = serde_json::to_value(&run.status)?
                .as_str()
                .unwrap()
                .to_owned();
            if merge
                && !matches!(
                    run.status,
                    OrchestrationV2RunStatus::Completed | OrchestrationV2RunStatus::Waiting
                )
            {
                return Err(Error::Invariant(format!(
                    "Merge-back source run {} is {status}; only provider-finished runs are supported.",
                    run.id
                )));
            }
            if !merge && !forkable(&run.status) {
                return Err(Error::Invariant(format!(
                    "Fork source run {} is {status}; in-progress and rolled-back runs cannot be forked.",
                    run.id
                )));
            }
            let source_transfers = transfers(conn, &source.thread.id)?;
            let base = if merge {
                source_transfers
                    .iter()
                    .rev()
                    .find(|t| {
                        t["type"] == "fork"
                            && t["sourceThreadId"] == target.0
                            && t["targetThreadId"] == source.thread.id.0
                    })
                    .map(|t| t["sourcePoint"].clone())
                    .ok_or_else(|| {
                        Error::Invariant(format!(
                            "No fork transfer exists between {target} and {}.",
                            source.thread.id
                        ))
                    })?
            } else {
                Value::Null
            };
            let provider = provider_for_run(&source, run);
            let transfer_id = format!("transfer:{}", encode_component(&command.id.0));
            let transfer = json!({
                "id":transfer_id,"type":if merge {"merge_back"} else {"fork"},
                "sourceThreadId":source.thread.id,"targetThreadId":target,
                "sourcePoint":canonical_point(&source,run),"basePoint":base,
                "sourceProviderInstanceId":run.provider_instance_id,
                "targetProviderInstanceId":target_projection.as_ref().map(|p| &p.thread.model_selection.instance_id),
                "targetRunId":null,"status":"pending","resolution":null,"createdBy":if user {"user"} else {"agent"},
                "error":if merge {if provider.is_none() {Some("Source merge-back run has no provider thread.")} else {None}}
                    else if provider.as_ref().is_some_and(|p| p["nativeThreadRef"]["strength"] == "strong") {None}
                    else {Some("Source provider thread does not expose a strong native thread ref.")},
                "createdAt":iso(now)?,"updatedAt":iso(now)?,"consumedAt":null
            });
            if !merge {
                let mut thread = serde_json::to_value(&source.thread)?;
                thread["id"] = json!(target);
                thread["title"] = json!(
                    title_for(operation).unwrap_or_else(|| format!("{} fork", source.thread.title))
                );
                thread["createdBy"] = json!(if user { "user" } else { "agent" });
                thread["creationSource"] = json!(if user { "web" } else { "mcp" });
                thread["activeProviderThreadId"] = Value::Null;
                thread["lineage"] = json!({"parentThreadId":source.thread.id,"relationshipToParent":"fork","rootThreadId":source.thread.lineage.root_thread_id});
                thread["forkedFrom"] =
                    json!({"type":"run","threadId":source.thread.id,"runId":run.id});
                thread["createdAt"] = json!(iso(now)?);
                thread["updatedAt"] = json!(iso(now)?);
                for key in [
                    "archivedAt",
                    "settledOverride",
                    "settledAt",
                    "snoozedUntil",
                    "snoozedAt",
                    "lastVisitedAt",
                    "deletedAt",
                ] {
                    thread[key] = Value::Null;
                }
                plan.emit_on(command, target, "thread.created", &thread, now)?;
            } else {
                for mut previous in transfers(conn, target)?.into_iter().filter(|t| {
                    t["type"] == "merge_back"
                        && t["status"] == "pending"
                        && t["sourceThreadId"] == source.thread.id.0
                        && t["targetThreadId"] == target.0
                }) {
                    previous["status"] = json!("superseded");
                    previous["error"] =
                        json!(format!("Superseded by merge-back transfer {transfer_id}."));
                    previous["updatedAt"] = json!(iso(now)?);
                    plan.emit_on(command, target, "context-transfer.updated", &previous, now)?;
                }
            }
            plan.emit_on(command, target, "context-transfer.created", &transfer, now)?;
        }
        TransferOperation::Update { transfer, handoff } => {
            let target: ThreadId = serde_json::from_value(transfer["targetThreadId"].clone())?;
            let current = transfers(conn, &target)?
                .into_iter()
                .find(|t| t["id"] == transfer["id"])
                .ok_or_else(|| Error::Invariant("Transfer missing.".into()))?;
            if matches!(
                current["status"].as_str(),
                Some("consumed" | "superseded" | "failed")
            ) {
                if **transfer != current {
                    return Err(Error::Invariant("Transfer is already terminal.".into()));
                }
                if let Some(handoff) = handoff {
                    delivery::persist_receipt(conn, handoff)?;
                    plan.emit_on(command, &target, "context-handoff.updated", handoff, now)?;
                    handoff_metadata(
                        &require_thread(conn, &target)?,
                        command,
                        &mut plan,
                        handoff,
                        now,
                    )?;
                }
                return Ok(plan);
            }
            if current["targetRunId"].is_string()
                && current["targetRunId"] != transfer["targetRunId"]
            {
                return Err(Error::Invariant(
                    "Transfer already belongs to another run.".into(),
                ));
            }
            if transfer["resolution"]["strategy"] == "native_fork" {
                conn.execute("UPDATE orchestration_transfer_delivery SET status='fork_accepted',native_thread_id=?2,payload_json=?3 WHERE transfer_id=?1",
                    rusqlite::params![transfer["id"].as_str(),transfer["resolution"]["providerThreadRef"]["nativeId"].as_str(),
                        json!({"status":"fork_accepted","targetRunId":transfer["targetRunId"],"nativeThreadId":transfer["resolution"]["providerThreadRef"]["nativeId"]}).to_string()])?;
                let target_projection = require_thread(conn, &target)?;
                if let Some(target_run) = target_projection
                    .runs
                    .iter()
                    .find(|r| transfer["targetRunId"] == r.id.0)
                    && let Some(mut provider) = provider_for_run(&target_projection, target_run)
                {
                    let source_id: ThreadId =
                        serde_json::from_value(transfer["sourceThreadId"].clone())?;
                    let source_projection = require_thread(conn, &source_id)?;
                    if let Some(source_run) = source_projection
                        .runs
                        .iter()
                        .find(|r| transfer["sourcePoint"]["runId"] == r.id.0)
                    {
                        provider["nativeThreadRef"] =
                            transfer["resolution"]["providerThreadRef"].clone();
                        provider["forkedFrom"] =
                            json!({"providerThreadId":source_run.provider_thread_id});
                        if let Some(turn) =
                            super::task::records(&source_projection, "provider-turn")
                                .iter()
                                .find(|t| {
                                    source_run
                                        .active_attempt_id
                                        .as_ref()
                                        .is_some_and(|a| t["runAttemptId"] == a.0)
                                })
                        {
                            provider["forkedFrom"]["providerTurnId"] = turn["id"].clone();
                        }
                        plan.emit_on(command, &target, "provider-thread.updated", &provider, now)?;
                    }
                }
            }
            if let Some(handoff) = handoff {
                delivery::persist_receipt(conn, handoff)?;
                plan.emit_on(command, &target, "context-handoff.updated", handoff, now)?;
                handoff_metadata(
                    &require_thread(conn, &target)?,
                    command,
                    &mut plan,
                    handoff,
                    now,
                )?;
            }
            plan.emit_on(command, &target, "context-transfer.updated", transfer, now)?;
        }
        TransferOperation::CreateHandoff { transfer, handoff } => {
            if transfer["sourceThreadId"] != command.thread_id.0
                || transfer["targetThreadId"] != command.thread_id.0
            {
                return Err(Error::Invariant(
                    "Provider handoff must stay in its thread.".into(),
                ));
            }
            plan.emit(command, "context-transfer.created", transfer, now)?;
            plan.emit(command, "context-handoff.updated", handoff, now)?;
            handoff_metadata(&source, command, &mut plan, handoff, now)?;
        }
        TransferOperation::User(_) => {
            return Err(Error::Invariant(
                "Nested user transfer is not supported.".into(),
            ));
        }
        TransferOperation::Checkpoint { record } => {
            conn.execute(
                "INSERT OR IGNORE INTO orchestration_file_checkpoints
                (id,thread_id,run_id,phase,captured_at,payload_json) VALUES(?1,?2,?3,?4,?5,?6)",
                rusqlite::params![
                    record.checkpoint.id.0,
                    record.checkpoint.thread_id.0,
                    record.checkpoint.run_id.as_ref().map(|r| &r.0),
                    record.phase,
                    record.checkpoint.captured_at,
                    serde_json::to_string(record)?
                ],
            )?;
            plan.emit(command, "checkpoint-scope.created", &record.scope, now)?;
            plan.emit(command, "checkpoint.captured", &record.checkpoint, now)?;
            if record.phase == "completed"
                && let Some(run) = source
                    .runs
                    .iter()
                    .find(|r| Some(&r.id) == record.checkpoint.run_id.as_ref())
            {
                let mut run = serde_json::to_value(run)?;
                run["checkpointId"] = json!(record.checkpoint.id);
                plan.emit(command, "run.updated", &run, now)?;
            }
        }
    }
    Ok(plan)
}

fn title_for(operation: &TransferOperation) -> Option<String> {
    if let TransferOperation::Fork { title, .. } = operation {
        title.clone()
    } else {
        None
    }
}

fn handoff_metadata(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    handoff: &Value,
    now: i64,
) -> Result<()> {
    if let Some(run) = projection
        .runs
        .iter()
        .find(|r| handoff["targetRunId"] == r.id.0)
        && run.context_handoff_id.is_none()
    {
        let mut run = json!(run);
        run["contextHandoffId"] = handoff["id"].clone();
        plan.emit_on(command, &projection.thread.id, "run.updated", &run, now)?;
    }
    if let Some(provider) = super::task::records(projection, "provider-thread")
        .iter()
        .find(|p| p["id"] == handoff["toProviderThreadId"])
    {
        let mut provider = provider.clone();
        let handoffs = provider["handoffIds"]
            .as_array_mut()
            .ok_or_else(|| Error::Invariant("Provider handoff roster invalid.".into()))?;
        if !handoffs.contains(&handoff["id"]) {
            handoffs.push(handoff["id"].clone());
            plan.emit_on(
                command,
                &projection.thread.id,
                "provider-thread.updated",
                &provider,
                now,
            )?;
        }
    }
    Ok(())
}

impl Kernel {
    pub async fn transfer_command(
        &self,
        source: &ThreadId,
        id: CommandId,
        operation: TransferOperation,
    ) -> Result<super::CommandReceipt> {
        self.dispatch(
            &Command {
                id,
                thread_id: source.clone(),
                operation: Operation::Transfer(Box::new(operation)),
            },
            crate::now_ms(),
        )
        .await
    }
}

impl super::Store {
    pub fn thread_transfers(&self, id: &ThreadId) -> Result<Vec<Value>> {
        self.read(|conn| transfers(conn, id))
    }
}

pub(crate) fn inherited_items(
    store: &super::Store,
    projection: &ThreadProjection,
) -> Result<Vec<Value>> {
    fn visit(
        store: &super::Store,
        projection: &ThreadProjection,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> Result<Vec<Value>> {
        if !seen.insert(projection.thread.id.0.clone()) {
            return Err(Error::Invariant("Cyclic fork lineage.".into()));
        }
        let transfer = store
            .thread_transfers(&projection.thread.id)?
            .into_iter()
            .find(|t| t["type"] == "fork" && t["targetThreadId"] == projection.thread.id.0);
        let Some(transfer) = transfer else {
            return Ok(vec![]);
        };
        let source: ThreadId = serde_json::from_value(transfer["sourceThreadId"].clone())?;
        let source = store
            .thread(&source)?
            .ok_or_else(|| Error::Invariant("Inherited source thread missing.".into()))?;
        let run = source
            .runs
            .iter()
            .find(|r| transfer["sourcePoint"]["runId"] == r.id.0)
            .ok_or_else(|| Error::Invariant("Inherited source run missing.".into()))?;
        let mut items = visit(store, &source, seen)?;
        items.extend(delivery::local_items(&source, run.ordinal));
        for item in &mut items {
            item["inherited"] = json!(true);
            item["sourceThreadId"] = item["threadId"].clone();
            item["sourceItemId"] = item["id"].clone();
        }
        Ok(items)
    }
    visit(store, projection, &mut Default::default())
}

/// Shared by ThreadService's transactional send planner, ordinary admission,
/// and the queued-start planner BEFORE accepting a new run.
pub fn ensure_start_allowed(transfers: &[Value], thread: &ThreadId, queued: bool) -> Result<()> {
    let pending: Vec<_> = transfers
        .iter()
        .filter(|t| {
            t["targetThreadId"] == thread.0 && t["type"] == "merge_back" && t["status"] == "pending"
        })
        .collect();
    if queued && !pending.is_empty() {
        return Err(Error::Invariant(format!(
            "Thread {thread} has a pending merge-back transfer; queued merge-back consumption is not implemented yet."
        )));
    }
    if pending
        .iter()
        .filter_map(|t| t["sourceThreadId"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1
    {
        return Err(Error::Invariant(format!(
            "Thread {thread} has pending merge-back transfers from multiple forks."
        )));
    }
    Ok(())
}
