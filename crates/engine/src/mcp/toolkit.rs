use std::sync::{Arc, PoisonError, RwLock};

use serde_json::{Value, json};
use zeron_proto::orchestration::normalize_contract;
use zeron_proto::orchestration_mcp::{
    OrchestrationToolInput, ToolDescriptor, pinned_tool_inventory,
};

use super::service_stub::UnavailableOrchestratorService;
use super::{INSTRUCTIONS, auth::InvocationScope, codec};
use crate::HarnessRegistry;
use crate::orchestration::service::OrchestratorService;

pub struct Toolkit {
    pub registry: Arc<HarnessRegistry>,
    service: RwLock<Arc<dyn OrchestratorService>>,
    queue_service: RwLock<Option<Arc<dyn crate::orchestration::queue_service::QueueService>>>,
    scheduler: RwLock<Option<Arc<dyn crate::orchestration::scheduler::service::SchedulerService>>>,
    threads: RwLock<Option<Arc<dyn crate::orchestration::thread_service::ThreadService>>>,
    inventory: Vec<ToolDescriptor>,
    descriptors: Vec<Value>,
    null_refusals: Value,
}

impl Toolkit {
    pub fn new(registry: Arc<HarnessRegistry>) -> Self {
        let source: Vec<Value> = serde_json::from_str(include_str!(
            "../../../proto/tests/t3_oracle/fixtures/tools.json"
        ))
        .expect("pinned tools");
        Self {
            registry,
            service: RwLock::new(Arc::new(UnavailableOrchestratorService)),
            queue_service: RwLock::new(None),
            scheduler: RwLock::new(None),
            threads: RwLock::new(None),
            inventory: pinned_tool_inventory(),
            null_refusals: serde_json::from_str(include_str!(
                "../../tests/t3_mcp_oracle/null-refusals.json"
            ))
            .expect("pinned null refusals"),
            descriptors: source
                .into_iter()
                .filter(|t| t["phase"] == "core")
                .map(|t| t["descriptor"].clone())
                .collect(),
        }
    }

    pub fn set_service(&self, service: Arc<dyn OrchestratorService>) {
        *self.service.write().unwrap_or_else(PoisonError::into_inner) = service;
    }

    pub fn set_scheduler(
        &self,
        service: Arc<dyn crate::orchestration::scheduler::service::SchedulerService>,
    ) {
        *self
            .scheduler
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(service);
    }

    pub fn set_thread_service(
        &self,
        service: Arc<dyn crate::orchestration::thread_service::ThreadService>,
    ) {
        *self.threads.write().unwrap_or_else(PoisonError::into_inner) = Some(service);
    }

    pub fn tools(&self) -> &[Value] {
        &self.descriptors
    }

    pub fn set_queue_service(
        &self,
        service: Arc<dyn crate::orchestration::queue_service::QueueService>,
    ) {
        *self
            .queue_service
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(service);
    }

