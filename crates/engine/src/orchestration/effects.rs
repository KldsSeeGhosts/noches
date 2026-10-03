//! Leased, per-thread FIFO effect lanes. Provider starts complete on acceptance,
//! not when the live provider session ends: four workers are not four children.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use zeron_proto::orchestration::*;

use super::projection::decode;
use super::store::{Store, WriteBoundary};
use super::{Error, Result};

pub const DEFAULT_WORKER_CONCURRENCY: usize = 4;
pub const DEFAULT_LEASE_MS: i64 = 30_000;
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum EffectRequest {
    #[serde(rename = "provider-runtime.continue")]
    ProviderRuntimeContinue { source_run_id: RunId },
    #[serde(rename = "provider-session.detach")]
    ProviderSessionDetach {
        provider_session_id: ProviderSessionId,
    },
    #[serde(rename = "provider-turn.start")]
    ProviderTurnStart { run_id: RunId },
    #[serde(rename = "provider-turn.interrupt")]
    ProviderTurnInterrupt {
        provider_session_id: ProviderSessionId,
        provider_thread_id: ProviderThreadId,
        provider_turn_id: ProviderTurnId,
    },
    #[serde(rename = "provider-turn.steer")]
    ProviderTurnSteer {
        provider_session_id: ProviderSessionId,
        provider_thread_id: ProviderThreadId,
        provider_turn_id: ProviderTurnId,
        message_id: MessageId,
    },
    #[serde(rename = "provider-turn.restart")]
    ProviderTurnRestart {
        provider_session_id: ProviderSessionId,
        provider_thread_id: ProviderThreadId,
        provider_turn_id: ProviderTurnId,
        interrupted_attempt_id: RunAttemptId,
        run_id: RunId,
    },
    #[serde(rename = "runtime-request.respond")]
    RuntimeRequestRespond {
        provider_session_id: ProviderSessionId,
        request_id: RuntimeRequestId,
        decision: Option<ProviderApprovalDecision>,
        answers: Option<ProviderUserInputAnswers>,
    },
    #[serde(rename = "provider-thread.rollback")]
    ProviderThreadRollback {
        provider_thread_id: ProviderThreadId,
        checkpoint_id: CheckpointId,
        scope_id: CheckpointScopeId,
    },
    #[serde(rename = "checkpoint.capture")]
    CheckpointCapture {
        run_id: RunId,
        scope_id: CheckpointScopeId,
    },
    #[serde(rename = "terminal.cleanup")]
    TerminalCleanup,
    #[serde(rename = "attachment.cleanup")]
    AttachmentCleanup { attachment_ids: Vec<String> },
    #[serde(rename = "thread-title.generate")]
    ThreadTitleGenerate { kind: TitleKind },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum TitleKind {
    #[serde(rename = "initial")]
    Initial { message_id: MessageId },
    #[serde(rename = "regenerate")]
    Regenerate,
}

impl EffectRequest {
    pub fn process_bound(&self) -> bool {
        matches!(
            self,
            Self::ProviderTurnStart { .. }
                | Self::ProviderTurnInterrupt { .. }
                | Self::ProviderTurnSteer { .. }
                | Self::ProviderTurnRestart { .. }
                | Self::RuntimeRequestRespond { .. }
        )
    }

    pub fn lane(&self) -> &'static str {
        if matches!(self, Self::ThreadTitleGenerate { .. }) {
            "title"
        } else {
            "provider"
        }
    }

    pub(crate) fn effect_type(&self) -> Result<String> {
        Ok(serde_json::to_value(self)?["type"]
            .as_str()
            .expect("effect type")
            .into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: String,
    pub command_id: CommandId,
    pub thread_id: ThreadId,
    pub request: EffectRequest,
    pub status: EffectStatus,
    pub attempt_count: u32,
    pub available_at: i64,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<i64>,
    pub dispatch_started: bool,
    pub last_error: Option<String>,
}

#[derive(Default)]
pub(crate) struct Cancellations(Mutex<HashMap<String, (u32, CancellationToken)>>);

impl Cancellations {
    pub(crate) fn cancel(&self, ids: &[String]) {
        let tokens = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        for id in ids {
            if let Some((_, token)) = tokens.get(id) {
                token.cancel();
            }
        }
    }

    fn register(&self, id: &str, generation: u32) -> CancellationToken {
        let token = CancellationToken::new();
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id.into(), (generation, token.clone()));
        token
    }

    fn clear(&self, id: &str, generation: u32) {
        let mut tokens = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if tokens
            .get(id)
            .is_some_and(|(current, _)| *current == generation)
        {
            tokens.remove(id);
        }
    }
}

pub(crate) fn get(conn: &Connection, id: &str) -> Result<Option<Effect>> {
    let raw = conn
        .query_row(
            "SELECT command_id,thread_id,payload_json,status,attempt_count,available_at,
            lease_owner,lease_expires_at,dispatch_started,last_error
         FROM orchestration_effect_outbox WHERE effect_id=?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, u32>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()?;
    raw.map(
        |(
            command,
            thread,
            request,
            status,
            attempts,
            available,
            owner,
            expires,
            started,
            error,
        )| {
            Ok(Effect {
                id: id.into(),
                command_id: CommandId(command),
                thread_id: ThreadId(thread),
                request: decode(&request)?,
                status: decode(&format!("\"{status}\""))?,
                attempt_count: attempts,
                available_at: available,
                lease_owner: owner,
                lease_expires_at: expires,
                dispatch_started: started,
                last_error: error,
            })
        },
    )
    .transpose()
}

pub(crate) fn enqueue(
    conn: &Connection,
    id: &str,
    command: &CommandId,
    thread: &ThreadId,
    request: &EffectRequest,
    now: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO orchestration_effect_outbox
         (effect_id,command_id,thread_id,effect_type,lane,payload_json,process_bound,
          status,available_at,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8,?8)",
        params![
            id,
            command.0,
            thread.0,
            request.effect_type()?,
            request.lane(),
            serde_json::to_string(request)?,
            request.process_bound(),
            now
        ],
    )?;
    Ok(())
}

pub(crate) fn cancel_process(
    conn: &Connection,
    thread: &ThreadId,
    now: i64,
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "UPDATE orchestration_effect_outbox SET status='cancelled',lease_owner=NULL,
         lease_expires_at=NULL,completed_at=?2,last_error='Process work retired by owner.'
         WHERE thread_id=?1 AND process_bound=1 AND status IN ('pending','running')
         RETURNING effect_id",
    )?;
    Ok(stmt
        .query_map(params![thread.0, now], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?)
}

impl Store {
    pub fn effect(&self, id: &str) -> Result<Option<Effect>> {
        self.read(|conn| get(conn, id))
    }

    pub fn effects(&self) -> Result<Vec<Effect>> {
        self.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT effect_id FROM orchestration_effect_outbox ORDER BY ordinal")?;
            stmt.query_map([], |row| row.get::<_, String>(0))?
                .map(|id| {
                    get(conn, &id?)?.ok_or_else(|| Error::Invariant("effect disappeared".into()))
                })
                .collect()
        })
    }

    /// Expired process work is never stolen by a second worker. Once dispatch
    /// started its acceptance is uncertain; the lane is held until reconciled.
    pub fn claim_effect(&self, worker: &str, now: i64, lease_ms: i64) -> Result<Option<Effect>> {
        self.write(|tx| {
            tx.execute(
                "UPDATE orchestration_effect_outbox
                 SET status=CASE WHEN process_bound=1 AND dispatch_started=1 THEN 'uncertain' ELSE 'pending' END,
                     lease_owner=NULL,lease_expires_at=NULL,available_at=?1,
                     last_error='Effect lease expired.'
                 WHERE status='running' AND lease_expires_at<=?1",
                [now],
            )?;
            self.boundary(WriteBoundary::EffectExpired)?;
            let id: Option<String> = tx.query_row(
                "SELECT candidate.effect_id FROM orchestration_effect_outbox candidate
                 WHERE candidate.status='pending' AND candidate.available_at<=?1
                   AND NOT EXISTS (
                     SELECT 1 FROM orchestration_effect_outbox active
                     WHERE active.thread_id=candidate.thread_id AND active.lane=candidate.lane
                       AND (active.status IN ('running','uncertain')
                         OR (active.status='pending' AND active.ordinal<candidate.ordinal)))
                 ORDER BY candidate.available_at,candidate.ordinal LIMIT 1",
                [now], |row| row.get(0),
            ).optional()?;
            if let Some(id) = &id {
                tx.execute(
                    "UPDATE orchestration_effect_outbox SET status='running',
                     attempt_count=attempt_count+1,lease_owner=?2,lease_expires_at=?3,dispatch_started=0
                     WHERE effect_id=?1",
                    params![id, worker, now.saturating_add(lease_ms.max(1))],
                )?;
                self.boundary(WriteBoundary::EffectClaim)?;
            }
            id.map(|id| get(tx, &id)?.ok_or_else(|| Error::Invariant("claim disappeared".into()))).transpose()
        })
    }

    pub fn begin_effect(&self, effect: &Effect, now: i64) -> Result<bool> {
        self.write(|tx| {
            let changed = tx.execute(
                "UPDATE orchestration_effect_outbox SET dispatch_started=1
                 WHERE effect_id=?1 AND status='running' AND lease_owner=?2
                   AND attempt_count=?3 AND lease_expires_at>?4 AND dispatch_started=0",
                params![effect.id, effect.lease_owner, effect.attempt_count, now],
            )? == 1;
            self.boundary(WriteBoundary::EffectBegin)?;
            Ok(changed)
        })
    }

    /// Lease fencing uses both worker and claim generation. Even a reused worker
    /// ID cannot acknowledge a reclaimed effect.
    pub fn finish_effect(
        &self,
        effect: &Effect,
        outcome: &EffectOutcome,
        now: i64,
        max_attempts: u32,
    ) -> Result<bool> {
        let (status, available, error) = match outcome {
            EffectOutcome::Succeeded => ("succeeded", now, None),
            EffectOutcome::Uncertain => {
                ("uncertain", now, Some("External acceptance is uncertain."))
            }
            EffectOutcome::Failed => ("failed", now, Some("Effect execution failed.")),
            EffectOutcome::Retry if effect.attempt_count < max_attempts.max(1) => (
                "pending",
                now.saturating_add(retry_delay_ms(effect.attempt_count)),
                Some("Retryable effect execution failure."),
            ),
            EffectOutcome::Retry => ("failed", now, Some("Effect retry budget exhausted.")),
        };
        self.write(|tx| {
            let changed = tx.execute(
                "UPDATE orchestration_effect_outbox SET status=?5,available_at=?6,
                    lease_owner=NULL,lease_expires_at=NULL,last_error=?7,
                    completed_at=CASE WHEN ?5 IN ('succeeded','failed') THEN ?4 ELSE NULL END
                 WHERE effect_id=?1 AND status='running' AND lease_owner=?2
                   AND attempt_count=?3 AND lease_expires_at>?4",
                params![
                    effect.id,
                    effect.lease_owner,
                    effect.attempt_count,
                    now,
                    status,
                    available,
                    error
                ],
            )? == 1;
            self.boundary(WriteBoundary::EffectAck)?;
            Ok(changed)
        })
    }

    pub fn cancel_effect(&self, id: &str, now: i64) -> Result<bool> {
        let cancelled = self.write(|tx| {
            let changed = tx.execute(
                "UPDATE orchestration_effect_outbox SET status='cancelled',completed_at=?2,
                 lease_owner=NULL,lease_expires_at=NULL,last_error='Cancelled by owner.'
                 WHERE effect_id=?1 AND status IN ('pending','running','uncertain')",
                params![id, now],
            )? == 1;
            self.boundary(WriteBoundary::EffectCancel)?;
            Ok(changed)
        })?;
        if cancelled {
            self.cancellations.cancel(&[id.into()]);
        }
        Ok(cancelled)
    }

    /// Losing a lease does not prove an external operation was unaccepted.
    /// Only the exact claim still present may be abandoned; never touch a new
    /// owner's claim. This also durably records timeout uncertainty immediately.
    pub fn abandon_effect(&self, effect: &Effect, now: i64) -> Result<bool> {
        self.write(|tx| {
            let changed = tx.execute(
                "UPDATE orchestration_effect_outbox
                 SET status=CASE WHEN process_bound=1 THEN 'uncertain' ELSE 'pending' END,
                     lease_owner=NULL,lease_expires_at=NULL,available_at=?4,
                     last_error='Worker lost lease after dispatch.'
                 WHERE effect_id=?1 AND status='running' AND lease_owner=?2 AND attempt_count=?3",
                params![effect.id, effect.lease_owner, effect.attempt_count, now],
            )? == 1;
            self.boundary(WriteBoundary::EffectLeaseLost)?;
            Ok(changed)
        })
    }

    /// Only reconciliation may release an uncertainty barrier. No automatic
    /// retry is offered: a provider-specific later slice must prove acceptance.
    pub fn resolve_uncertain(&self, id: &str, accepted: bool, now: i64) -> Result<bool> {
        self.write(|tx| {
            let changed = tx.execute(
                "UPDATE orchestration_effect_outbox SET status=?2,completed_at=?3
             WHERE effect_id=?1 AND status='uncertain'",
                params![id, if accepted { "succeeded" } else { "cancelled" }, now],
            )? == 1;
            self.boundary(WriteBoundary::EffectResolved)?;
            Ok(changed)
        })
    }
}

