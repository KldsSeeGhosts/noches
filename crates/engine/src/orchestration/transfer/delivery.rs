//! Durable native/inline delivery receipts. Pending means acceptance may have
//! happened: use a fresh native thread, never blindly re-inject into the old one.
use super::super::task::{SelectionTransition, selection_transition};
use super::{context, provider_for_run};
use crate::orchestration::{
    Error, Kernel, Result,
    event::{encode_component, iso},
    projection::ThreadProjection,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use zeron_harness::{Harness, mcp::SessionMcpContext, session_lifecycle::NativeForkRequest};
use zeron_proto::{RunRequest, orchestration::*};

/// TODO(merge-threads): a raw, pinned-run, observational history primitive.
/// ThreadService::read is not substitutable: its page is rendered/truncated
/// text, lacks raw native/tool fields and a through-run bound, and can
/// acknowledge a delegated result. This trait cannot send or acknowledge.
#[async_trait]
pub trait TransferThreadAccess: Send + Sync {
    async fn history(&self, thread: &ThreadId, through_run: &RunId) -> Result<Vec<Value>>;
}
pub struct KernelThreadAccess {
    pub kernel: Kernel,
}
#[async_trait]
impl TransferThreadAccess for KernelThreadAccess {
    async fn history(&self, thread: &ThreadId, through_run: &RunId) -> Result<Vec<Value>> {
        let projection = self
            .kernel
            .store
            .thread(thread)?
            .ok_or_else(|| Error::Invariant("Source thread missing.".into()))?;
        let ordinal = projection
            .runs
            .iter()
            .find(|r| &r.id == through_run)
            .map(|r| r.ordinal)
            .ok_or_else(|| Error::Invariant("Source run missing.".into()))?;
        let mut items = super::inherited_items(&self.kernel.store, &projection)?;
        items.extend(local_items(&projection, ordinal));
        Ok(items)
    }
}

pub(crate) fn local_items(projection: &ThreadProjection, ordinal: i64) -> Vec<Value> {
    let runs: std::collections::BTreeMap<_, _> = projection
        .runs
        .iter()
        .filter(|r| r.ordinal <= ordinal && r.status != OrchestrationV2RunStatus::RolledBack)
        .map(|r| (r.id.0.as_str(), r))
        .collect();
    let turn_items = super::super::task::records(projection, "turn-item");
    let canonical_messages: std::collections::HashSet<_> = turn_items
        .iter()
        .filter(|item| item["runId"].is_string())
        .filter_map(|item| item["messageId"].as_str())
        .collect();
    let mut items: Vec<_> = turn_items
        .iter()
        .filter(|i| {
            i["runId"].as_str().is_some_and(|id| runs.contains_key(id))
                || (i["runId"].is_null()
                    && projection.thread.history_origin.as_ref()
                        == Some(&OrchestrationV2ThreadHistoryOrigin::V1Import)
                    && !i["messageId"]
                        .as_str()
                        .is_some_and(|id| canonical_messages.contains(id)))
        })
        .cloned()
        .collect();
    // Canonical turn-items come from threads; legacy ordinary runs may still
    // have only message rows. Preserve those without duplicating modern items.
    for message in super::super::task::records(projection, "message") {
        let Some(run) = message["runId"].as_str().and_then(|id| runs.get(id)) else {
            continue;
        };
        if let Some(item) = items
            .iter_mut()
            .find(|i| i["messageId"] == message["id"] || i["id"] == message["id"])
        {
            // Ordinary admission can adopt the Loro user entry before the
            // provider accepts it. Its imported item must retain its stable ID,
            // but the canonical message now owns a logical run. Without that
            // association, retry filtering silently drops the untold root input.
            // This is history attribution, not a native acceptance receipt.
            if item["runId"].is_null() {
                item["runId"] = json!(run.id);
                item["nodeId"] = message["nodeId"].clone();
                item["providerThreadId"] = json!(run.provider_thread_id);
            }
            continue;
        }
        items.push(json!({"id":message["id"],"type":if message["role"] == "user" {"user_message"} else {"assistant_message"},
            "threadId":projection.thread.id,"runId":run.id,"providerThreadId":run.provider_thread_id,
            "text":message["text"],"status":run.status,"ordinal":if message["role"] == "user" {0} else {i64::MAX},
            "runStatus":run.status}));
    }
    items.sort_by_key(|i| {
        (
            i["runId"]
                .as_str()
                .and_then(|id| runs.get(id))
                .map(|r| r.ordinal)
                .unwrap_or(0),
            i["ordinal"].as_i64().unwrap_or(0),
        )
    });
    for item in &mut items {
        if let Some(run) = item["runId"].as_str().and_then(|id| runs.get(id)) {
            item["runStatus"] = json!(run.status);
        }
    }
    items
}

pub fn native_fork_eligible(
    transfer: &Value,
    run: &OrchestrationV2Run,
    target: &ProviderInstanceId,
    capabilities: &OrchestrationV2ProviderCapabilities,
) -> bool {
    matches!(
        run.status,
        OrchestrationV2RunStatus::Completed | OrchestrationV2RunStatus::Waiting
    ) && transfer["sourceProviderInstanceId"] == target.0
        && transfer["sourcePoint"]["providerThreadRef"]["strength"] == "strong"
        && capabilities.threads.can_fork_thread
        && capabilities.threads.can_fork_from_turn
        && capabilities.identity.native_thread_ids == OrchestrationV2NativeRefStrength::Strong
}

/// Native resume is authorized by the *selected* provider handle's accepted
/// root, not the last session on the app chat. Retired process records can be
/// recovered from the event log after restart without reviving a process.
pub(super) fn resumable_native(
    kernel: &Kernel,
    projection: &ThreadProjection,
    run: &OrchestrationV2Run,
    cwd: &str,
) -> Result<Option<String>> {
    let Some(provider) = provider_for_run(projection, run).filter(|p| {
        p["providerInstanceId"] == run.provider_instance_id.0
            && !matches!(p["status"].as_str(), Some("closed" | "archived" | "error"))
    }) else {
        return Ok(None);
    };
    let Some(native) = provider["nativeThreadRef"]["nativeId"]
        .as_str()
        .filter(|id| !id.is_empty())
    else {
        return Ok(None);
    };
    let accepted = projection.runs.iter().rev().find(|previous| {
        previous.ordinal <= run.ordinal
            && previous.provider_thread_id == run.provider_thread_id
            && previous.provider_instance_id == run.provider_instance_id
            && (previous.id == run.id || super::forkable(&previous.status))
            && projection.attempts.iter().any(|attempt| {
                attempt.run_id == previous.id
                    && Some(&attempt.provider_thread_id) == previous.provider_thread_id.as_ref()
                    && attempt.provider_instance_id == previous.provider_instance_id
                    && (previous.active_attempt_id.as_ref() == Some(&attempt.id)
                        // Restart preserves the native conversation's accepted
                        // predecessor, not a fabricated turn on its new root.
                        || (previous.id == run.id
                            && attempt.status == OrchestrationV2RunAttemptStatus::Superseded
                            && projection.attempts.iter().any(|current| {
                                Some(&current.id) == run.active_attempt_id.as_ref()
                                    && current.run_id == run.id
                                    && Some(&current.root_node_id) == run.root_node_id.as_ref()
                                    && current.reason
                                        == OrchestrationV2RunAttemptReason::SteeringRestart
                                    && current.attempt_ordinal == attempt.attempt_ordinal + 1
                                    && current.provider_thread_id == attempt.provider_thread_id
                                    && current.provider_instance_id == attempt.provider_instance_id
                            })))
                    && attempt.native_thread_id.as_ref().map(String::as_str) == Some(native)
                    && super::super::task::records(projection, "provider-turn")
                        .iter()
                        .any(|turn| {
                            turn["runAttemptId"] == attempt.id.0
                                && turn["nodeId"] == attempt.root_node_id.0
                                && turn["providerThreadId"] == attempt.provider_thread_id.0
                                && attempt
                                    .provider_turn_id
                                    .as_ref()
                                    .is_some_and(|id| turn["id"] == id.0)
                        })
            })
    });
    let Some(accepted) = accepted.filter(|previous| {
        selection_transition(
            provider["driver"].as_str().unwrap_or_default(),
            &previous.model_selection,
            &run.model_selection,
        ) == SelectionTransition::ApplyOnNextTurn
    }) else {
        return Ok(None);
    };
    let Some(session_id) = provider["providerSessionId"].as_str() else {
        return Ok(None);
    };
    let session = super::super::task::records(projection, "provider-session")
        .iter()
        .find(|s| s["id"] == session_id)
        .cloned();
    let session = match session {
        Some(session) => Some(session),
        None => kernel.store.read(|conn| {
            use rusqlite::OptionalExtension;
            let raw: Option<String> = conn
                .query_row(
                    "SELECT json_extract(envelope_json,'$.event.payload')
                 FROM orchestration_events WHERE stream_id=?1
                 AND event_type='provider-session.attached'
                 AND json_extract(envelope_json,'$.event.payload.id')=?2
                 ORDER BY sequence DESC LIMIT 1",
                    rusqlite::params![projection.thread.id.0, session_id],
                    |row| row.get(0),
                )
                .optional()?;
            raw.map(|raw| Ok(serde_json::from_str::<Value>(&raw)?))
                .transpose()
        })?,
    };
    let Some(session) =
        session.filter(|s| s["providerInstanceId"] == accepted.provider_instance_id.0)
    else {
        return Ok(None);
    };
    let same_checkout = session["cwd"].as_str().is_some_and(|saved| {
        saved == cwd
            || std::fs::canonicalize(saved)
                .ok()
                .zip(std::fs::canonicalize(cwd).ok())
                .is_some_and(|(saved, current)| saved == current)
    });
    Ok(same_checkout.then(|| native.to_owned()))
}

/// ProviderTurnStartService.ts retains ready handoffs from failed/interrupted
/// starts on the same provider thread. Consumption stays on the original run;
/// retry delivery must not create another transfer or reuse an uncertain native.
/// A bare `/compact` (no attachments) is native maintenance: it must not carry
/// imported history, and a handoff prepared for it is still owed afterwards.
pub(super) fn is_compaction(message: &Value) -> bool {
    message["attachments"].as_array().is_none_or(Vec::is_empty)
        && message["text"].as_str().is_some_and(|text| {
            // JS `String.trim`: Unicode space + BOM, not NEL.
            text.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}')
                .eq_ignore_ascii_case("/compact")
        })
}