    pub async fn request(&self, scope: InvocationScope, message: Value) -> Option<Value> {
        let id = message.get("id")?.clone();
        if message["jsonrpc"] != "2.0"
            || !matches!(id, Value::String(_) | Value::Number(_) | Value::Null)
        {
            return Some(codec::rpc_error(Value::Null, -32600, "Invalid Request"));
        }
        let result = match message["method"].as_str() {
            Some("initialize") => json!({
                "protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":false}},
                "serverInfo":{"name":"t3-code","version":env!("CARGO_PKG_VERSION")},"instructions":INSTRUCTIONS
            }),
            Some("ping") => json!({}),
            Some("tools/list") => json!({"tools":self.descriptors}),
            Some("tools/call") => {
                let name = message["params"]["name"].as_str().unwrap_or("");
                let Some(tool) = self.inventory.iter().find(|t| {
                    t.name == name && t.phase == zeron_proto::orchestration_mcp::ToolPhase::Core
                }) else {
                    return Some(codec::rpc_error(
                        id,
                        -32602,
                        &format!("Tool '{name}' not found"),
                    ));
                };
                let mut args = message["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if let Some(description) = codec::null_refusal(&self.null_refusals[name], &args) {
                    return Some(self.invalid(id, tool, &description));
                }
                if let Some(schedule) = args.get_mut("schedule")
                    && let Some(encoded) = schedule.as_str()
                {
                    match serde_json::from_str(encoded) {
                        Ok(decoded) => *schedule = decoded,
                        Err(_) => {
                            return Some(self.invalid(
                                id,
                                tool,
                                "Expected a valid JSON string\n  at [\"schedule\"]",
                            ));
                        }
                    }
                }
                for field in ["modelSelection", "defaultModelSelection"] {
                    if let Some(selection) = args.get_mut(field)
                        && !selection.is_null()
                    {
                        match normalize_contract("ModelSelection", selection.clone()) {
                            Ok(decoded) => *selection = decoded,
                            Err(_) => {
                                return Some(self.invalid(id, tool, "Invalid model selection."));
                            }
                        }
                    }
                }
                // Normalize strict option shorthand before the decoded schema.
                if let Some(options) = args.get_mut("target").and_then(|t| t.get_mut("options"))
                    && !options.is_null()
                {
                    match codec::normalize_target_options(options.clone()) {
                        Ok(value) => *options = value,
                        Err(description) => {
                            return Some(self.invalid(id, tool, &description));
                        }
                    }
                }
                codec::decode(&tool.decoded_input_schema, &mut args);
                if let Err(description) = codec::validate(&tool.decoded_input_schema, &args) {
                    return Some(self.invalid(id, tool, &description));
                }
                if let Err(description) = codec::refinements(name, &args) {
                    return Some(self.invalid(id, tool, &description));
                }
                let typed = match serde_json::from_value::<OrchestrationToolInput>(
                    json!({"name":name,"arguments":args}),
                ) {
                    Ok(input) => input,
                    Err(_) => return Some(self.invalid(id, tool, "Invalid tool arguments.")),
                };
                self.call(&scope, tool, typed, args).await
            }
            _ => return Some(codec::rpc_error(id, -32601, "Method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    fn invalid(&self, id: Value, tool: &ToolDescriptor, description: &str) -> Value {
        if tool.failure_mode.as_ref().is_some_and(|m| m == "error") {
            codec::rpc_error(
                id,
                -32602,
                &format!("Invalid parameters for tool '{}': {description}", tool.name),
            )
        } else {
            json!({"jsonrpc":"2.0","id":id,"result":codec::result(codec::invalid_parameters(&tool.name, description))})
        }
    }

    async fn call(
        &self,
        scope: &InvocationScope,
        tool: &ToolDescriptor,
        input: OrchestrationToolInput,
        args: Value,
    ) -> Value {
        let name = tool.name.as_str();
        if tool.group == "pullRequests" {
            return self.pull_request_unavailable(scope, name);
        }
        if tool.group == "worktree" && !scope.capabilities.contains("worktree") {
            return codec::result(if name == "t3_worktree_list" {
                codec::failure(
                    "capability_denied",
                    "This credential cannot inspect worktrees.",
                )
            } else {
                json!({"_tag":"WorktreeMcpFailure","code":"capability_denied","message":"This MCP credential does not grant worktree capabilities."})
            });
        }
        if !matches!(name, "t3_worktree_handoff" | "t3_worktree_status")
            && !scope.capabilities.contains("orchestration")
        {
            return codec::result(codec::failure(
                "capability_denied",
                if tool.group == "orchestrator" {
                    "This MCP credential does not grant orchestration capabilities."
                } else {
                    "This credential cannot control threads."
                },
            ));
        }
        if let Some(task) = &scope.task_id
            && name != "run_scheduled_task_now"
            && args
                .get("taskId")
                .and_then(Value::as_str)
                .is_some_and(|id| id != task)
        {
            return codec::result(codec::failure("task_not_found", "The task was not found."));
        }
        let queue_service = self
            .queue_service
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        match input {
            _ if matches!(
                name,
                "t3_queue_list"
                    | "t3_queue_read"
                    | "t3_queue_edit"
                    | "t3_queue_cancel"
                    | "t3_queue_reorder"
                    | "t3_queue_promote_to_steer"
                    | "t3_pending_request_list"
                    | "t3_pending_request_read"
                    | "t3_pending_request_respond"
                    | "t3_thread_update"
                    | "t3_thread_organize"
                    | "t3_thread_search"
            ) =>
            {
                crate::orchestration::queue::mcp::dispatch(queue_service, scope, name, args).await
            }
            input @ (OrchestrationToolInput::ScheduleTask(_)
            | OrchestrationToolInput::ListScheduledTasks(_)
            | OrchestrationToolInput::UpdateScheduledTask(_)
            | OrchestrationToolInput::DeleteScheduledTask(_)
            | OrchestrationToolInput::RunScheduledTaskNow(_)) => {
                crate::orchestration::scheduler::mcp::dispatch(
                    &self.scheduler,
                    scope.caller.clone(),
                    input,
                )
                .await
            }
            OrchestrationToolInput::OrchestratorCapabilities(_) => {
                self.registry
                    .provider_instances
                    .refresh_all(&self.registry)
                    .await;
                let providers: Vec<_> = self
                    .registry
                    .provider_instances
                    .snapshot(&self.registry)
                    .iter()
                    .map(|p| p.capability())
                    .collect();
                codec::result(json!({
                    "parentThreadId":scope.caller.thread_id,
                    "inheritedProviderInstanceId":scope.selection.instance_id,"inheritedModel":scope.selection.model,
                    "runtimeMode":scope.caller.runtime_mode,"interactionMode":scope.caller.interaction_mode,
                    "providers":providers,
                    "features":{"appOwnedSubagents":true,"asyncPolling":true,"cancellation":true,
                        "batchThreadCreation":true,"threadManagement":true,"incrementalThreadRead":true,
                        "scheduledTasks":true,"maxBatchThreads":20}
                }))
            }
            OrchestrationToolInput::DelegateTask(mut input) => {
                // Catalog/policy preconditions precede receipted domain
                // dispatch on every retry. No stale success bypasses them.
                let catalog = &self.registry.provider_instances;
                catalog.refresh_all(&self.registry).await;
                {
                    let selection = match catalog.resolve_target(
                        &self.registry,
                        &scope.selection,
                        args.get("target"),
                    ) {
                        Ok(selection) => selection,
                        Err(error) => {
                            return codec::result(
                                serde_json::to_value(error.into_failure()).expect("failure"),
                            );
                        }
                    };
                    let mut target =
                        json!({"providerInstanceId":selection.instance_id,"model":selection.model});
                    if let Some(options) = selection.options.as_ref() {
                        target["options"] = serde_json::to_value(options).expect("options");
                    }
                    input.target = zeron_proto::orchestration::Optional::Present(
                        serde_json::from_value(target).expect("resolved target"),
                    );
                }
                if let Some(requested) = args["runtimeMode"]
                    .as_str()
                    .filter(|mode| *mode != "inherit")
                {
                    let child: zeron_proto::RuntimeMode =
                        serde_json::from_value(json!(requested)).expect("validated mode");
                    if !scope.caller.runtime_mode.permits(child) {
                        return codec::result(codec::failure(
                            "runtime_mode_escalation_denied",
                            &format!(
                                "Child runtime mode {requested} is broader than parent mode {}.",
                                serde_json::to_value(scope.caller.runtime_mode)
                                    .expect("mode")
                                    .as_str()
                                    .expect("string")
                            ),
                        ));
                    }
                }
                if let Some(requested) = args["interactionMode"]
                    .as_str()
                    .filter(|mode| *mode != "inherit")
                {
                    let child: zeron_proto::InteractionMode =
                        serde_json::from_value(json!(requested)).expect("validated mode");
                    if !scope.caller.interaction_mode.permits(child) {
                        return codec::result(codec::failure(
                            "interaction_mode_escalation_denied",
                            &format!(
                                "Child interaction mode {requested} is broader than parent mode {}.",
                                serde_json::to_value(scope.caller.interaction_mode)
                                    .expect("mode")
                                    .as_str()
                                    .expect("string")
                            ),
                        ));
                    }
                }
                let service = self
                    .service
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                codec::result(
                    match service.delegate_task(scope.caller.clone(), *input).await {
                        Ok(value) => serde_json::to_value(value).expect("delegate result"),
                        Err(error) => serde_json::to_value(error.into_failure()).expect("failure"),
                    },
                )
            }
            OrchestrationToolInput::TaskStatus(input) => {
                let service = self
                    .service
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                codec::result(
                    match service.task_status(scope.caller.clone(), *input).await {
                        Ok(value) => serde_json::to_value(value).expect("task status"),
                        Err(error) => serde_json::to_value(error.into_failure()).expect("failure"),
                    },
                )
            }
            OrchestrationToolInput::TaskCancel(input) => {
                let service = self
                    .service
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                codec::result(
                    match service.task_cancel(scope.caller.clone(), *input).await {
                        Ok(value) => serde_json::to_value(value).expect("task cancel"),
                        Err(error) => serde_json::to_value(error.into_failure()).expect("failure"),
                    },
                )
            }
            input @ (OrchestrationToolInput::T3ThreadList(_)
            | OrchestrationToolInput::T3ThreadRead(_)
            | OrchestrationToolInput::T3ThreadSend(_)
            | OrchestrationToolInput::T3ThreadWait(_)
            | OrchestrationToolInput::T3ThreadInterrupt(_)
            | OrchestrationToolInput::T3ThreadConfiguration(_)
            | OrchestrationToolInput::T3ThreadConfigure(_)
            | OrchestrationToolInput::CreateThreads(_)) => {
                crate::orchestration::threads::mcp::dispatch(&self.threads, scope, input).await
            }
            _ if matches!(name, "t3_worktree_handoff" | "t3_worktree_status") => {
                codec::result(json!({
                    "_tag":"WorktreeMcpFailure","code":"operation_failed",
                    "message":format!("Unable to read thread {}: The operation could not be completed.", scope.caller.thread_id)
                }))
            }
            // Future domains deliberately refuse; there is no legacy RPC
            // emulation, mutation, receipt, provider call, or hidden success.
            _ => codec::result(codec::unavailable()),
        }
    }

    fn pull_request_unavailable(&self, scope: &InvocationScope, name: &str) -> Value {
        if !scope.capabilities.contains("pull-requests") {
            return codec::error_text(
                "MCP credential does not grant the pull-requests capability.",
            );
        }
        codec::error_text(match name {
            "link_pull_request" => "Could not link the pull request.",
            "unlink_pull_request" => "Could not unlink the pull request.",
            "list_thread_pull_requests" => "Could not list the pull request.",
            _ => "Could not change whether the pull request is watched.",
        })
    }
}
