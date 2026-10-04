//! Production host assembly. Credentials and process handles never enter the
//! replicated projection. The same live catalog serves tools and the runner.
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use zeron_proto::RunRequest;
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::{DelegateTaskInputTarget, OrchestratorMcpFailureCode};
use zeron_proto::provider_instance::{ModelSelection, ProviderInstanceId};

use super::command::Command;
use super::runner::{RunnerBridge, RunnerInstances, RunnerMcp, RunnerProvider};
use super::service::{CallerScope, ToolError};
use super::sync_publish::{
    ProjectionPublisher, PublicationBatch, PublicationDocument, PublicationWorker,
};
use super::task::{DelegationService, DelegationTargets, ResolvedTarget, TaskOperation};
use super::{Error, Kernel, Result};
use crate::mcp::auth::InvocationScope;
use crate::{DocHost, HarnessRegistry, SessionsEngine, WorkspaceHost};

pub struct HostCatalog(pub Arc<HarnessRegistry>);

#[async_trait]
impl DelegationTargets for HostCatalog {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&DelegateTaskInputTarget>,
    ) -> std::result::Result<ResolvedTarget, ToolError> {
        let target = target
            .map(serde_json::to_value)
            .transpose()
            .map_err(tool_error)?;
        let selection = self.0.provider_instances.resolve_target(
            &self.0,
            &parent.model_selection,
            target.as_ref(),
        )?;
        let instance = self
            .0
            .provider_instances
            .snapshot(&self.0)
            .into_iter()
            .find(|p| p.provider_instance_id == selection.instance_id)
            .ok_or_else(|| tool_error("Provider disappeared from the catalog."))?;
        Ok(ResolvedTarget {
            selection,
            driver: instance.driver_kind,
        })
    }
}

#[async_trait]
impl RunnerInstances for HostCatalog {
    async fn resolve(
        &self,
        id: &ProviderInstanceId,
    ) -> std::result::Result<RunnerProvider, ToolError> {
        let instance = self
            .0
            .provider_instances
            .snapshot(&self.0)
            .into_iter()
            .find(|p| &p.provider_instance_id == id)
            .ok_or_else(|| tool_error("Provider is absent from the live catalog."))?;
        if !instance.constraints().is_empty() {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::ProviderUnavailable,
                instance.constraints().join(" "),
            ));
        }
        let harness = self
            .0
            .provider_instances
            .resolve_runtime(&self.0, id, true)
            .map_err(tool_error)?;
        Ok(RunnerProvider {
            capabilities: capabilities(harness.as_ref()),
            harness,
        })
    }
}

fn tool_error(error: impl std::fmt::Display) -> ToolError {
    ToolError::new(
        OrchestratorMcpFailureCode::OrchestrationError,
        error.to_string(),
    )
}