pub(super) fn run_is_compaction(projection: &ThreadProjection, run: &OrchestrationV2Run) -> bool {
    super::super::task::records(projection, "message")
        .iter()
        .any(|message| message["id"] == run.user_message_id.0 && is_compaction(message))
}

/// T3 `hasConversation`: a user message other than a bare `/compact`.
pub(super) fn has_conversation(projection: &ThreadProjection) -> bool {
    super::super::task::records(projection, "message")
        .iter()
        .any(|message| message["role"] == "user" && !is_compaction(message))
}

fn handoff_for_run(
    handoff: &Value,
    projection: &ThreadProjection,
    run: &OrchestrationV2Run,
) -> bool {
    handoff["status"] == "ready"
        && (handoff["targetRunId"] == run.id.0
            || (run
                .provider_thread_id
                .as_ref()
                .is_some_and(|p| handoff["toProviderThreadId"] == p.0)
                && projection.runs.iter().any(|source| {
                    handoff["targetRunId"] == source.id.0
                        && (matches!(
                            source.status,
                            OrchestrationV2RunStatus::Failed
                                | OrchestrationV2RunStatus::Interrupted
                        )
                            // A completed `/compact` deferred its handoff and
                            // never delivered it; the next turn still owes it.
                            || (source.status == OrchestrationV2RunStatus::Completed
                                && handoff["delivery"].is_null()
                                && run_is_compaction(projection, source)))
                })))
}

use zeron_proto::provider_instance::ProviderInstanceId;

async fn persist(kernel: &Kernel, transfer: &Value, handoff: Option<&Value>) -> Result<()> {
    let target: ThreadId = serde_json::from_value(transfer["targetThreadId"].clone())?;
    let receipt = kernel
        .transfer_command(
            &target,
            CommandId(format!("transfer-update:{}", uuid::Uuid::new_v4())),
            super::TransferOperation::Update {
                transfer: Box::new(transfer.clone()),
                handoff: handoff.cloned().map(Box::new),
            },
        )
        .await?;
    if receipt.status == crate::orchestration::ReceiptStatus::Rejected {
        return Err(Error::Invariant(receipt.error.unwrap_or_default()));
    }
    Ok(())
}

/// Generic injection/persistence seam mirrors ContextHandoffDelivery.ts tests.
#[async_trait]
pub trait HandoffDelivery: Send + Sync {
    async fn inject(&self, native: &str, messages: &[Value], context: &str) -> Result<bool>;
    async fn persist(&self, handoff: &Value) -> Result<()>;
    /// Retain the exact bounded inline selection and its input-attempt fence.
    /// A fresh provider has no native ID yet; readiness cannot settle it.
    async fn stage_inline(
        &self,
        _handoff: &Value,
        _items: &[Value],
        _omitted: &[Value],
    ) -> Result<()> {
        Ok(())
    }
}

