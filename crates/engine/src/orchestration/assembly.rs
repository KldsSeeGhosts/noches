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
use super::launch::host_intake::HostLaunchIntake;
use super::runner::{RunnerBridge, RunnerInstances, RunnerMcp, RunnerProvider};
use super::service::{CallerScope, ToolError};
use super::sync_publish::{
    ProjectionPublisher, PublicationBatch, PublicationDocument, PublicationWorker,
};
use super::task::{DelegationService, DelegationTargets, ResolvedTarget, TaskOperation};
use super::{Error, Kernel, Result};
use crate::mcp::auth::InvocationScope;
use crate::{DocHost, HarnessRegistry, SessionsEngine, WorkspaceHost};

// ── F1 git-actions host assembly (owned by git-actions slice) ──────────────
pub struct GitActionsHost {
    pub service: super::git_actions::GitActionsService,
    stop: CancellationToken,
    worker: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl GitActionsHost {
    #[allow(clippy::too_many_arguments)]
    pub fn assemble(
        store: super::Store,
        sessions: SessionsEngine,
        terminals: crate::Terminals,
        doc_host: DocHost,
        workspace: WorkspaceHost,
        registry: Arc<HarnessRegistry>,
        repos: crate::Repos,
        history_roots: Vec<(zeron_proto::git_actions::HistorySource, std::path::PathBuf)>,
        pull_requests: Option<Arc<dyn super::pull_requests::PullRequestLinks>>,
    ) -> anyhow::Result<Self> {
        let service = super::git_actions::GitActionsService::new(
            store,
            repos,
            sessions,
            terminals,
            doc_host,
            workspace,
            registry,
            history_roots,
        )?;
        if let Some(pull_requests) = pull_requests {
            service.set_pr_linker(Arc::new(super::git_actions::UserPullRequestLinker(
                pull_requests,
            )));
        }
        let stop = CancellationToken::new();
        let token = stop.clone();
        let worker_service = service.clone();
        let worker = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {},
                }
                worker_service.pull_tick().await;
            }
        });
        Ok(Self {
            service,
            stop,
            worker: std::sync::Mutex::new(Some(worker)),
        })
    }
    pub async fn shutdown(&self) {
        self.stop.cancel();
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(worker) = worker {
            let _ = worker.await;
        }
        self.service.shutdown().await;
    }
}
impl Drop for GitActionsHost {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
// ── End F1 assembly ──────────────────────────────────────────────────────

pub struct HostCatalog(pub Arc<HarnessRegistry>);

#[async_trait]
impl DelegationTargets for HostCatalog {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&DelegateTaskInputTarget>,
    ) -> std::result::Result<ResolvedTarget, ToolError> {
        self.0.provider_instances.refresh_all(&self.0).await;
        let target = target
            .map(serde_json::to_value)
            .transpose()
            .map_err(tool_error)?;
        let mut inherited = parent.model_selection.clone();
        // Registry chats with no model and a cold discovery cache retain the
        // ordinary intake's implicit default. Resolve it only on actual send,
        // never by doing provider discovery/work during transcript import.
        if inherited.model == "default"
            && parent.history_origin.as_ref() == Some(&OrchestrationV2ThreadHistoryOrigin::V1Import)
            && let Some(provider) = self
                .0
                .provider_instances
                .snapshot(&self.0)
                .into_iter()
                .find(|p| p.provider_instance_id == inherited.instance_id)
            && !provider.models.iter().any(|m| m.id == inherited.model)
            && let Some(model) = provider.models.first()
        {
            inherited.model = model.id.clone();
        }
        let selection =
            self.0
                .provider_instances
                .resolve_target(&self.0, &inherited, target.as_ref())?;
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
    let mut capabilities: OrchestrationV2ProviderCapabilities = serde_json::from_value(json!({
        "sessions":{"supportsMultipleProviderThreadsPerSession":false,"supportsModelSwitchInSession":false,
            "supportsProviderSwitchingViaHandoff":false,"supportsRuntimeModeSwitchInSession":false,"pendingRequestsSurviveRestart":false},
        "threads":{"canCreateEmptyThread":false,"canReadThreadSnapshot":false,"canRollbackThread":false,
            "canForkThread":false,"canForkFromTurn":false,"canForkFromSubagentThread":false,"exposesNativeThreadId":false},
        "turns":{"exposesNativeTurnId":false,"emitsTurnStarted":true,"emitsTurnCompleted":true,
            "supportsInterrupt":true,"supportsActiveSteering":harness.supports_steering() && harness.steering_mode() == zeron_proto::SteeringMode::StepBoundary,
            "supportsSteeringByInterruptRestart":!matches!(harness.id(), zeron_proto::HarnessId::ClaudeCode | zeron_proto::HarnessId::Pi),
            "supportsQueuedMessages":true,"terminalStatusQuality":"strong"},
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
        "checkpointing":{"appCanCheckpointFilesystem":true,"supportsNestedCheckpointScopes":false,
            "providerCanRollbackConversation":false,"providerRollbackReturnsSnapshot":false,"providerCanReadConversationSnapshot":false},
        "identity":{"nativeThreadIds":"weak","nativeTurnIds":"weak","nativeItemIds":"weak","nativeRequestIds":"weak"},
        "runtimePolicy":{"enforcement":"client-boundary"}
    })).expect("host capability shape");
    if let Some(lifecycle) = harness.session_lifecycle() {
        capabilities.threads.can_fork_thread = true;
        capabilities.threads.can_fork_from_turn = lifecycle.can_fork_from_turn();
        capabilities.threads.exposes_native_thread_id = true;
        capabilities.turns.exposes_native_turn_id = true;
        capabilities.identity.native_thread_ids = OrchestrationV2NativeRefStrength::Strong;
        capabilities.identity.native_turn_ids = OrchestrationV2NativeRefStrength::Strong;
    }
    capabilities
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
                    capabilities: ["orchestration", "worktree", "pull-requests"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
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
    pub threads: Arc<dyn super::thread_service::ThreadService>,
    pub scheduler: Arc<super::scheduler::Scheduler>,
    pub launch: Option<Arc<super::launch::HostLaunchService>>,
    pub pull_requests: Arc<super::pull_requests::PullRequestService>,
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
        kernel
            .store
            .install_admission(super::adoption::RegistryAdmission {
                workspace: workspace.clone(),
                docs: doc_host.clone(),
                registry: registry.clone(),
            });
        let catalog = Arc::new(HostCatalog(registry.clone()));
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
        // --- transfer slice assembly ---
        sessions.mcp_server().toolkit.set_transfer_service(Arc::new(
            super::transfer::mcp::EngineTransferService {
                kernel: kernel.clone(),
            },
        ));
        // --- end transfer slice assembly ---
        // BEGIN wave3 threads: shared controls, passive read model, MCP facade.
        let threads = Arc::new(super::threads::KernelThreadService {
            kernel: kernel.clone(),
            delegation: service.clone(),
        });
        sessions
            .mcp_server()
            .toolkit
            .set_thread_service(threads.clone());
        // END wave3 threads.
        sessions.set_orchestration_runner(Arc::downgrade(&bridge));
        let stop = CancellationToken::new();
        // BEGIN scheduler slice: ordinary thread/launch intake.
        let scheduler = Arc::new(super::scheduler::Scheduler::new(
            kernel.store.clone(),
            Arc::new(super::scheduler::dispatch::ThreadDispatch {
                store: kernel.store.clone(),
                threads: threads.clone(),
                launch: None,
            }),
        ));
        scheduler.recover()?;
        sessions
            .mcp_server()
            .toolkit
            .set_scheduler(scheduler.clone());
        let mut workers = bridge.spawn_workers(stop.clone());
        // Start scheduling only after launch intake is installed below. A
        // first-tick unbound task must not race host assembly.
        // ── P4b queue/questions/lifecycle assembly ───────────────────────────
        let pull_requests = Arc::new(super::pull_requests::PullRequestService {
            kernel: kernel.clone(),
            host: Arc::new(super::pull_requests::host::GitHubHost::default()),
        });
        let queue_domain = Arc::new(super::queue::QueueDomain {
            kernel: kernel.clone(),
            links: pull_requests.clone(),
        });
        let queue_host = Arc::new(super::queue::host::HostQueue {
            domain: queue_domain.clone(),
            docs: doc_host.clone(),
            registry: registry.clone(),
        });
        sessions.mcp_server().set_queue_service(queue_host.clone());
        doc_host.set_orchestration_queue(Arc::downgrade(&queue_host));
        let queue_stop = stop.clone();
        workers.push(tokio::spawn(async move {
            loop {
                if queue_stop.is_cancelled() {
                    break;
                }
                if let Err(error) = queue_host.repair_all().await {
                    tracing::warn!(code=?error.code,"queue intent repair deferred");
                }
                if let Err(error) = queue_domain.wake_due(crate::now_ms()).await {
                    tracing::warn!(%error,"thread snooze wake failed");
                }
                tokio::select! {
                    _=queue_stop.cancelled()=>break,
                    _=tokio::time::sleep(std::time::Duration::from_millis(250))=>{}
                }
            }
        }));
        // ── end P4b assembly ────────────────────────────────────────────────
        // END scheduler slice.
        // -- PR links/watch/settlement (wave 3 pr-watch) --
        sessions
            .mcp_server()
            .set_pull_requests(pull_requests.clone());
        workers.push(
            super::pull_requests::reactor::PullRequestReactor::new(
                pull_requests.clone(),
                Arc::new(PrContext(workspace.clone())),
            )
            .spawn(stop.clone()),
        );
        // -- end PR links/watch/settlement --
        let publisher = PublicationWorker {
            store: kernel.store.clone(),
            publisher: Arc::new(ChatPublisher {
                doc_host,
                workspace,
                device_id,
                registry: registry.clone(),
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
            threads,
            scheduler,
            launch: None,
            pull_requests,
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

    /// -- wave-3 launch assembly --
    pub fn install_launch(
        &mut self,
        repos: crate::Repos,
        actions: crate::ProjectActionsStore,
        terminals: crate::Terminals,
        registry: Arc<HarnessRegistry>,
        data_dir: std::path::PathBuf,
    ) -> Result<()> {
        let service = Arc::new(super::launch::HostLaunchService::new(
            self.bridge.kernel.clone(),
            self.bridge.workspace.clone(),
            repos,
            actions,
            terminals,
            registry,
            data_dir,
            Arc::new(HostLaunchIntake {
                bridge: self.bridge.clone(),
                threads: self.threads.clone(),
            }),
        )?);
        self.bridge
            .sessions
            .mcp_server()
            .toolkit
            .set_launch_service(service.clone());
        service
            .projects()
            .map_err(|e| Error::Invariant(e.message))?;
        service
            .recover_preparations()
            .map_err(|e| Error::Invariant(e.message))?;
        self.scheduler
            .set_dispatcher(Arc::new(super::scheduler::dispatch::ThreadDispatch {
                store: self.bridge.kernel.store.clone(),
                threads: self.threads.clone(),
                launch: Some(service.clone()),
            }));
        self.workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(self.scheduler.spawn(self.stop.clone()));
        self.launch = Some(service);
        Ok(())
    }
}

struct PrContext(WorkspaceHost);
impl super::pull_requests::reactor::PrThreadContext for PrContext {
    fn cwd(&self, thread: &OrchestrationV2AppThread) -> Option<std::path::PathBuf> {
        thread
            .worktree_path
            .as_ref()
            .filter(|p| std::path::Path::new(p).is_dir())
            .cloned()
            .or_else(|| {
                self.0
                    .chat(&thread.id.0)
                    .ok()
                    .flatten()
                    .and_then(|c| c.source_context.map(|s| s.repo_root).or(c.cwd))
            })
            .map(Into::into)
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
        request: &mut RunRequest,
        message_id: &str,
        harness: &dyn zeron_harness::Harness,
    ) -> Result<CallerScope> {
        let thread = &scope.thread_id;
        super::transfer::ensure_start_allowed(
            &self.kernel.store.thread_transfers(thread)?,
            thread,
            false,
        )?;
        // Sessions publishes Idle before this independent observer commits
        // Done. Bound the admission race without inventing another active run.
        for _ in 0..100 {
            let active = self.kernel.store.thread(thread)?.is_some_and(|p| {
                p.runs.iter().any(|run| {
                    !super::command::run_terminal(&run.status)
                        && run.status != OrchestrationV2RunStatus::Queued
                })
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
        let project_root = self
            .workspace
            .space(&scope.project_id.0)
            .ok()
            .flatten()
            .map(|s| s.path);
        projection.thread.worktree_path = if project_root.as_deref() == Some(request.cwd.as_str()) {
            None
        } else {
            Some(request.cwd.clone())
        };
        let update = Command {
            id: CommandId(format!("session-binding:{message_id}")),
            thread_id: thread.clone(),
            operation: super::command::Operation::SessionBinding(Box::new(projection.thread)),
        };
        let receipt = self.kernel.dispatch(&update, crate::now_ms()).await?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        let queued = self.kernel.store.thread(thread)?.is_some_and(|p| {
            p.runs.iter().any(|r| {
                r.user_message_id.0 == message_id && r.status == OrchestrationV2RunStatus::Queued
            })
        });
        if queued {
            // TODO(merge-threads): reuse this admission primitive in thread.send.
            let domain = super::queue::QueueDomain::new(self.kernel.clone());
            let receipt = domain
                .mutate(
                    None,
                    thread.clone(),
                    "host.adopt_loro_delivery",
                    json!({"messageId":message_id,
                        "driver":crate::provider_instances::legacy_driver(harness.id())}),
                    CommandId(format!("session-run:{message_id}")),
                    crate::now_ms(),
                )
                .await?;
            if receipt.status == super::ReceiptStatus::Rejected {
                return Err(Error::Invariant(receipt.error.unwrap_or_default()));
            }
        } else {
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
        }
        let projection = self.kernel.store.thread(thread)?.unwrap();
        // Registry admission supplies canonical project identity, including
        // old projectless rows whose saved cwd differs from an expanded "~".
        scope.project_id = projection.thread.project_id.clone();
        let run = projection
            .runs
            .iter()
            .find(|r| r.user_message_id.0 == message_id)
            .unwrap()
            .clone();
        scope.run_id = run.id.clone();
        if let Err(error) = (super::checkpoint::FileCheckpointService {
            kernel: self.kernel.clone(),
        })
        .capture_turn(thread, &run, "started")
        .await
        {
            tracing::warn!(%error,"turn-start file checkpoint unavailable");
        }
        self.observe_external(thread.clone(), run, capabilities(harness))
            .await?;
        Ok(scope)
    }

    pub(crate) async fn prepare_external_turn(
        &self,
        thread: &ThreadId,
        run_id: &RunId,
        request: &mut RunRequest,
        harness: &dyn zeron_harness::Harness,
    ) -> Result<()> {
        let projection = self
            .kernel
            .store
            .thread(thread)?
            .ok_or_else(|| Error::Invariant("Transfer thread missing.".into()))?;
        let run = projection
            .runs
            .iter()
            .find(|run| &run.id == run_id)
            .ok_or_else(|| Error::Invariant("Transfer run missing.".into()))?;
        let prepared = super::transfer::delivery::prepare_run(
            &self.kernel,
            thread,
            run,
            request,
            harness,
            &capabilities(harness),
            self.sessions.registered_mcp(&thread.0).unwrap_or_default(),
        )
        .await;
        if let Err(error) = prepared {
            self.record_event(
                thread,
                run,
                0,
                zeron_proto::AgentEvent::Done {
                    status: zeron_proto::DoneStatus::Errored,
                    result: None,
                    error: Some(error.to_string()),
                    session_id: None,
                },
                None,
            )
            .await?;
            self.settle(thread, run).await?;
            return Err(error);
        }
        Ok(())
    }
}

struct ChatPublisher {
    doc_host: DocHost,
    workspace: WorkspaceHost,
    device_id: String,
    registry: Arc<HarnessRegistry>,
}

/// Materialize browsing identity without starting or discovering a provider.
/// Forks need their inherited selection and checkout *before* their first send;
/// otherwise the composer silently chooses its own default harness/cwd.
pub(crate) fn materialize_thread(
    workspace: &WorkspaceHost,
    registry: &HarnessRegistry,
    thread: &OrchestrationV2AppThread,
) -> Result<()> {
    let existing = workspace
        .chat(&thread.id.0)
        .map_err(|e| Error::Invariant(e.to_string()))?;
    if existing
        .as_ref()
        .is_some_and(|c| c.device_id != workspace.device_id())
    {
        return Err(Error::NotOwner);
    }
    let parent = thread
        .lineage
        .parent_thread_id
        .as_ref()
        .and_then(|id| workspace.chat(&id.0).ok().flatten());
    let space = parent
        .as_ref()
        .and_then(|c| c.space_id.clone())
        .or_else(|| {
            workspace
                .space(&thread.project_id.0)
                .ok()
                .flatten()
                .map(|s| s.id)
        });
    let cwd = thread
        .worktree_path
        .clone()
        .or_else(|| parent.as_ref().and_then(|c| c.cwd.clone()))
        .or_else(|| {
            space
                .as_ref()
                .and_then(|id| workspace.space(id).ok().flatten())
                .map(|s| s.path)
        });
    if space
        .as_ref()
        .and_then(|id| workspace.space(id).ok().flatten())
        .is_some_and(|s| s.device_id != workspace.device_id())
    {
        return Err(Error::NotOwner);
    }
    let provider = registry
        .provider_instances
        .snapshot(registry)
        .into_iter()
        .find(|p| p.provider_instance_id == thread.provider_instance_id);
    let harness = provider.and_then(|p| p.harness_id).or_else(|| {
        parent
            .as_ref()
            .and_then(|c| c.config.as_ref())
            .filter(|c| c.instance_id.as_ref() == Some(&thread.provider_instance_id))
            .map(|c| c.harness)
    });
    let config = harness.map(|harness| {
        let selection = json!(thread.model_selection);
        let options: serde_json::Map<_, _> = selection["options"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|o| Some((o["id"].as_str()?.to_owned(), o["value"].clone())))
            .collect();
        let reasoning_key = match harness {
            zeron_proto::HarnessId::ClaudeCode => "effort",
            zeron_proto::HarnessId::Pi => "thinking",
            _ => "reasoningEffort",
        };
        zeron_proto::ChatConfig {
            instance_id: Some(thread.provider_instance_id.clone()),
            harness,
            model: Some(thread.model_selection.model.clone()),
            reasoning: options
                .get(reasoning_key)
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            model_options: options,
            sandbox: parent
                .as_ref()
                .and_then(|c| c.config.as_ref())
                .map(|c| c.sandbox)
                .unwrap_or(zeron_proto::SandboxLevel::WorkspaceWrite),
            runtime_mode: thread.runtime_mode,
            interaction_mode: thread.interaction_mode,
        }
    });
    workspace
        .create_chat(
            &thread.id.0,
            space.as_deref(),
            Some(workspace.device_id()),
            config.clone(),
            cwd,
        )
        .map_err(|e| Error::Invariant(e.to_string()))?;
    if existing.is_none() {
        workspace
            .rename_chat(&thread.id.0, &thread.title)
            .map_err(|e| Error::Invariant(e.to_string()))?;
        if let Some(branch) = &thread.branch {
            workspace
                .set_chat_branch(&thread.id.0, branch)
                .map_err(|e| Error::Invariant(e.to_string()))?;
        }
    } else if existing.as_ref().is_some_and(|c| c.config.is_none())
        && thread.lineage.parent_thread_id.is_some()
        && let Some(config) = config
    {
        workspace
            .set_chat_config(&thread.id.0, &config)
            .map_err(|e| Error::Invariant(e.to_string()))?;
    }
    Ok(())
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
        materialize_thread(&self.workspace, &self.registry, &thread)?;
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