pub fn retry_delay_ms(attempt_count: u32) -> i64 {
    (100_i64.saturating_mul(1_i64 << attempt_count.saturating_sub(1).min(20))).min(30_000)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOutcome {
    Succeeded,
    /// Executor knows that retry is safe; never use for ambiguous acceptance.
    Retry,
    Failed,
    Uncertain,
}

#[async_trait]
pub trait EffectExecutor: Send + Sync {
    /// Stable effect identity is the external idempotency identity. Return on
    /// acceptance, not turn completion. No SQL mutex/transaction is held here.
    /// Dropping this future requests cancellation; adapters must support it.
    async fn execute(&self, effect: &Effect, cancellation: CancellationToken) -> EffectOutcome;
}

pub struct EffectWorker {
    pub store: Store,
    pub executor: Arc<dyn EffectExecutor>,
    pub worker_id: String,
    pub lease_ms: i64,
    pub max_attempts: u32,
}

impl EffectWorker {
    pub fn new(store: Store, executor: Arc<dyn EffectExecutor>, worker_id: String) -> Self {
        Self {
            store,
            executor,
            worker_id,
            lease_ms: DEFAULT_LEASE_MS,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }

    /// One bounded claim; the daemon will own scheduling in the runner slice.
    pub async fn step(&self, now: i64) -> Result<bool> {
        let started_at = std::time::Instant::now();
        let Some(effect) = self
            .store
            .claim_effect(&self.worker_id, now, self.lease_ms)?
        else {
            return Ok(false);
        };
        let token = self
            .store
            .cancellations
            .register(&effect.id, effect.attempt_count);
        struct Registration<'a> {
            cancellations: &'a Cancellations,
            id: &'a str,
            generation: u32,
        }
        impl Drop for Registration<'_> {
            fn drop(&mut self) {
                self.cancellations.clear(self.id, self.generation);
            }
        }
        let _registration = Registration {
            cancellations: &self.store.cancellations,
            id: &effect.id,
            generation: effect.attempt_count,
        };
        let started = self.store.begin_effect(&effect, now);
        if !matches!(started, Ok(true)) {
            return started.map(|_| true);
        }
        let outcome = tokio::select! {
            biased;
            _ = token.cancelled() => {
                return Ok(true);
            }
            outcome = self.executor.execute(&effect, token.clone()) => outcome,
            _ = tokio::time::sleep(std::time::Duration::from_millis(self.lease_ms.max(1) as u64)) => {
                if effect.request.process_bound() { EffectOutcome::Uncertain } else { EffectOutcome::Retry }
            }
        };
        let completed_at =
            now.saturating_add(started_at.elapsed().as_millis().min(i64::MAX as u128) as i64);
        if !self
            .store
            .finish_effect(&effect, &outcome, completed_at, self.max_attempts)?
        {
            let status = self.store.effect(&effect.id)?.map(|effect| effect.status);
            if status != Some(EffectStatus::Cancelled)
                && !self.store.abandon_effect(&effect, completed_at)?
            {
                return Err(Error::Invariant("worker lost its effect lease".into()));
            }
        }
        Ok(true)
    }
}
