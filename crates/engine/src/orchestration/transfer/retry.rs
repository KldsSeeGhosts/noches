//! ProviderTurnStartService's missed-input delta. An initialized native session
//! is not an accepted turn; recover only failed/interrupted untold root attempts.
use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{Value, json};
use zeron_proto::orchestration::*;

use crate::orchestration::{
    Error, Kernel, ReceiptStatus, Result,
    event::{encode_component, iso},
    projection::ThreadProjection,
    task::records,
};

use super::{TransferOperation, canonical_point, context, delivery::local_items};

pub(super) async fn prepare(
    kernel: &Kernel,
    projection: &ThreadProjection,
    run: &OrchestrationV2Run,
    native: Option<&str>,
    effective_handoffs: &[Value],
) -> Result<Option<(Value, Value)>> {
    let attempts: HashMap<_, _> = projection
        .attempts
        .iter()
        .map(|a| (a.id.0.as_str(), a))
        .collect();
    let accepted: HashSet<_> = records(projection, "provider-turn")
        .iter()
        .filter_map(|turn| {
            let attempt = attempts.get(turn["runAttemptId"].as_str()?)?;
            (turn["nodeId"] == attempt.root_node_id.0
                && turn["providerThreadId"] == attempt.provider_thread_id.0)
                .then_some(*attempt)
        })
        .map(|attempt| &attempt.id)
        .collect();
    // Once input was accepted, re-preparing the current run must not allocate
    // or change a historical snapshot (even if an older failure still exists).
    if run
        .active_attempt_id
        .as_ref()
        .is_some_and(|id| accepted.contains(id))
    {
        return Ok(None);
    }
    let id = format!("provider-retry-handoff:{}", encode_component(&run.id.0));
    if let Some(transfer) = kernel
        .store
        .thread_transfers(&run.thread_id)?
        .into_iter()
        .find(|t| t["id"] == id)
    {
        let handoff = records(projection, "context-handoff")
            .iter()
            .find(|h| h["transferId"] == id)
            .cloned()
            .ok_or_else(|| Error::Invariant("Provider retry handoff missing.".into()))?;
        return Ok(Some((transfer, handoff)));
    }
    let missed: Vec<_> = projection
        .runs
        .iter()
        .filter(|source| {
            source.ordinal < run.ordinal
                && source.provider_thread_id == run.provider_thread_id
                && matches!(
                    source.status,
                    OrchestrationV2RunStatus::Failed | OrchestrationV2RunStatus::Interrupted
                )
                && source
                    .active_attempt_id
                    .as_ref()
                    .is_some_and(|id| !accepted.contains(id))
        })
        .collect();
    let Some(last) = missed.iter().max_by_key(|r| r.ordinal).copied() else {
        return Ok(None);
    };
    let from = missed.iter().map(|r| r.ordinal).min().unwrap();
    let missed_ids: HashSet<_> = missed.iter().map(|r| r.id.0.as_str()).collect();
    let mut covered: BTreeSet<&str> = BTreeSet::new();
    for h in records(projection, "context-handoff").iter().filter(|h| {
        h["toProviderThreadId"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
            && native.is_some()
            && h["delivery"]["nativeThreadId"].as_str() == native
            && matches!(
                h["delivery"]["status"].as_str(),
                Some("inline" | "injected")
            )
    }) {
        covered.extend(
            ["itemIds", "omittedItemIds"]
                .into_iter()
                .flat_map(|key| h["delivery"][key].as_array().into_iter().flatten())
                .filter_map(Value::as_str),
        );
    }
    // A full switch/fork or carried retry already supplying the same items
    // takes precedence. Deliberate omissions count too; never retry them forever.
    for h in effective_handoffs {
        covered.extend(
            h["history"]["messages"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| m["itemId"].as_str()),
        );
        covered.extend(
            h["history"]["omittedItemIds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
    }
    let items: Vec<_> = local_items(projection, last.ordinal)
        .into_iter()
        .filter(|i| {
            i["runId"]
                .as_str()
                .is_some_and(|id| missed_ids.contains(id))
                && !i["id"].as_str().is_some_and(|id| covered.contains(id))
        })
        .collect();
    let messages: Vec<_> = items
        .iter()
        .filter_map(context::historical_message)
        .collect();
    if messages.is_empty() {
        return Ok(None);
    }
    let coverage = context::coverage(&run.thread_id.0, from, last.ordinal, &items);
    let selected = context::select_history(&messages, &coverage, 0, context::token_cap());
    let handoff_id = format!("handoff:{}", encode_component(&id));
    let now = iso(crate::now_ms())?;
    let handoff = json!({"id":handoff_id,"transferId":id,"threadId":run.thread_id,"targetRunId":run.id,
        "fromProviderThreadIds":[run.provider_thread_id],"toProviderThreadId":run.provider_thread_id,
        "coveredRunOrdinals":{"from":from,"to":last.ordinal},"strategy":"delta_since_target_last_seen",
        "status":"ready","summaryMessageId":null,"summaryText":"",
        "history":{"messages":selected.messages,"coverage":coverage,"omittedItems":selected.omitted_items,"omittedItemIds":selected.omitted_item_ids},
        "createdByProviderInstanceId":null,"createdAt":now,"updatedAt":now});
    let transfer = json!({"id":id,"type":"provider_handoff","sourceThreadId":run.thread_id,"targetThreadId":run.thread_id,
        "sourcePoint":canonical_point(projection,last),"basePoint":null,
        "sourceProviderInstanceId":run.provider_instance_id,"targetProviderInstanceId":run.provider_instance_id,
        "targetRunId":run.id,"status":"consumed","resolution":{"strategy":"delta_context","contextHandoffId":handoff_id},
        "createdBy":"system","error":null,"createdAt":now,"updatedAt":now,"consumedAt":now});
    let receipt = kernel
        .transfer_command(
            &run.thread_id,
            CommandId(id.clone()),
            TransferOperation::CreateHandoff {
                transfer: Box::new(transfer.clone()),
                handoff: Box::new(handoff.clone()),
            },
        )
        .await?;
    if receipt.status == ReceiptStatus::Rejected {
        return Err(Error::Invariant(receipt.error.unwrap_or_default()));
    }
    // A competing preparation may have won the stable command reservation.
    // Return that authoritative snapshot, never the losing caller's candidate.
    let transfer = kernel
        .store
        .thread_transfers(&run.thread_id)?
        .into_iter()
        .find(|t| t["id"] == id)
        .ok_or_else(|| Error::Invariant("Saved retry transfer missing.".into()))?;
    let saved = kernel
        .store
        .thread(&run.thread_id)?
        .ok_or_else(|| Error::Invariant("Saved retry thread missing.".into()))?;
    let handoff = records(&saved, "context-handoff")
        .iter()
        .find(|h| h["transferId"] == id)
        .cloned()
        .ok_or_else(|| Error::Invariant("Saved retry handoff missing.".into()))?;
    Ok(Some((transfer, handoff)))
}
