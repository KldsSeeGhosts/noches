use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use zeron_proto::orchestration::{CommandId, OrchestrationV2DomainEvent, ThreadId};
use zeron_sync::DocsStore;

use super::command::{self, Command};
use super::effects::{self, Cancellations};
use super::event::{APPLICATION_EVENT_VERSION, Envelope, encode_component, iso};
use super::projection::{self, ThreadProjection, decode};
use super::{Error, Result};

const MIGRATIONS: &[&str] = &[
    include_str!("schema.sql"),
    include_str!("schema_scheduler.sql"),
    include_str!("schema_git_actions.sql"),
    include_str!("schema_launch.sql"),
    include_str!("schema_transfer.sql"),
    include_str!("schema_queue.sql"),
    include_str!("schema_steering.sql"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBoundary {
    BeforeReceipt,
    ReceiptReserved,
    EventAppended,
    ProjectionApplied,
    ProjectionRow,
    BindingActivated,
    ActivityUpdated,
    FrontierUpdated,
    EffectEnqueued,
    EffectsRetired,
    AdoptionRecorded,
    PublicationEnqueued,
    ReceiptFinalized,
    IntentProcessed,
    EffectClaim,
    EffectBegin,
    EffectAck,
    EffectCancel,
    EffectExpired,
    EffectLeaseLost,
    EffectResolved,
    PublicationAck,
    PublicationBatchAck,
    RecoveryEffects,
    ProjectionRebuilt,
    ProjectionCleared,
    EpochAdvanced,
    BeforeCommit,
    /// Simulates loss of the response, NOT a rollback.
    AfterCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptStatus {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    pub command_id: CommandId,
    pub thread_id: ThreadId,
    pub command_type: String,
    pub accepted_at: String,
    pub result_sequence: i64,
    pub status: ReceiptStatus,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct Store {
    pub(crate) thread_locks: Arc<super::ThreadLocks>,
    pub(crate) docs: Arc<DocsStore>,
    pub(crate) host_id: Arc<str>,
    pub(crate) cancellations: Arc<Cancellations>,
    pub(crate) publication_lane: Arc<tokio::sync::Mutex<()>>,
    failure: Arc<Mutex<Option<(WriteBoundary, usize)>>>,
    admission: Arc<OnceLock<super::adoption::RegistryAdmission>>,
}

impl Store {
    pub fn open(docs: Arc<DocsStore>, host_id: &str) -> Result<Self> {
        if host_id.is_empty() {
            return Err(Error::Invariant("host identity is empty".into()));
        }
        docs.with_connection(|conn| -> Result<()> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS orchestration_schema_migrations
                 (version INTEGER PRIMARY KEY,applied_at INTEGER NOT NULL) STRICT",
            )?;
            let current: i64 = conn.query_row(
                "SELECT COALESCE(MAX(version),0) FROM orchestration_schema_migrations",
                [],
                |row| row.get(0),
            )?;
            if current > MIGRATIONS.len() as i64 {
                return Err(Error::Invariant(
                    "kernel schema is newer than this engine".into(),
                ));
            }
            for (index, sql) in MIGRATIONS.iter().enumerate() {
                let version = index as i64 + 1;
                if index == 0 && version <= current {
                    continue;
                }
                // Wave slices appended domain scripts independently, so a
                // positional version can denote different domains before
                // integration. Replay the IF-NOT-EXISTS domain DDL on open;
                // the original, non-idempotent kernel schema runs only once.
                let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT OR IGNORE INTO orchestration_schema_migrations VALUES(?1,?2)",
                    params![version, crate::now_ms()],
                )?;
                tx.commit()?;
            }
            conn.execute(
                "INSERT OR IGNORE INTO orchestration_host VALUES(1,?1,1)",
                [host_id],
            )?;
            let owner: String = conn.query_row(
                "SELECT host_id FROM orchestration_host WHERE singleton=1",
                [],
                |row| row.get(0),
            )?;
            if owner != host_id {
                return Err(Error::NotOwner);
            }
            Ok(())
        })?;
        Ok(Self {
            thread_locks: Arc::default(),
            docs,
            host_id: host_id.into(),
            cancellations: Arc::default(),
            publication_lane: Arc::default(),
            failure: Arc::default(),
            admission: Arc::default(),
        })
    }

    /// Deterministic one-shot crash seam. `occurrence=2`, for example, targets
    /// the second event/projection write. Not exposed through any RPC/MCP API.
    pub fn inject_failure(&self, boundary: WriteBoundary, occurrence: usize) {
        *self.failure.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((boundary, occurrence.max(1)));
    }

    pub(crate) fn boundary(&self, boundary: WriteBoundary) -> Result<()> {
        let mut failure = self.failure.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((target, remaining)) = failure.as_mut()
            && *target == boundary
        {
            *remaining -= 1;
            if *remaining == 0 {
                *failure = None;
                return Err(Error::Injected(boundary));
            }
        }
        Ok(())
    }

    pub(crate) fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        self.docs.with_connection(|conn| {
            // One consistent frontier even if a diagnostic handle has opened
            // another connection to the same profile.
            let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
            let value = f(&tx)?;
            tx.commit()?;
            Ok(value)
        })
    }

    pub(crate) fn write<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        self.docs.with_connection(|conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let owner: String = tx.query_row(
                "SELECT host_id FROM orchestration_host WHERE singleton=1",
                [],
                |row| row.get(0),
            )?;
            if owner != self.host_id.as_ref() {
                return Err(Error::NotOwner);
            }
            let value = f(&tx)?;
            self.boundary(WriteBoundary::BeforeCommit)?;
            tx.commit()?;
            self.boundary(WriteBoundary::AfterCommit)?;
            Ok(value)
        })
    }

    pub fn receipt(&self, id: &CommandId) -> Result<Option<CommandReceipt>> {
        self.read(|conn| receipt(conn, id))
    }

    pub fn thread(&self, id: &ThreadId) -> Result<Option<ThreadProjection>> {
        self.thread_in_project(id, None)
    }

    pub(crate) fn stored_thread(&self, id: &ThreadId) -> Result<Option<ThreadProjection>> {
        self.read(|conn| projection::read_thread(conn, id))
    }

    pub(crate) fn install_admission(&self, admission: super::adoption::RegistryAdmission) {
        let _ = self.admission.set(admission);
    }

    pub(crate) fn registry_unavailable(
        &self,
        id: &ThreadId,
        project: Option<&zeron_proto::orchestration::ProjectId>,
    ) -> bool {
        self.admission
            .get()
            .is_some_and(|a| a.unavailable(id, project))
    }

    pub(crate) fn admit_project(
        &self,
        project: &zeron_proto::orchestration::ProjectId,
    ) -> Result<()> {
        if let Some(admission) = self.admission.get() {
            for chat in admission
                .workspace
                .read_chats()
                .map_err(|e| Error::Invariant(e.to_string()))?
            {
                if chat.device_id == self.host_id.as_ref()
                    && super::adoption::project_id(&chat) == project.0
                {
                    self.thread_in_project(&ThreadId(chat.id), Some(project))?;
                }
            }
        }
        Ok(())
    }
    /// Resolve outside the SQL connection. The stable adoption receipt and
    /// immediate transaction fence independently racing handles/first calls.
    pub(crate) fn thread_in_project(
        &self,
        id: &ThreadId,
        project: Option<&zeron_proto::orchestration::ProjectId>,
    ) -> Result<Option<ThreadProjection>> {
        if self.registry_unavailable(id, project) {
            return Ok(None);
        }
        let existing = self.read(|conn| projection::read_thread(conn, id))?;
        if existing.is_some() {
            return Ok(existing);
        }
        let Some(admission) = self.admission.get() else {
            return Ok(None);
        };
        let Some((thread, messages)) = admission.prepare(id, project)? else {
            return Ok(None);
        };
        let receipt = self.dispatch(
            &Command {
                id: CommandId(format!("registry-adopt:{}", encode_component(&id.0))),
                thread_id: id.clone(),
                operation: super::Operation::Adopt {
                    legacy_chat_id: id.0.clone(),
                    thread: Box::new(thread),
                    messages,
                },
            },
            crate::now_ms(),
        )?;
        // An ordinary turn could have created the projection after our read.
        // Its existing identity wins; never overwrite or replay its input.
        let current = self.read(|conn| projection::read_thread(conn, id))?;
        if current.is_none() && receipt.status == ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(current)
    }

    pub fn events(&self) -> Result<Vec<Envelope>> {
        self.read(events)
    }

    pub fn projection_frontier(&self) -> Result<i64> {
        self.read(|conn| {
            Ok(conn.query_row(
                "SELECT last_sequence FROM orchestration_projection_metadata WHERE singleton=1",
                [],
                |row| row.get(0),
            )?)
        })
    }

    pub fn rebuild(&self) -> Result<()> {
        self.write(|tx| {
            projection::rebuild_checked(tx, &events(tx)?, &|boundary| self.boundary(boundary))?;
            self.boundary(WriteBoundary::ProjectionRebuilt)
        })
    }

    pub fn dispatch(&self, command: &Command, now: i64) -> Result<CommandReceipt> {
        let (receipt, cancellations) = self.write(|tx| {
            // Stable ID, not payload equality, is T3's replay namespace.
            if let Some(receipt) = receipt(tx, &command.id)? {
                return Ok((receipt, vec![]));
            }
            self.boundary(WriteBoundary::BeforeReceipt)?;
            let command_type = command.command_type()?;
            let accepted_at = iso(now)?;
            let planned = command::plan(tx, command, now);
            let plan = match planned {
                Ok(plan) => plan,
                Err(Error::Invariant(error)) => {
                    let sequence = latest_sequence(tx)?;
                    let receipt = CommandReceipt {
                        command_id: command.id.clone(),
                        thread_id: command.thread_id.clone(),
                        command_type,
                        accepted_at,
                        result_sequence: sequence,
                        status: ReceiptStatus::Rejected,
                        error: Some(error),
                    };
                    put_receipt(tx, &receipt)?;
                    self.boundary(WriteBoundary::ReceiptFinalized)?;
                    return Ok((receipt, vec![]));
                }
                Err(error) => return Err(error),
            };
            if plan.events.is_empty() && !matches!(command.operation, super::Operation::Queue(_)) {
                return Err(Error::Invariant(
                    "accepted command produced no events".into(),
                ));
            }
            let mut receipt = CommandReceipt {
                command_id: command.id.clone(),
                thread_id: command.thread_id.clone(),
                command_type,
                accepted_at,
                result_sequence: latest_sequence(tx)?,
                status: ReceiptStatus::Accepted,
                error: None,
            };
            put_receipt(tx, &receipt)?;
            self.boundary(WriteBoundary::ReceiptReserved)?;
            let mut changed_threads = std::collections::BTreeSet::new();
            for event in plan.events {
                let value = serde_json::to_value(&event)?;
                let thread: ThreadId = serde_json::from_value(value["threadId"].clone())?;
                if !command.lock_threads().contains(&thread) {
                    return Err(Error::Invariant(
                        "kernel command crosses lock participants".into(),
                    ));
                }
                let stored = self.append_event(tx, Some(command.id.clone()), event)?;
                receipt.result_sequence = stored.sequence;
                changed_threads.insert(thread.0);
            }
            let mut cancellations = if plan.cancel_process_effects {
                let ids = effects::cancel_process(tx, &command.thread_id, now)?;
                self.boundary(WriteBoundary::EffectsRetired)?;
                ids
            } else {
                vec![]
            };
            for thread in &plan.cancel_threads {
                cancellations.extend(effects::cancel_process(tx, thread, now)?);
                self.boundary(WriteBoundary::EffectsRetired)?;
            }
            for (index, request) in plan.effects.iter().enumerate() {
                effects::enqueue(
                    tx,
                    &format!("effect:{}:{index}", encode_component(&command.id.0)),
                    &command.id,
                    &command.thread_id,
                    request,
                    now,
                )?;
                self.boundary(WriteBoundary::EffectEnqueued)?;
            }
            for (index, (thread, request)) in plan.routed_effects.iter().enumerate() {
                if !command.lock_threads().contains(thread) {
                    return Err(Error::Invariant("effect crosses lock participants".into()));
                }
                effects::enqueue(
                    tx,
                    &format!(
                        "effect:{}:{}",
                        encode_component(&command.id.0),
                        index + plan.effects.len()
                    ),
                    &command.id,
                    thread,
                    request,
                    now,
                )?;
                self.boundary(WriteBoundary::EffectEnqueued)?;
            }
            if let Some(legacy) = &plan.adoption {
                tx.execute(
                    "INSERT INTO orchestration_adoptions VALUES(?1,?2,?3,'legacy/native')",
                    params![legacy, command.thread_id.0, receipt.result_sequence],
                )?;
                self.boundary(WriteBoundary::AdoptionRecorded)?;
            }
            super::ui_queue::persist(
                tx,
                &command.thread_id,
                plan.queue_lifecycle.as_ref(),
                plan.queue_intents.as_deref(),
            )?;
            if let Some(patch) = &plan.queue_patch {
                tx.execute(
                    "INSERT OR IGNORE INTO orchestration_queue_patches VALUES(?1,?2,?3,?4)",
                    params![
                        command.id.0,
                        command.thread_id.0,
                        receipt.result_sequence,
                        serde_json::to_string(patch)?
                    ],
                )?;
            }
            if !changed_threads.is_empty() {
                super::sync_publish::enqueue(
                    tx,
                    &self.host_id,
                    &receipt.command_id,
                    receipt.result_sequence,
                    changed_threads,
                )?;
                self.boundary(WriteBoundary::PublicationEnqueued)?;
            }
            put_receipt(tx, &receipt)?;
            self.boundary(WriteBoundary::ReceiptFinalized)?;
            Ok((receipt, cancellations))
        })?;
        self.cancellations.cancel(&cancellations);
        Ok(receipt)
    }

    /// T3 EventSink guarded external input: a stale owner commits nothing, not
    /// even a command receipt. Stable event IDs deduplicate exact batch replay.
    pub fn append_provider_events(
        &self,
        thread_id: &ThreadId,
        guard: &command::ProviderGuard,
        input: &[OrchestrationV2DomainEvent],
        now: i64,
    ) -> Result<bool> {
        self.write(|tx| {
            let Some(projection) = projection::read_thread(tx, thread_id)? else {
                return Ok(false);
            };
            match command::check_guard(tx, &projection, guard) {
                Ok(()) => {}
                Err(Error::Invariant(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
            let command = Command {
                id: CommandId("external-input".into()),
                thread_id: thread_id.clone(),
                operation: command::Operation::ProviderEvents {
                    guard: guard.clone(),
                    events: input.to_vec(),
                },
            };
            let plan = command::plan(tx, &command, now)?;
            let mut duplicates = 0;
            for event in &plan.events {
                let value = serde_json::to_value(event)?;
                let existing: Option<String> = tx
                    .query_row(
                        "SELECT envelope_json FROM orchestration_events WHERE event_id=?1",
                        [value["id"].as_str()],
                        |row| row.get(0),
                    )
                    .optional()?;
                if let Some(existing) = existing {
                    let existing: Envelope = decode(&existing)?;
                    if existing.event != *event {
                        return Err(Error::Invariant(
                            "event identity reused with different content".into(),
                        ));
                    }
                    duplicates += 1;
                }
            }
            if duplicates == plan.events.len() {
                return Ok(false);
            }
            if duplicates != 0 {
                return Err(Error::Invariant(
                    "partially duplicated provider batch".into(),
                ));
            }
            let mut sequence = 0;
            let mut last_id = String::new();
            for event in plan.events {
                last_id = serde_json::to_value(&event)?["id"].as_str().unwrap().into();
                sequence = self.append_event(tx, None, event)?.sequence;
            }
            super::sync_publish::enqueue(
                tx,
                &self.host_id,
                &CommandId(format!("external:{last_id}")),
                sequence,
                [thread_id.0.clone()].into(),
            )?;
            self.boundary(WriteBoundary::PublicationEnqueued)?;
            Ok(true)
        })
    }

    pub(crate) fn append_event(
        &self,
        tx: &Transaction<'_>,
        command_id: Option<CommandId>,
        event: OrchestrationV2DomainEvent,
    ) -> Result<Envelope> {
        let value = serde_json::to_value(&event)?;
        let thread: ThreadId = serde_json::from_value(value["threadId"].clone())?;
        let stream_version: i64 = tx.query_row(
            "SELECT COALESCE(MAX(stream_version)+1,0) FROM orchestration_events WHERE stream_id=?1",
            [&thread.0],
            |row| row.get(0),
        )?;
        let sequence = latest_sequence(tx)? + 1;
        let stored = Envelope {
            application_event_version: APPLICATION_EVENT_VERSION,
            sequence,
            stream_version,
            command_id,
            event,
        };
        tx.execute(
            "INSERT INTO orchestration_events
             (sequence,application_event_version,event_id,command_id,aggregate_kind,
              stream_id,stream_version,event_type,occurred_at,envelope_json)
             VALUES(?1,2,?2,?3,'thread',?4,?5,?6,?7,?8)",
            params![
                sequence,
                value["id"].as_str(),
                stored.command_id.as_ref().map(|id| &id.0),
                thread.0,
                stream_version,
                value["type"].as_str(),
                value["occurredAt"].as_str(),
                serde_json::to_string(&stored)?
            ],
        )?;
        self.boundary(WriteBoundary::EventAppended)?;
        projection::apply_checked(tx, &stored, &|boundary| self.boundary(boundary))?;
        self.boundary(WriteBoundary::ProjectionApplied)?;
        Ok(stored)
    }

    /// New V2 remote-intent routing must call this AFTER dispatch returned a
    /// durable receipt. A crash between the calls only replays that receipt.
    /// This does not change the isolated legacy executor's behavior.
    pub fn mark_intent_processed(&self, id: &CommandId, now: i64) -> Result<()> {
        self.write(|tx| {
            if receipt(tx, id)?.is_none() {
                return Err(Error::Invariant(
                    "cannot process an unreceipted V2 intent".into(),
                ));
            }
            tx.execute(
                "INSERT OR IGNORE INTO processed_commands(command_id,processed_at) VALUES(?1,?2)",
                params![id.0, now],
            )?;
            self.boundary(WriteBoundary::IntentProcessed)
        })
    }

    pub fn is_v2_managed(&self, legacy_chat_id: &str) -> Result<bool> {
        self.read(|conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM orchestration_adoptions WHERE legacy_chat_id=?1)",
                [legacy_chat_id],
                |row| row.get(0),
            )?)
        })
    }
}