pub struct DeliveredContext {
    pub context: String,
    pub handoffs: Vec<Value>,
}

impl DeliveredContext {
    /// Call only after native input acceptance. This is not a task-result ack.
    pub async fn accepted(&self, delivery: &dyn HandoffDelivery) -> Result<()> {
        for handoff in &self.handoffs {
            let mut handoff = handoff.clone();
            if !handoff["delivery"].is_null() {
                handoff["delivery"]["status"] = json!("inline");
            }
            delivery.persist(&handoff).await?;
        }
        Ok(())
    }
}

pub async fn deliver_handoffs(
    handoffs: &[Value],
    native_id: Option<&str>,
    app_thread: &ThreadId,
    budget: usize,
    already_delivered: &std::collections::BTreeSet<String>,
    native_injection: bool,
    defer_inline: bool,
    delivery: &dyn HandoffDelivery,
) -> Result<DeliveredContext> {
    let pending: Vec<_> = handoffs
        .iter()
        .filter(|h| {
            native_id.is_none()
                || h["delivery"]["nativeThreadId"].as_str() != native_id
                || h["delivery"]["status"] == "pending"
        })
        .collect();
    if pending.is_empty() || (defer_inline && !native_injection) {
        return Ok(DeliveredContext {
            context: String::new(),
            handoffs: vec![],
        });
    }
    let mut coverage = pending.iter().map(|h| format!("Context handoff ({}):\n{}",
        if h["strategy"] == "fork_delta_summary" {"merge_back / fork_delta_summary"} else {h["strategy"].as_str().unwrap_or("")},
        h["history"]["coverage"].as_str().map(str::to_owned).unwrap_or_else(||
            format!("From thread {}, runs {}-{}. Recover history with t3_thread_read, view=activity; paginate with afterPosition, and use itemId/textOffset for long items.",
                h["threadId"].as_str().unwrap_or(""),h["coveredRunOrdinals"]["from"],h["coveredRunOrdinals"]["to"]))
    )).collect::<Vec<_>>().join("\n");
    if context::history_cost(&[], &coverage) > 4_000.min(budget / 2) {
        let mut strategies = vec![];
        for h in &pending {
            let s = h["strategy"].as_str().unwrap_or("");
            if !strategies.contains(&s) {
                strategies.push(s)
            }
        }
        coverage = format!(
            "Context handoff ({}). {} handoff records; detailed coverage references omitted. Recover history with t3_thread_read({{threadId:\"{}\",view:\"activity\",limit:20,maxCharsPerItem:4000}}); paginate with afterPosition=nextPosition. Follow fork/handoff source references in activity. For long items use itemId and textOffset=nextTextOffset until null.",
            strategies.join(", "),
            pending.len(),
            app_thread
        );
    }
    let mut seen = already_delivered.clone();
    let messages: Vec<_> = pending
        .iter()
        .flat_map(|h| h["history"]["messages"].as_array().into_iter().flatten())
        .filter(|m| {
            m["itemId"]
                .as_str()
                .is_some_and(|id| seen.insert(id.into()))
        })
        .cloned()
        .collect();
    let old = pending
        .iter()
        .filter(|h| h.get("history").is_none())
        .filter_map(|h| h["summaryText"].as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let full = format!("{coverage}\n{old}");
    if !old.is_empty() && context::history_cost(&[], &full) + 512 <= budget {
        coverage = full
    }
    let selected = context::select_history(
        &messages,
        &coverage,
        pending
            .iter()
            .map(|h| h["history"]["omittedItems"].as_u64().unwrap_or(0) as usize)
            .sum(),
        budget,
    );
    if context::history_cost(&selected.messages, &selected.context) > budget {
        if defer_inline {
            return Ok(DeliveredContext {
                context: String::new(),
                handoffs: vec![],
            });
        }
        return Err(Error::Invariant(context::BUDGET_ERROR.into()));
    }
    if native_injection
        && native_id.is_some()
        && pending.iter().any(|h| {
            h["delivery"]["nativeThreadId"].as_str() == native_id
                && h["delivery"]["status"] == "pending"
        })
    {
        return Err(Error::Invariant(context::UNCERTAIN_ERROR.into()));
    }
    let mut durable = Vec::new();
    let mut inline = Vec::new();
    for h in &pending {
        let mut h = (*h).clone();
        let candidates = h["history"]["messages"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut omitted = h["history"]["omittedItemIds"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        omitted.extend(
            candidates
                .iter()
                .filter(|m| {
                    selected
                        .omitted_item_ids
                        .iter()
                        .any(|id| m["itemId"] == *id)
                })
                .map(|m| m["itemId"].clone()),
        );
        let items: Vec<_> = selected
            .messages
            .iter()
            .filter(|m| candidates.iter().any(|c| c["itemId"] == m["itemId"]))
            .map(|m| m["itemId"].clone())
            .collect();
        if let Some(native) = native_id {
            h["delivery"] = json!({"nativeThreadId":native,"status":"pending","itemIds":items,"omittedItemIds":omitted});
        }
        delivery.persist(&h).await?;
        inline.push((h.clone(), items, omitted));
        durable.push(h);
    }
    if native_injection
        && let Some(native) = native_id
        && delivery
            .inject(native, &selected.messages, &selected.context)
            .await?
    {
        for h in &mut durable {
            h["delivery"]["status"] = json!("injected");
            delivery.persist(h).await?;
        }
        return Ok(DeliveredContext {
            context: String::new(),
            handoffs: vec![],
        });
    }
    if defer_inline {
        for h in pending {
            delivery.persist(h).await?
        }
        return Ok(DeliveredContext {
            context: String::new(),
            handoffs: vec![],
        });
    }
    for (handoff, items, omitted) in inline {
        delivery.stage_inline(&handoff, &items, &omitted).await?;
    }
    Ok(DeliveredContext {
        context: context::render_history(&selected.messages, &selected.context),
        handoffs: durable,
    })
}

struct EngineDelivery<'a> {
    kernel: &'a Kernel,
    run: &'a OrchestrationV2Run,
    transfers: Vec<Value>,
    harness: &'a dyn Harness,
}
#[async_trait]
impl HandoffDelivery for EngineDelivery<'_> {
    async fn inject(&self, native: &str, messages: &[Value], context: &str) -> Result<bool> {
        match self.harness.session_lifecycle() {
            Some(lifecycle) => lifecycle
                .inject_history(native, messages, context)
                .await
                .map_err(|e| Error::Invariant(e.to_string())),
            None => Ok(false),
        }
    }
    async fn persist(&self, handoff: &Value) -> Result<()> {
        let transfer = self
            .transfers
            .iter()
            .find(|t| t["id"] == handoff["transferId"])
            .ok_or_else(|| Error::Invariant("Handoff transfer missing.".into()))?;
        persist(self.kernel, transfer, Some(handoff)).await
    }
    async fn stage_inline(
        &self,
        handoff: &Value,
        items: &[Value],
        omitted: &[Value],
    ) -> Result<()> {
        self.kernel.store.write(|conn| {
            let native = handoff["delivery"]["nativeThreadId"].as_str();
            let status = if native.is_some() { "pending" } else { "inline_prepared" };
            conn.execute("INSERT INTO orchestration_transfer_delivery(transfer_id,target_run_id,native_thread_id,status,payload_json)
                VALUES(?1,?2,?3,?4,?5) ON CONFLICT(transfer_id) DO UPDATE SET native_thread_id=excluded.native_thread_id,status=excluded.status,payload_json=excluded.payload_json",
                rusqlite::params![handoff["transferId"].as_str(),handoff["targetRunId"].as_str(),
                    native,status,json!({"status":status,"nativeThreadId":native,"itemIds":items,"omittedItemIds":omitted,
                        "preparedRunId":self.run.id,"preparedAttemptId":self.run.active_attempt_id,
                        "preparedRootNodeId":self.run.root_node_id,"preparedProviderThreadId":self.run.provider_thread_id}).to_string()])?;
            Ok(())
        })
    }
}

/// A selection-changing restart replaces the provider generation inside one
/// logical run. Returns the attempt that first ran on the current generation
/// and the interrupted attempt just before it (a different generation), or None
/// when the run has stayed on its original generation.
fn restart_origin<'a>(
    projection: &'a ThreadProjection,
    run: &OrchestrationV2Run,
) -> Option<(&'a OrchestrationV2RunAttempt, &'a OrchestrationV2RunAttempt)> {
    let attempts = |ordinal: i64| {
        projection
            .attempts
            .iter()
            .find(|a| a.run_id == run.id && a.attempt_ordinal == ordinal)
    };
    let current = projection
        .attempts
        .iter()
        .find(|a| Some(&a.id) == run.active_attempt_id.as_ref() && a.run_id == run.id)?;
    let mut first = current;
    while first.attempt_ordinal > 1
        && first.reason == OrchestrationV2RunAttemptReason::SteeringRestart
        && let Some(previous) = attempts(first.attempt_ordinal - 1)
        && previous.provider_thread_id == first.provider_thread_id
    {
        first = previous;
    }
    if first.attempt_ordinal == 1
        || first.reason != OrchestrationV2RunAttemptReason::SteeringRestart
    {
        return None;
    }
    Some((first, attempts(first.attempt_ordinal - 1)?))
}