/// Conservative adapter snapshot: do not infer native fork/checkpoint/context
/// support from generic session operations.
pub fn capabilities(harness: &dyn zeron_harness::Harness) -> OrchestrationV2ProviderCapabilities {
    serde_json::from_value(json!({
        "sessions":{"supportsMultipleProviderThreadsPerSession":false,"supportsModelSwitchInSession":false,
            "supportsProviderSwitchingViaHandoff":false,"supportsRuntimeModeSwitchInSession":false,"pendingRequestsSurviveRestart":false},
        "threads":{"canCreateEmptyThread":false,"canReadThreadSnapshot":false,"canRollbackThread":false,
            "canForkThread":false,"canForkFromTurn":false,"canForkFromSubagentThread":false,"exposesNativeThreadId":false},
        "turns":{"exposesNativeTurnId":false,"emitsTurnStarted":true,"emitsTurnCompleted":true,
            "supportsInterrupt":true,"supportsActiveSteering":harness.supports_steering() && harness.steering_mode() == zeron_proto::SteeringMode::StepBoundary,
            "supportsSteeringByInterruptRestart":false,"supportsQueuedMessages":true,"terminalStatusQuality":"strong"},
        "streaming":{"streamsAssistantText":true,"streamsReasoning":false,"streamsToolOutput":false,
            "streamsPlanText":false,"emitsMessageCompleted":true},
        "tools":{"exposesToolItemIds":false,"emitsToolStarted":false,"emitsToolCompleted":false,
            "emitsToolOutput":false,"supportsMcpTools":true,"supportsDynamicToolCallbacks":false},
        "approvals":{"supportsCommandApproval":false,"supportsFileReadApproval":false,"supportsFileChangeApproval":false,
            "supportsApplyPatchApproval":false,"approvalsHaveNativeRequestIds":false,"approvalCallbacksAreLiveOnly":true,"approvalsCanOriginateFromSubagents":false},
        "planning":{"emitsPlanUpdated":false,"emitsTodoList":false,"emitsProposedPlan":false,"supportsStructuredQuestions":false,"planDeltasHaveItemIds":false},
        "subagents":{"supportsSubagents":false,"exposesSubagentThreadIds":false,"emitsSubagentLifecycle":false,
            "canWaitForSubagents":false,"canCloseSubagents":false,"canForkSubagentThread":false},
        "context":{"acceptsSystemContext":false,"acceptsDeveloperContext":false,"acceptsSyntheticUserContext":true,
            "canGenerateSummaries":false,"canConsumeHandoffSummaries":false,"supportsDeltaHandoff":false,"supportsFullThreadHandoff":false,"maxRecommendedHandoffChars":null},
        "checkpointing":{"appCanCheckpointFilesystem":false,"supportsNestedCheckpointScopes":false,
            "providerCanRollbackConversation":false,"providerRollbackReturnsSnapshot":false,"providerCanReadConversationSnapshot":false},
        "identity":{"nativeThreadIds":"weak","nativeTurnIds":"weak","nativeItemIds":"weak","nativeRequestIds":"weak"},
        "runtimePolicy":{"enforcement":"client-boundary"}
    })).expect("host capability shape")
}

struct HostMcp {
    kernel: Kernel,
    device_id: String,
}

#[async_trait]
impl RunnerMcp for HostMcp {
    async fn bind(
        &self,
        scope: CallerScope,
        sessions: &SessionsEngine,
    ) -> std::result::Result<(), ToolError> {
        let thread = self
            .kernel
            .store
            .thread(&scope.thread_id)
            .map_err(tool_error)?
            .ok_or_else(|| tool_error("MCP thread binding missing."))?;
        let run = thread
            .runs
            .iter()
            .find(|r| r.id == scope.run_id)
            .ok_or_else(|| tool_error("MCP run binding missing."))?;
        sessions
            .mcp_server()
            .register(
                sessions,
                &scope.thread_id.0.clone(),
                InvocationScope {
                    environment_id: self.device_id.clone(),
                    caller: scope,
                    selection: run.model_selection.clone(),
                    capabilities: ["orchestration"].into_iter().map(str::to_owned).collect(),
                    issued_at: 0,
                    task_id: None,
                },
            )
            .await
            .map_err(tool_error)?;
        Ok(())
    }
}