pub(crate) fn latest_sequence(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM orchestration_events",
        [],
        |row| row.get(0),
    )?)
}

pub(crate) fn events(conn: &Connection) -> Result<Vec<Envelope>> {
    let mut stmt =
        conn.prepare("SELECT envelope_json FROM orchestration_events ORDER BY sequence")?;
    stmt.query_map([], |row| row.get::<_, String>(0))?
        .map(|row| decode(&row?))
        .collect()
}

fn receipt(conn: &Connection, id: &CommandId) -> Result<Option<CommandReceipt>> {
    let raw = conn
        .query_row(
            "SELECT thread_id,command_type,accepted_at,result_sequence,status,error
         FROM orchestration_command_receipts WHERE command_id=?1",
            [&id.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()?;
    raw.map(
        |(thread, command_type, accepted_at, sequence, status, error)| {
            Ok(CommandReceipt {
                command_id: id.clone(),
                thread_id: ThreadId(thread),
                command_type,
                accepted_at,
                result_sequence: sequence,
                status: decode(&format!("\"{status}\""))?,
                error,
            })
        },
    )
    .transpose()
}

fn put_receipt(conn: &Connection, receipt: &CommandReceipt) -> Result<()> {
    conn.execute(
        "INSERT INTO orchestration_command_receipts VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(command_id) DO UPDATE SET result_sequence=excluded.result_sequence",
        params![
            receipt.command_id.0,
            receipt.thread_id.0,
            receipt.command_type,
            receipt.accepted_at,
            receipt.result_sequence,
            if receipt.status == ReceiptStatus::Accepted {
                "accepted"
            } else {
                "rejected"
            },
            receipt.error
        ],
    )?;
    Ok(())
}
