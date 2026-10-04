//! Project removal uses T3's thread-deletion lifecycle, never repo removal.
//! TODO(merge-threads): share this planner with canonical direct thread deletion.
use crate::orchestration::{
    Result,
    command::{Command, Plan, attempt_terminal, node_terminal, run_terminal},
    effects::{Effect, EffectOutcome, EffectRequest},
    event::iso,
    projection::ThreadProjection,
    runner::RunnerBridge,
    task::records,
};
use serde_json::json;

pub(crate) fn plan(p: &ThreadProjection, command: &Command, now: i64) -> Result<Plan> {
    let mut plan = Plan {
        cancel_process_effects: true,
        ..Default::default()
    };
    let time = iso(now)?;
    let mut thread = p.thread.clone();
    thread.deleted_at = Some(time.clone());
    thread.updated_at = time.clone();
    thread.title_regeneration = zeron_proto::orchestration::Optional::Present(None);
    plan.emit(command, "thread.deleted", &thread, now)?;
    let tasks = records(p, "subagent");
    for run in &p.runs {
        let mut value = serde_json::to_value(run)?;
        let active = !run_terminal(&run.status);
        if active {
            value["status"] = json!("cancelled");
            value["queuePosition"] = json!(null);
            value["completedAt"] = json!(time);
        }
        let cohort = value.get("delegatedCompletion").is_some()
            || tasks
                .iter()
                .any(|t| t["origin"] == "app_owned" && t["runId"] == run.id.0);
        if cohort {
            let next = value["delegatedCompletion"]["nextGeneration"]
                .as_i64()
                .unwrap_or(1);
            value["delegatedCompletion"] =
                json!({"disposition":"disposed","nextGeneration":next,"delivery":null});
        }
        if active || cohort {
            plan.emit(command, "run.updated", &value, now)?;
        }
    }
    for attempt in &p.attempts {
        if !attempt_terminal(&attempt.status)
            && p.runs
                .iter()
                .any(|r| r.id == attempt.run_id && !run_terminal(&r.status))
        {
            let mut value = serde_json::to_value(attempt)?;
            value["status"] = json!("cancelled");
            value["completedAt"] = json!(time);
            plan.emit(command, "run-attempt.updated", &value, now)?;
        }
    }
    for node in &p.nodes {
        if !node_terminal(&node.status)
            && p.runs
                .iter()
                .any(|r| Some(&r.id) == node.run_id.as_ref() && !run_terminal(&r.status))
        {
            let mut value = serde_json::to_value(node)?;
            value["status"] = json!("cancelled");
            value["completedAt"] = json!(time);
            plan.emit(command, "node.updated", &value, now)?;
        }
    }
    for mut request in records(p, "runtime-request").to_vec() {
        if request["status"] == "pending" {
            request["status"] = json!("cancelled");
            request["responseCapability"] =
                json!({"type":"not_resumable","reason":"The thread was deleted."});
            request["resolvedAt"] = json!(time);
            plan.emit(command, "runtime-request.updated", &request, now)?;
        }
    }
    for mut task in tasks.to_vec() {
        if task["origin"] == "app_owned"
            && !task["runId"].is_null()
            && !matches!(
                task["completionDelivery"]["state"].as_str(),
                Some("acknowledged" | "delivered" | "disposed")
            )
        {
            task["completionDelivery"] = json!({"state":"disposed","observedByRunId":null});
            task["updatedAt"] = json!(time);
            plan.emit(command, "subagent.updated", &task, now)?;
        }
    }
    for session in records(p, "provider-session") {
        if matches!(session["status"].as_str(), Some("stopped" | "error")) {
            continue;
        }
        let provider_session_id = serde_json::from_value(session["id"].clone())?;
        plan.emit(command, "provider-session.detached",&json!({"providerSessionId":provider_session_id,"detachedAt":time,"reason":"Thread deleted."}), now)?;
        plan.effects.push(EffectRequest::ProviderSessionDetach {
            provider_session_id,
        });
    }
    plan.effects.push(EffectRequest::TerminalCleanup);
    let ids: std::collections::BTreeSet<_> = records(p, "message")
        .iter()
        .flat_map(|m| m["attachments"].as_array().into_iter().flatten())
        .filter_map(|a| a["id"].as_str().map(str::to_owned))
        .collect();
    if !ids.is_empty() {
        plan.effects.push(EffectRequest::AttachmentCleanup {
            attachment_ids: ids.into_iter().collect(),
        });
    }
    Ok(plan)
}

pub(crate) async fn execute(bridge: &RunnerBridge, effect: &Effect) -> Result<EffectOutcome> {
    match &effect.request {
        EffectRequest::ProviderSessionDetach { .. } => {
            bridge
                .sessions
                .mcp_server()
                .credentials
                .revoke_thread(&effect.thread_id.0);
            bridge
                .sessions
                .interrupt(&effect.thread_id.0)
                .await
                .map_err(super::invariant)?;
        }
        EffectRequest::TerminalCleanup | EffectRequest::AttachmentCleanup { .. } => {
            let service = bridge
                .sessions
                .mcp_server()
                .toolkit
                .launch_service()
                .ok_or_else(|| super::invariant("Launch cleanup service is unavailable."))?;
            service
                .cleanup(
                    &effect.thread_id.0,
                    match &effect.request {
                        EffectRequest::AttachmentCleanup { attachment_ids } => {
                            Some(attachment_ids.clone())
                        }
                        _ => None,
                    },
                )
                .await
                .map_err(|e| super::invariant(e.message))?;
        }
        _ => return Err(super::invariant("Unexpected cleanup request.")),
    }
    Ok(EffectOutcome::Succeeded)
}

impl super::HostLaunchService {
    pub(crate) fn cleanup_owned(
        &self,
        thread: &str,
        attachments: Option<Vec<String>>,
    ) -> Result<()> {
        if let Some(ids) = attachments {
            for id in ids {
                if let Some(path) = self.kernel.store.launch_attachment_path(&id, thread)? {
                    match std::fs::remove_file(path) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(super::invariant(e)),
                    }
                    self.kernel.store.write(|tx| {
                        tx.execute("DELETE FROM orchestration_launch_claims WHERE id=?1", [&id])?;
                        Ok(())
                    })?;
                }
            }
        } else if let Ok(w) = self.workflow(thread) {
            if let Some(run) = w["setup"]["runId"].as_str() {
                if let Some(cancel) = super::lock(&self.setup_cancels).get(run) {
                    cancel.cancel();
                }
            }
            if let Some(terminal) = w["setup"]["terminalId"].as_str() {
                let _ = self.terminals.close(terminal);
            }
        }
        Ok(())
    }
}
