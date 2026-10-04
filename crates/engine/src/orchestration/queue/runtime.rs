//! Runtime user-question projection hook. Approvals can block lifecycle changes
//! but are never readable/respondable through the question tool family.
use serde_json::{Value, json};
use zeron_proto::AgentEvent;
use zeron_proto::orchestration::RunId;

use crate::orchestration::command::{Command, Plan};
use crate::orchestration::event::{encode_component, iso};
use crate::orchestration::projection::ThreadProjection;
use crate::orchestration::{Result, task};

pub(crate) fn observe(
    p: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    run_id: &RunId,
    provider: &Value,
    event: &AgentEvent,
    now: i64,
) -> Result<()> {
    let Some(run) = p.runs.iter().find(|r| &r.id == run_id) else {
        return Ok(());
    };
    let incoming = match event {
        AgentEvent::InputRequested {
            request_id,
            questions,
        } => Some((request_id.as_str(), "user_input", Some(questions))),
        AgentEvent::PermissionRequested { request } => Some((request.id.as_str(), "command", None)),
        _ => None,
    };
    if let Some((id, kind, questions)) = incoming {
        let node_id = format!("node:request:{}", encode_component(id));
        let request = json!({"id":id,"nodeId":node_id,"providerTurnId":null,"nativeRequestRef":null,
            "kind":kind,"status":"pending","responseCapability":{"type":"live","providerSessionId":provider["providerSessionId"]},
            "createdAt":iso(now)?,"resolvedAt":null});
        plan.emit(command, "runtime-request.updated", &request, now)?;
        let node = json!({"id":node_id,"threadId":p.thread.id,"runId":run.id,"rootNodeId":run.root_node_id,
            "parentNodeId":run.root_node_id,"kind":if kind=="user_input"{"user_input_request"}else{"approval_request"},
            "status":"waiting","countsForRun":true,"providerThreadId":run.provider_thread_id,
            "providerTurnId":null,"nativeItemRef":null,"runtimeRequestId":id,"checkpointScopeId":null,
            "startedAt":iso(now)?,"completedAt":null});
        plan.emit(command, "node.updated", &node, now)?;
        if let Some(questions) = questions {
            let questions:Vec<_>=questions.iter().map(|q|json!({"id":q.id,"header":q.header,"question":q.question,
                "options":q.options.iter().map(|label|json!({"label":label,"description":""})).collect::<Vec<_>>(),
                "multiSelect":q.multi_select})).collect();
            let item = json!({"id":format!("item:request:{}",encode_component(id)),"threadId":p.thread.id,
                "runId":run.id,"nodeId":node_id,"providerThreadId":run.provider_thread_id,
                "providerTurnId":null,"nativeItemRef":null,"parentItemId":null,
                "ordinal":task::records(p,"turn-item").len()+1,"status":"running","title":null,
                "startedAt":iso(now)?,"completedAt":null,"updatedAt":iso(now)?,
                "type":"user_input_request","requestId":id,"questions":questions});
            plan.emit(command, "turn-item.updated", &item, now)?;
        }
    }
    let resolution = match event {
        AgentEvent::InputResolved { request_id } => Some((request_id.as_str(), "resolved")),
        AgentEvent::PermissionUpdated { request } => Some((
            request.id.as_str(),
            match request.state {
                zeron_proto::RequestState::Pending => "pending",
                zeron_proto::RequestState::Resolved => "resolved",
                zeron_proto::RequestState::Expired => "expired",
            },
        )),
        _ => None,
    };
    for request in task::records(p, "runtime-request")
        .iter()
        .filter(|r| r["status"] == "pending")
    {
        let belongs = p
            .nodes
            .iter()
            .any(|n| r_node(request) == Some(n.id.0.as_str()) && n.run_id.as_ref() == Some(run_id));
        if !belongs {
            continue;
        }
        let status = if let Some((id, status)) = resolution {
            if request["id"] != id {
                continue;
            }
            status
        } else if matches!(event, AgentEvent::Done { .. })
            && request["responseCapability"]["type"] != "message"
        {
            "expired"
        } else {
            continue;
        };
        let mut request = request.clone();
        request["status"] = json!(status);
        if status != "pending" {
            request["resolvedAt"] = json!(iso(now)?);
        }
        plan.emit(command, "runtime-request.updated", &request, now)?;
        for node in p
            .nodes
            .iter()
            .filter(|n| r_node(&request) == Some(n.id.0.as_str()))
        {
            let mut node = serde_json::to_value(node)?;
            node["status"] = json!(if status == "resolved" {
                "completed"
            } else {
                "cancelled"
            });
            node["completedAt"] = json!(iso(now)?);
            plan.emit(command, "node.updated", &node, now)?;
        }
    }
    Ok(())
}

fn r_node(request: &Value) -> Option<&str> {
    request["nodeId"].as_str()
}
