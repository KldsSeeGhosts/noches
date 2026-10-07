//! The desktop composer's chat config is the user's next selection. Mirror it
//! into the orchestration thread's saved selection (T3 persists the composer
//! choice as `thread.model-selection-updated` / `thread.provider-switched`) so
//! the queue's promotion mode and the next admitted turn follow it mid-run.
//! It only edits the saved selection: a running turn is never interrupted, and
//! the live run keeps the selection it was admitted with.

use super::{Command, Error, Kernel, ReceiptStatus, Result};
use crate::HarnessRegistry;
use serde_json::json;
use zeron_proto::ChatConfig;

/// Returns whether the saved selection moved. A thread that does not exist yet
/// (the first admission will adopt the config) and an unchanged selection are
/// no-ops; both make a replayed or duplicated config write harmless.
pub(crate) async fn mirror_chat_config(
    kernel: &Kernel,
    registry: &HarnessRegistry,
    chat_id: &str,
    config: &ChatConfig,
) -> Result<bool> {
    let thread = chat_id.into();
    let Some(projection) = kernel.store.thread(&thread)? else {
        return Ok(false);
    };
    let saved = &projection.thread;
    if saved.archived_at.is_some()
        || saved.deleted_at.is_some()
        || serde_json::to_value(&saved.lineage)?["relationshipToParent"] == "subagent"
    {
        return Ok(false);
    }
    let instance = config
        .instance_id
        .clone()
        .unwrap_or_else(|| crate::provider_instances::legacy_instance_id(config.harness));
    let wanted = crate::provider_instances::request_selection(
        registry,
        &instance,
        config.harness,
        config.model.as_deref(),
        &config.model_options,
        config.reasoning,
    )?;
    if wanted == saved.model_selection {
        return Ok(false);
    }
    // Same trusted-catalog validation the queue promotion freezes: an unknown
    // model or invalid option never becomes the thread's saved selection.
    super::queue::host::resolve_selection(registry, saved, &wanted)
        .await
        .map_err(Error::Invariant)?;
    let kind = if saved.provider_instance_id == wanted.instance_id {
        "thread.model-selection.set"
    } else {
        "provider.switch"
    };
    // A fresh id per applied change: the equality check above is the
    // idempotency, so returning to an earlier selection is a new command
    // rather than a replay of the old receipt.
    let command = Command::wire(serde_json::from_value(json!({
        "type":kind,"commandId":format!("composer-selection:{}",uuid::Uuid::new_v4()),
        "threadId":thread,"modelSelection":wanted
    }))?)?;
    let receipt = kernel.dispatch(&command, crate::now_ms()).await?;
    if receipt.status == ReceiptStatus::Rejected {
        return Err(Error::Invariant(receipt.error.unwrap_or_default()));
    }
    Ok(true)
}