pub struct OrchestrationHost {
    pub bridge: Arc<RunnerBridge>,
    /// The same service the MCP tools call; the UI's user-authority Stop uses it.
    pub service: Arc<DelegationService>,
    stop: CancellationToken,
    workers: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl OrchestrationHost {
    pub(crate) fn discover(&self, registry: Arc<HarnessRegistry>, accounts: crate::AgentAccounts) {
        let stop = self.stop.clone();
        let worker = tokio::spawn(async move {
            tokio::select! {
                _ = stop.cancelled() => {},
                _ = async {
                    if let Ok(Ok(snapshot)) = tokio::time::timeout(
                        std::time::Duration::from_secs(15), accounts.list(false),
                    ).await {
                        registry.provider_instances.apply_accounts(&snapshot);
                    }
                    registry.provider_instances.refresh_all(&registry).await;
                } => {}
            }
        });
        self.workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(worker);
    }
    pub fn assemble(
        kernel: Kernel,
        sessions: SessionsEngine,
        doc_host: DocHost,
        workspace: WorkspaceHost,
        registry: Arc<HarnessRegistry>,
        device_id: String,
    ) -> Result<Self> {
        let catalog = Arc::new(HostCatalog(registry));
        let bridge = Arc::new(RunnerBridge {
            kernel: kernel.clone(),
            sessions: sessions.clone(),
            doc_host: doc_host.clone(),
            workspace: workspace.clone(),
            device_id: device_id.clone(),
            instances: catalog.clone(),
            mcp: Arc::new(HostMcp {
                kernel: kernel.clone(),
                device_id: device_id.clone(),
            }),
        });
        // Recovery is SQL/lock-only; never block_on a provider/network future.
        // Complete it under InstanceLock before installing callable services.
        futures::executor::block_on(bridge.recover_mailbox())?;
        let service = Arc::new(DelegationService {
            kernel: kernel.clone(),
            targets: catalog,
        });
        sessions.mcp_server().set_service(service.clone());
        sessions.set_orchestration_runner(Arc::downgrade(&bridge));
        let stop = CancellationToken::new();
        let mut workers = bridge.spawn_workers(stop.clone());
        let publisher = PublicationWorker {
            store: kernel.store.clone(),
            publisher: Arc::new(ChatPublisher {
                doc_host,
                workspace,
                device_id,
            }),
        };
        let publisher_stop = stop.clone();
        workers.push(tokio::spawn(async move {
            loop {
                if publisher_stop.is_cancelled() {
                    break;
                }
                match publisher.step().await {
                    Ok(true) => continue,
                    Err(error) => tracing::error!(%error, "orchestration publication failed"),
                    Ok(false) => {}
                }
                tokio::select! {
                    _ = publisher_stop.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(25)) => {}
                }
            }
        }));
        Ok(Self {
            bridge,
            service,
            stop,
            workers: std::sync::Mutex::new(workers),
        })
    }