/// Whether a started run is a generation the planner rebuilt from portable
/// context (provider handoff to this run, or a user reset), rather than one
/// continuing an accepted native session. Only then may the engine be told
/// not to resume whatever it remembers for the chat; a run with no positive
/// evidence (for example the first canonical turn of an adopted chat) keeps
/// the engine's own continuity.
pub(crate) fn fresh_native_start(
    kernel: &Kernel,
    thread: &ThreadId,
    run: &OrchestrationV2Run,
    resume: Option<&str>,
) -> Result<bool> {
    if resume.is_some() {
        return Ok(false);
    }
    let projection = kernel
        .store
        .thread(thread)?
        .ok_or_else(|| Error::Invariant("Run thread missing after preparation.".into()))?;
    if super::super::queue::session_control::fresh_after_reset(&projection, run) {
        return Ok(true);
    }
    let to = run.provider_thread_id.as_ref().map(|id| id.0.as_str());
    let handoffs = kernel.store.thread_transfers(thread)?;
    Ok(handoffs.iter().any(|t| {
        t["type"] == "provider_handoff"
            && t["targetRunId"] == run.id.0
            && super::super::task::records(&projection, "context-handoff")
                .iter()
                .any(|h| h["transferId"] == t["id"] && h["toProviderThreadId"].as_str() == to)
    }))
}

/// Delegated-completion and notification runs are automatic deliveries; they
/// never consume context the user is waiting to send (a pending merge-back).
fn automatic_run(projection: &ThreadProjection, run: &OrchestrationV2Run) -> bool {
    super::super::task::records(projection, "message")
        .iter()
        .find(|m| m["id"] == run.user_message_id.0)
        .is_some_and(|m| m.get("delegatedCompletion").is_some() || m.get("notification").is_some())
}

