//! Safe next-turn delivery: never restart active work to make room for mail.
use super::Result;
use super::command::{Command, Plan};
use super::effects::EffectRequest;
use super::event::iso;
use super::projection::ThreadProjection;
use super::task::{active_run, records};
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

pub(crate) fn drain(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    now: i64,
) -> Result<()> {
    if active_run(projection).is_some()
        || projection.thread.archived_at.is_some()
        || projection.thread.deleted_at.is_some()
    {
        return Ok(());
    }
    for queued in projection.runs.iter().filter(|run| {
        run.status == OrchestrationV2RunStatus::Queued && run.queue_held.as_ref() != Some(&true)
    }) {
        // Mail whose members were all observed/disposed must not wake.
        if let Some(message) = records(projection, "message")
            .iter()
            .find(|message| message["id"] == queued.user_message_id.0)
            && !message["delegatedCompletion"].is_null()
        {
            let ownership = &message["delegatedCompletion"];
            let current = super::mailbox::DeliveryCommand {
                parent_run_id: RunId(ownership["parentRunId"].as_str().unwrap().into()),
                generation: ownership["generation"].as_i64().unwrap(),
                message_id: queued.user_message_id.clone(),
                action: super::mailbox::DeliveryAction::Queue,
            };
            if super::mailbox::current_delivery(projection, &current).is_none() {
                let mut value = serde_json::to_value(queued)?;
                value["status"] = json!("cancelled");
                value["completedAt"] = json!(iso(now)?);
                plan.emit(command, "run.updated", &value, now)?;
                continue;
            }
        }
        let mut value = serde_json::to_value(queued)?;
        value["status"] = json!("starting");
        value["queuePosition"] = Value::Null;
        plan.emit(command, "run.updated", &value, now)?;
        if let Some(provider_id) = &queued.provider_thread_id
            && let Some(provider) = records(projection, "provider-thread")
                .iter()
                .find(|provider| provider["id"] == provider_id.0)
        {
            let mut provider = provider.clone();
            provider["lastRunOrdinal"] = json!(queued.ordinal);
            provider["ownerNodeId"] = json!(queued.root_node_id);
            plan.emit(command, "provider-thread.updated", &provider, now)?;
        }
        plan.effects.push(EffectRequest::ProviderTurnStart {
            run_id: queued.id.clone(),
        });
        break;
    }
    Ok(())
}