    pub async fn shutdown(&self) {
        self.stop.cancel();
        let workers = std::mem::take(
            &mut *self
                .workers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        for worker in workers {
            let _ = worker.await;
        }
    }
}

impl Drop for OrchestrationHost {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

impl RunnerBridge {
    /// Ordinary session admission, before tool injection/provider start. Runner
    /// starts already have a canonical run and must not be adopted a second time.
    pub async fn admit_parent(
        &self,
        mut scope: CallerScope,
        selection: ModelSelection,
        request: &RunRequest,
        message_id: &str,
        harness: &dyn zeron_harness::Harness,
    ) -> Result<CallerScope> {
        let thread = &scope.thread_id;
        // Sessions publishes Idle before this independent observer commits
        // Done. Bound the admission race without inventing another active run.
        for _ in 0..100 {
            let active = self.kernel.store.thread(thread)?.is_some_and(|p| {
                p.runs
                    .last()
                    .is_some_and(|run| !super::command::run_terminal(&run.status))
            });
            if !active {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        if self.kernel.store.thread(thread)?.is_none() {
            let create = Command::wire(serde_json::from_value(json!({
                "type":"thread.create","commandId":format!("session-adopt:{}",thread.0),
                "createdBy":"user","creationSource":"web","threadId":thread,"projectId":scope.project_id,
                "title":self.workspace.chat(&thread.0).ok().flatten().and_then(|c| c.title).unwrap_or_else(|| "Conversation".into()),
                "modelSelection":selection,"runtimeMode":scope.runtime_mode,"interactionMode":scope.interaction_mode,
                "branch":null,"worktreePath":request.cwd
            }))?)?;
            let receipt = self.kernel.dispatch(&create, crate::now_ms()).await?;
            if receipt.status == super::ReceiptStatus::Rejected {
                return Err(Error::Invariant(receipt.error.unwrap_or_default()));
            }
        }
        // Refresh selection/modes/cwd on later ordinary turns, preserving lineage.
        let mut projection = self.kernel.store.thread(thread)?.unwrap();
        projection.thread.model_selection = selection;
        projection.thread.provider_instance_id = scope.provider_instance_id.clone();
        projection.thread.runtime_mode = scope.runtime_mode;
        projection.thread.interaction_mode = scope.interaction_mode;
        projection.thread.worktree_path = Some(request.cwd.clone());
        let update = Command {
            id: CommandId(format!("session-binding:{message_id}")),
            thread_id: thread.clone(),
            operation: super::command::Operation::SessionBinding(Box::new(projection.thread)),
        };
        let receipt = self.kernel.dispatch(&update, crate::now_ms()).await?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        self.kernel
            .task_command(
                thread,
                CommandId(format!("session-run:{message_id}")),
                TaskOperation::ExternalMessage {
                    prompt: request.prompt.clone(),
                    driver: crate::provider_instances::legacy_driver(harness.id()),
                    message_id: MessageId(message_id.into()),
                },
            )
            .await?;
        let projection = self.kernel.store.thread(thread)?.unwrap();
        let run = projection.runs.last().unwrap().clone();
        scope.run_id = run.id.clone();
        self.observe_external(thread.clone(), run, capabilities(harness))
            .await?;
        Ok(scope)
    }
}

struct ChatPublisher {
    doc_host: DocHost,
    workspace: WorkspaceHost,
    device_id: String,
}

#[async_trait]
impl ProjectionPublisher for ChatPublisher {
    async fn publish(
        &self,
        batch: &PublicationBatch,
        document: &PublicationDocument,
    ) -> Result<()> {
        if batch.host_id != self.device_id {
            return Err(Error::NotOwner);
        }
        // Registry patches are projected into each participating chat. The
        // ordinary workspace registry provides discovery, not a new sync room.
        let Some(id) = document.doc_id.strip_prefix("orchestration/thread/") else {
            for summary in document.payload["threads"].as_array().into_iter().flatten() {
                let id = summary["id"]
                    .as_str()
                    .ok_or_else(|| Error::Invariant("registry thread identity missing".into()))?;
                let mut summary = summary.clone();
                summary["hostId"] = json!(batch.host_id);
                summary["hostEpoch"] = json!(batch.host_epoch);
                summary["version"] = json!(document.version);
                summary["batchId"] = json!(batch.batch_id);
                self.workspace
                    .publish_orchestration_summary(id, summary)
                    .map_err(|error| Error::Invariant(error.to_string()))?;
            }
            return Ok(());
        };
        let thread: OrchestrationV2AppThread =
            serde_json::from_value(document.payload["thread"].clone())?;
        let space = thread
            .lineage
            .parent_thread_id
            .as_ref()
            .and_then(|p| self.workspace.chat(&p.0).ok().flatten())
            .and_then(|c| c.space_id);
        self.workspace
            .create_chat(
                id,
                space.as_deref(),
                Some(&self.device_id),
                None,
                thread.worktree_path.clone(),
            )
            .map_err(|e| Error::Invariant(e.to_string()))?;
        let handle = self
            .doc_host
            .open(id)
            .map_err(|e| Error::Invariant(e.to_string()))?;
        handle
            .doc()
            .publish_orchestration(
                &batch.host_id,
                batch.host_epoch,
                document.version,
                &batch.batch_id,
                &serde_json::to_value(&batch.documents)?,
                &document.payload,
            )
            .map_err(|e| Error::Invariant(e.to_string()))?;
        // Local offline handles are persisted before SQL acknowledgment too.
        self.doc_host
            .persist_orchestration(handle)
            .await
            .map_err(|error| Error::Invariant(error.to_string()))?;
        Ok(())
    }
}