/// Normal start integration. `request.prompt` retains current input verbatim;
/// imported context is prepended only to native input, not historical transcript.
pub async fn prepare_run(
    kernel: &Kernel,
    thread: &ThreadId,
    run: &OrchestrationV2Run,
    request: &mut RunRequest,
    harness: &dyn Harness,
    capabilities: &OrchestrationV2ProviderCapabilities,
    mcp: SessionMcpContext,
) -> Result<()> {
    let projection = kernel
        .store
        .thread(thread)?
        .ok_or_else(|| Error::Invariant("Transfer target missing.".into()))?;
    let compaction = run_is_compaction(&projection, run);
    if compaction && !has_conversation(&projection) {
        return Err(Error::Invariant(
            "Start a conversation before compacting this thread.".into(),
        ));
    }
    let all = kernel.store.thread_transfers(thread)?;
    super::ensure_start_allowed(&all, thread)?;
    // Queued runs may be admitted while a merge-back waits. The first
    // non-automatic run to actually start consumes it; a cancelled queued run
    // never reaches here, and an automatic delivery leaves it pending.
    let automatic = automatic_run(&projection, run);
    let claims_pending =
        |t: &Value| t["status"] == "pending" && !(automatic && t["type"] == "merge_back");
    let needs_full_switch = all
        .iter()
        .any(|t| t["type"] == "merge_back" && claims_pending(t) && t["targetThreadId"] == thread.0);
    let relevant: Vec<_> = all
        .into_iter()
        .filter(|t| {
            t["targetThreadId"] == thread.0
                && matches!(t["type"].as_str(), Some("fork" | "merge_back"))
                && (claims_pending(t) || t["targetRunId"] == run.id.0)
        })
        .collect();
    let mut handoffs = vec![];
    let mut durable_transfers = vec![];
    let mut target_native = resumable_native(kernel, &projection, run, &request.cwd)?;
    // A native thread the harness can no longer resume (a vanished session
    // file) is as good as none: the planner rebuilds the conversation from
    // portable context rather than letting the run start a blank session.
    if let (Some(native), Some(lifecycle)) = (&target_native, harness.session_lifecycle())
        && !lifecycle
            .can_resume_now(native, &request.cwd)
            .await
            .map_err(|e| Error::Invariant(e.to_string()))?
    {
        tracing::warn!(
            thread = %thread.0,
            "native session {native} is no longer available; continuing from portable context"
        );
        target_native = None;
    }
    let target_provider = provider_for_run(&projection, run).unwrap_or(Value::Null);
    if request.resume.is_none()
        || target_native.is_some()
        || provider_for_run(&projection, run)
            .is_some_and(|p| p["nativeThreadRef"]["nativeId"].is_string())
    {
        request.resume = target_native.clone();
    }
    let restart = restart_origin(&projection, run);
    let earlier = projection
        .runs
        .iter()
        .filter(|r| {
            r.ordinal < run.ordinal
                && super::forkable(&r.status)
                && !super::super::queue::session_control::never_started(r)
        })
        .max_by_key(|r| r.ordinal);
    // A restart that moved the run onto another generation hands off from the
    // run itself, partial output included. Otherwise the previous finished run
    // is the source, and only when the selection cannot ride the native session.
    let source = if restart.is_some() {
        Some(run)
    } else {
        earlier.filter(|previous| {
            previous.provider_instance_id != run.provider_instance_id
                // A user reset closed the previous conversation: rebuild fully.
                || (previous.provider_thread_id != run.provider_thread_id
                    && provider_for_run(&projection, previous)
                        .is_some_and(|p| p["status"] == "closed"))
                // The fresh generation has no accepted native conversation yet
                // (its first run was cancelled or failed before one existed).
                || super::super::queue::session_control::fresh_generation(&projection, run)
                || selection_transition(
                    target_provider["driver"].as_str().unwrap_or_default(),
                    &previous.model_selection,
                    &run.model_selection,
                ) == SelectionTransition::CreateWithHandoff
                || (provider_for_run(&projection, run)
                    .is_some_and(|p| p["nativeThreadRef"]["nativeId"].is_string())
                    && target_native.is_none()
                    && request.resume.is_none())
        })
    };
    if let Some(previous) = source {
        // One transfer per provider generation of a run, never per run: the
        // earlier attempt's handoff belongs to the generation it fed.
        let (id, command_id) = match restart {
            Some((first, _)) => (
                format!(
                    "provider-handoff:{}:attempt:{}",
                    encode_component(&run.id.0),
                    first.attempt_ordinal
                ),
                format!(
                    "provider-handoff:{}:attempt:{}",
                    run.id.0, first.attempt_ordinal
                ),
            ),
            None => (
                format!("provider-handoff:{}", encode_component(&run.id.0)),
                format!("provider-handoff:{}", run.id.0),
            ),
        };
        let source_instance = restart
            .map(|(_, interrupted)| interrupted.provider_instance_id.clone())
            .unwrap_or_else(|| previous.provider_instance_id.clone());
        let existing = kernel
            .store
            .thread_transfers(thread)?
            .into_iter()
            .find(|t| t["id"] == id);
        let (transfer, handoff) = if let Some(transfer) = existing {
            let handoff = super::super::task::records(&projection, "context-handoff")
                .iter()
                .find(|h| h["transferId"] == id)
                .cloned()
                .ok_or_else(|| Error::Invariant("Provider handoff missing.".into()))?;
            (transfer, handoff)
        } else {
            let seen = if needs_full_switch || target_native.is_none() {
                None
            } else {
                projection
                    .runs
                    .iter()
                    .filter(|r| {
                        r.ordinal < run.ordinal
                            && r.provider_instance_id == run.provider_instance_id
                            && r.provider_thread_id == run.provider_thread_id
                            && super::forkable(&r.status)
                            && projection.attempts.iter().any(|a| {
                                a.run_id == r.id
                                    && a.native_thread_id.as_ref().map(String::as_str)
                                        == target_native.as_deref()
                            })
                    })
                    .max_by_key(|r| r.ordinal)
            };
            let from = seen.map(|r| r.ordinal + 1).unwrap_or(1);
            let mut items = local_items(&projection, previous.ordinal);
            items.retain(|i| {
                i["runId"]
                    .as_str()
                    .and_then(|id| projection.runs.iter().find(|r| r.id.0 == id))
                    .map(|r| r.ordinal >= from)
                    .unwrap_or(seen.is_none() && i["runId"].is_null())
            });
            if restart.is_some() {
                // The prompt being delivered is the run's current input, not history.
                items.retain(|i| {
                    i["messageId"] != run.user_message_id.0 && i["id"] != run.user_message_id.0
                });
            }
            if seen.is_none() {
                items.splice(0..0, super::inherited_items(&kernel.store, &projection)?);
            }
            let coverage = context::coverage(&thread.0, from, previous.ordinal, &items);
            let messages: Vec<_> = items
                .iter()
                .filter_map(context::historical_message)
                .collect();
            let selected = context::select_history(&messages, &coverage, 0, context::token_cap());
            let handoff_id = format!("handoff:{}", encode_component(&id));
            let now = iso(crate::now_ms())?;
            let handoff = json!({"id":handoff_id,"transferId":id,"threadId":thread,"targetRunId":run.id,
                "fromProviderThreadIds":projection.runs.iter().filter(|r| r.ordinal>=from && r.ordinal<=previous.ordinal)
                    .filter_map(|r| r.provider_thread_id.as_ref()).collect::<Vec<_>>(),
                "toProviderThreadId":run.provider_thread_id,"coveredRunOrdinals":{"from":from,"to":previous.ordinal},
                "strategy":if seen.is_none() {"full_thread_summary"} else {"delta_since_target_last_seen"},
                "status":"ready","summaryMessageId":null,"summaryText":"",
                "history":{"messages":selected.messages,"coverage":coverage,"omittedItems":selected.omitted_items,"omittedItemIds":selected.omitted_item_ids},
                "createdByProviderInstanceId":null,"createdAt":now,"updatedAt":now});
            let transfer = json!({"id":id,"type":"provider_handoff","sourceThreadId":thread,"targetThreadId":thread,
                "sourcePoint":super::canonical_point(&projection,previous),"basePoint":seen.map(|r| super::canonical_point(&projection,r)),
                "sourceProviderInstanceId":source_instance,"targetProviderInstanceId":run.provider_instance_id,
                "targetRunId":run.id,"status":"consumed","resolution":{"strategy":if seen.is_none() {"portable_context"} else {"delta_context"},"contextHandoffId":handoff_id},
                "createdBy":"system","error":null,"createdAt":now,"updatedAt":now,"consumedAt":now});
            let receipt = kernel
                .transfer_command(
                    thread,
                    CommandId(command_id),
                    super::TransferOperation::CreateHandoff {
                        transfer: Box::new(transfer.clone()),
                        handoff: Box::new(handoff.clone()),
                    },
                )
                .await?;
            if receipt.status == crate::orchestration::ReceiptStatus::Rejected {
                return Err(Error::Invariant(receipt.error.unwrap_or_default()));
            }
            (transfer, handoff)
        };
        handoffs.push(handoff);
        durable_transfers.push(transfer);
        // A delta is valid only on the exact accepted target conversation.
        // Returning to a provider must not start a fresh process with only its
        // missed delta, nor resume the provider we have just left.
        request.resume = target_native.clone();
    }
    for mut transfer in relevant {
        // Restart after logical consumption belongs to this run, never to a
        // later run. A provider-start recovery must not consume it twice.
        if transfer["targetRunId"].is_string() && transfer["targetRunId"] != run.id.0 {
            return Err(Error::Invariant(
                "Transfer already belongs to another run.".into(),
            ));
        }
        let source: ThreadId = serde_json::from_value(transfer["sourceThreadId"].clone())?;
        let source_projection = kernel
            .store
            .thread(&source)?
            .ok_or_else(|| Error::Invariant("Transfer source missing.".into()))?;
        let source_run = source_projection
            .runs
            .iter()
            .find(|r| transfer["sourcePoint"]["runId"] == r.id.0)
            .ok_or_else(|| {
                Error::Invariant(format!(
                    "Pending merge-back transfer {} has no resolvable source run.",
                    transfer["id"].as_str().unwrap_or("")
                ))
            })?;
        let source_provider =
            provider_for_run(&source_projection, source_run).ok_or_else(|| {
                Error::Invariant(format!(
                    "Pending {} transfer {} has no resolvable source provider thread.",
                    if transfer["type"] == "fork" {
                        "fork"
                    } else {
                        "merge-back"
                    },
                    transfer["id"].as_str().unwrap_or("")
                ))
            })?;
        let receipt = kernel.store.read(|conn| {
            use rusqlite::OptionalExtension;
            let v: Option<String> = conn
                .query_row(
                    "SELECT payload_json FROM orchestration_transfer_delivery WHERE transfer_id=?1",
                    [transfer["id"].as_str().unwrap()],
                    |r| r.get(0),
                )
                .optional()?;
            v.map(|v| Ok(serde_json::from_str::<Value>(&v)?))
                .transpose()
        })?;
        if transfer["resolution"]["strategy"] == "native_fork" {
            request.resume = transfer["resolution"]["providerThreadRef"]["nativeId"]
                .as_str()
                .map(str::to_owned);
            continue;
        }
        if receipt
            .as_ref()
            .is_some_and(|r| r["status"] == "fork_pending")
        {
            return Err(Error::Invariant(
                "Native fork acceptance is uncertain; inspect transfers before retrying.".into(),
            ));
        }
        let next_run = source_projection
            .runs
            .iter()
            .filter(|r| {
                r.ordinal > source_run.ordinal
                    && r.provider_thread_id == source_run.provider_thread_id
            })
            .min_by_key(|r| r.ordinal);
        let next_turn = next_run.and_then(|r| {
            super::canonical_point(&source_projection, r)["providerTurnRef"]["nativeId"]
                .as_str()
                .map(str::to_owned)
        });
        // A native cursor forks atomically at the boundary. A legacy source
        // turn (no cursor) can still fork natively when the adapter can revert
        // a head fork and every later turn is settled and countable; otherwise
        // use portable context, never a moving head or an OpenCode boundary
        // whose later cursor is unknown.
        let legacy_rollback = harness
            .session_lifecycle()
            .filter(|lifecycle| lifecycle.supports_fork_rollback())
            .and_then(|_| legacy_rollback_turns(&source_projection, source_run));
        let stable_native_boundary = (transfer["sourcePoint"]["providerTurnRef"]["nativeId"]
            .is_string()
            && (next_run.is_none() || next_turn.is_some()))
            || (!transfer["sourcePoint"]["providerTurnRef"]["nativeId"].is_string()
                && legacy_rollback.is_some());
        let fork_request = (transfer["type"] == "fork"
            && stable_native_boundary
            && native_fork_eligible(
                &transfer,
                source_run,
                &run.provider_instance_id,
                capabilities,
            ))
        .then(|| NativeForkRequest {
            source_thread_id: transfer["sourcePoint"]["providerThreadRef"]["nativeId"]
                .as_str()
                .unwrap_or("")
                .into(),
            source_turn_id: transfer["sourcePoint"]["providerTurnRef"]["nativeId"]
                .as_str()
                .map(str::to_owned),
            source_next_turn_id: next_turn,
            rollback_turns: legacy_rollback,
            cwd: request.cwd.clone(),
            model: run.model_selection.model.clone(),
            runtime_mode: request.runtime_mode,
            interaction_mode: request.interaction_mode,
            mcp: mcp.clone(),
        });
        // A definite "cannot fork this boundary" routes to portable context
        // before anything is recorded as in flight.
        let native_fork = match fork_request {
            Some(fork_request) => {
                let lifecycle = harness
                    .session_lifecycle()
                    .ok_or_else(|| Error::Invariant("Native fork adapter unavailable.".into()))?;
                lifecycle
                    .can_fork_now(&fork_request)
                    .await
                    .map_err(|e| Error::Invariant(e.to_string()))?
                    .then_some((lifecycle, fork_request))
            }
            None => None,
        };
        if let Some((lifecycle, fork_request)) = native_fork {
            kernel.store.write(|tx| {
                let marker = json!({"status":"fork_pending","targetRunId":run.id});
                tx.execute("INSERT INTO orchestration_transfer_delivery(transfer_id,target_run_id,status,payload_json) VALUES(?1,?2,'fork_pending',?3)",
                    rusqlite::params![transfer["id"].as_str().unwrap(),run.id.0,marker.to_string()])?;
                Ok(())
            })?;
            let native = lifecycle
                .fork_thread(fork_request)
                .await
                .map_err(|e| Error::Invariant(e.to_string()))?;
            transfer["resolution"] = json!({"strategy":"native_fork","providerThreadRef":{
                "driver":source_provider["driver"],"nativeId":native,"strength":"strong"
            }});
            transfer["status"] = json!("consumed");
            transfer["consumedAt"] = json!(iso(crate::now_ms())?);
            transfer["targetRunId"] = json!(run.id);
            transfer["targetProviderInstanceId"] = json!(run.provider_instance_id);
            transfer["error"] = Value::Null;
            transfer["updatedAt"] = json!(iso(crate::now_ms())?);
            persist(kernel, &transfer, None).await?;
            request.resume = Some(native);
            continue;
        }
        let handoff_id = format!(
            "handoff:{}",
            encode_component(transfer["id"].as_str().unwrap())
        );
        let existing = super::super::task::records(&projection, "context-handoff")
            .iter()
            .find(|h| h["id"] == handoff_id);
        let handoff = if let Some(existing) = existing {
            existing.clone()
        } else {
            let access = KernelThreadAccess {
                kernel: kernel.clone(),
            };
            let items = if transfer["type"] == "merge_back" {
                local_items(&source_projection, source_run.ordinal)
            } else {
                access.history(&source, &source_run.id).await?
            };
            let messages: Vec<_> = items
                .iter()
                .filter_map(context::historical_message)
                .collect();
            let from = if transfer["type"] == "merge_back" {
                source_projection
                    .runs
                    .first()
                    .map(|r| r.ordinal)
                    .unwrap_or(1)
            } else {
                1
            };
            let coverage = context::coverage(&source.0, from, source_run.ordinal, &items);
            let selected = context::select_history(&messages, &coverage, 0, context::token_cap());
            json!({"id":handoff_id,"transferId":transfer["id"],"threadId":thread,"targetRunId":run.id,
                "fromProviderThreadIds":[source_provider["id"]],"toProviderThreadId":run.provider_thread_id,
                "coveredRunOrdinals":{"from":from,"to":source_run.ordinal},
                "strategy":if transfer["type"] == "merge_back" {"fork_delta_summary"} else {"full_thread_summary"},
                "status":"ready","summaryMessageId":null,"summaryText":"",
                "history":{"messages":selected.messages,"coverage":coverage,"omittedItems":selected.omitted_items,"omittedItemIds":selected.omitted_item_ids},
                "createdByProviderInstanceId":null,"createdAt":iso(crate::now_ms())?,"updatedAt":iso(crate::now_ms())?})
        };
        if transfer["status"] != "consumed" {
            transfer["resolution"] = json!({"strategy":if transfer["type"] == "merge_back" {"fork_delta_context"} else {"portable_context"},"contextHandoffId":handoff_id});
            transfer["targetRunId"] = json!(run.id);
            transfer["targetProviderInstanceId"] = json!(run.provider_instance_id);
            transfer["status"] = json!("consumed");
            transfer["consumedAt"] = json!(iso(crate::now_ms())?);
            transfer["error"] = Value::Null;
            transfer["updatedAt"] = json!(iso(crate::now_ms())?);
        }
        persist(kernel, &transfer, Some(&handoff)).await?;
        handoffs.push(handoff);
        durable_transfers.push(transfer);
    }
    for handoff in super::super::task::records(&projection, "context-handoff")
        .iter()
        .filter(|h| handoff_for_run(h, &projection, run))
    {
        if handoffs.iter().any(|h| h["id"] == handoff["id"]) {
            continue;
        }
        if let Some(transfer) = kernel
            .store
            .thread_transfers(thread)?
            .into_iter()
            .find(|t| t["id"] == handoff["transferId"])
        {
            // A provider handoff fed the generation it was prepared for. After a
            // restart onto another generation the run's own handoff covers the
            // history again; re-injecting the earlier one would duplicate it.
            if transfer["type"] == "provider_handoff"
                && handoff["toProviderThreadId"].as_str()
                    != run.provider_thread_id.as_ref().map(|id| id.0.as_str())
            {
                continue;
            }
            handoffs.push(handoff.clone());
            durable_transfers.push(transfer);
        }
    }
    if let Some((transfer, handoff)) = super::retry::prepare(
        kernel,
        &projection,
        run,
        request.resume.as_deref(),
        &handoffs,
    )
    .await?
        && !handoffs.iter().any(|h| h["id"] == handoff["id"])
    {
        handoffs.push(handoff);
        durable_transfers.push(transfer);
    }
    if handoffs.is_empty() {
        return Ok(());
    }
    let native = request.resume.as_deref();
    // Refuse resuming an ambiguous receipt even on a text-only adapter.
    if native.is_some()
        && handoffs.iter().any(|h| {
            h["delivery"]["nativeThreadId"].as_str() == native
                && h["delivery"]["status"] == "pending"
        })
    {
        return Err(Error::Invariant(context::UNCERTAIN_ERROR.into()));
    }
    let usage = super::super::task::records(&projection, "provider-thread")
        .iter()
        .find(|p| p["nativeThreadRef"]["nativeId"].as_str() == native)
        .and_then(|p| {
            let (measured, selection) = context::latest_native_context_usage(&projection, p)?;
            let previous = if selection == run.model_selection {
                serde_json::from_value::<context::ContextUsage>(p["contextUsage"].clone())
                    .ok()
                    .unwrap_or(measured)
            } else {
                measured
            };
            context::context_usage_for_handoff(
                native.is_some(),
                selection == run.model_selection,
                false,
                Some(&previous),
                None,
            )
        });
    let attachments: Vec<_> = request
        .attachments
        .iter()
        .map(|_| json!({"type":"image"}))
        .collect();
    let settled: Vec<_> = super::super::task::records(&projection, "context-handoff")
        .iter()
        .filter(|h| {
            native.is_some()
                && h["toProviderThreadId"].as_str()
                    == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                && h["delivery"]["nativeThreadId"].as_str() == native
                && matches!(
                    h["delivery"]["status"].as_str(),
                    Some("injected" | "inline")
                )
        })
        .collect();
    let delivered_ids: std::collections::BTreeSet<String> = settled
        .iter()
        .flat_map(|h| h["delivery"]["itemIds"].as_array().into_iter().flatten())
        .filter_map(|id| id.as_str().map(str::to_owned))
        .collect();
    let native_provider = super::super::task::records(&projection, "provider-thread")
        .iter()
        .find(|p| {
            native.is_some()
                && p["id"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                && p["nativeThreadRef"]["nativeId"].as_str() == native
        });
    let native_estimate = if usage.is_some() || native_provider.is_none() {
        0
    } else {
        let p = native_provider.unwrap();
        let accepted_runs: std::collections::BTreeSet<_> = projection
            .attempts
            .iter()
            .filter(|a| {
                let value = json!(a);
                value["providerThreadId"] == p["id"]
                    && value["nativeThreadId"].as_str() == native
                    && super::super::task::records(&projection, "provider-turn")
                        .iter()
                        .any(|t| t["runAttemptId"] == a.id.0 && t["nodeId"] == value["rootNodeId"])
            })
            .map(|a| a.run_id.0.as_str())
            .collect();
        local_items(&projection, run.ordinal.saturating_sub(1))
            .iter()
            .filter(|i| {
                i["runId"]
                    .as_str()
                    .is_some_and(|id| accepted_runs.contains(id))
                    || i["id"]
                        .as_str()
                        .is_some_and(|id| delivered_ids.contains(id))
            })
            .filter_map(context::historical_message)
            .map(|m| m["text"].as_str().map(str::len).unwrap_or(0))
            .sum()
    };
    let budget = context::handoff_budget(
        context::token_cap(),
        &request.prompt,
        &attachments,
        usage.as_ref(),
        native_estimate,
        declared_model_window(harness, request),
    );
    let delivery = EngineDelivery {
        kernel,
        run,
        transfers: durable_transfers,
        harness,
    };
    let prepared = deliver_handoffs(
        &handoffs,
        native,
        thread,
        budget,
        &delivered_ids,
        harness.session_lifecycle().is_some(),
        // `/compact` must compact the native conversation, not an imported
        // preamble: inline delivery waits for the next ordinary turn.
        compaction,
        &delivery,
    )
    .await?;
    if !prepared.context.is_empty() {
        request.prompt = format!("{}\n\n{}", prepared.context, request.prompt);
    }
    Ok(())
}

/// Terminal provider turns after the source run's turn on its provider thread:
/// the revert count for a legacy (cursor-less) native fork. None when the
/// boundary turn is unknown or any later turn is still live, because a count
/// over a moving head would cut the wrong place.
fn legacy_rollback_turns(
    projection: &ThreadProjection,
    source_run: &OrchestrationV2Run,
) -> Option<usize> {
    let turns = super::super::task::records(projection, "provider-turn");
    let attempt = source_run.active_attempt_id.as_ref()?;
    let boundary = turns.iter().find(|turn| turn["runAttemptId"] == attempt.0)?;
    let provider = &boundary["providerThreadId"];
    let boundary_ordinal = boundary["ordinal"].as_i64()?;
    let mut later = 0;
    for turn in turns
        .iter()
        .filter(|turn| turn["providerThreadId"] == *provider)
        .filter(|turn| turn["ordinal"].as_i64().is_some_and(|o| o > boundary_ordinal))
    {
        match turn["status"].as_str()? {
            "completed" | "interrupted" | "failed" | "cancelled" => later += 1,
            _ => return None,
        }
    }
    Some(later)
}

/// The target model's catalog-declared window, so a first handoff (no
/// occupancy telemetry yet) is bounded by the model it will actually reach
/// instead of a fixed default.
pub(super) fn declared_model_window(harness: &dyn Harness, request: &RunRequest) -> Option<usize> {
    let model = request.model.as_deref()?;
    harness
        .model_context_window(model, &request.model_options)
        .map(|window| window as usize)
        .filter(|window| *window > 0)
}

/// A provider acknowledgement / first native output confirms input acceptance.
/// SessionStarted alone does not. Persist the
/// inline receipt under the same kernel transaction as native run observation.
pub(crate) fn accepted(
    conn: &rusqlite::Connection,
    projection: &ThreadProjection,
    command: &super::super::command::Command,
    plan: &mut super::super::command::Plan,
    run: &OrchestrationV2Run,
    native: &str,
    now: i64,
) -> Result<()> {
    for mut handoff in super::super::task::records(projection, "context-handoff")
        .iter()
        .filter(|h| {
            handoff_for_run(h, projection, run)
                && h["delivery"]["status"] != "injected"
                && h["history"]["messages"].is_array()
        })
        .cloned()
    {
        use rusqlite::OptionalExtension;
        let prepared: Option<String> = conn.query_row(
            "SELECT payload_json FROM orchestration_transfer_delivery WHERE transfer_id=?1 AND status IN ('inline_prepared','pending')",
            [handoff["transferId"].as_str()],|r| r.get(0)).optional()?;
        let prepared: Option<Value> = prepared.map(|v| serde_json::from_str(&v)).transpose()?;
        let Some(prepared) = prepared.filter(|p| {
            p["preparedRunId"] == run.id.0
                && p["preparedAttemptId"].as_str()
                    == run.active_attempt_id.as_ref().map(|id| id.0.as_str())
                && p["preparedRootNodeId"].as_str()
                    == run.root_node_id.as_ref().map(|id| id.0.as_str())
                && p["preparedProviderThreadId"].as_str()
                    == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                && (p["nativeThreadId"].is_null() || p["nativeThreadId"] == native)
        }) else {
            continue;
        };
        handoff["delivery"] = json!({"nativeThreadId":native,"status":"inline",
            "itemIds":prepared["itemIds"],"omittedItemIds":prepared["omittedItemIds"]});
        persist_receipt(conn, &handoff)?;
        plan.emit(command, "context-handoff.updated", &handoff, now)?;
    }
    Ok(())
}

pub(crate) fn persist_receipt(conn: &rusqlite::Connection, handoff: &Value) -> Result<()> {
    if handoff["delivery"].is_null() {
        return Ok(());
    }
    // Re-observing the same pending native receipt must retain the private
    // prepared-input fence/selection until its original acknowledgement. A
    // different native identity or settled status discards that staging data.
    conn.execute("INSERT INTO orchestration_transfer_delivery(transfer_id,target_run_id,native_thread_id,status,payload_json)
        VALUES(?1,?2,?3,?4,?5) ON CONFLICT(transfer_id) DO UPDATE SET
        payload_json=CASE
            WHEN orchestration_transfer_delivery.status='pending' AND excluded.status='pending'
              AND orchestration_transfer_delivery.native_thread_id IS excluded.native_thread_id
              AND json_extract(orchestration_transfer_delivery.payload_json,'$.preparedRunId') IS NOT NULL
            THEN json_patch(excluded.payload_json,orchestration_transfer_delivery.payload_json)
            ELSE excluded.payload_json END,
        native_thread_id=excluded.native_thread_id,status=excluded.status",
        rusqlite::params![handoff["transferId"].as_str(),handoff["targetRunId"].as_str(),
            handoff["delivery"]["nativeThreadId"].as_str(),handoff["delivery"]["status"].as_str(),handoff.to_string()])?;
    Ok(())
}
