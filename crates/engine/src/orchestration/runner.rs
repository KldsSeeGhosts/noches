//! Kernel effect executor over the ordinary sessions runtime and transcript.
//! Instance resolution/MCP issuance are host services, never serialized secrets.
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::json;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use zeron_harness::Harness;
use zeron_proto::orchestration::*;
use zeron_proto::provider_instance::ProviderInstanceId;
use zeron_proto::{AgentEvent, ChatConfig, DoneStatus, RunRequest, SandboxLevel};

use super::command::{Command, Plan, run_terminal};
use super::effects::{Effect, EffectExecutor, EffectOutcome, EffectRequest, EffectWorker};
use super::event::{encode_component, iso};
use super::mailbox::{DeliveryAction, DeliveryCommand, current_delivery};
use super::projection::ThreadProjection;
use super::service::{CallerScope, ToolError};
use super::task::{TaskOperation, records};
use super::{Error, Kernel, Result};
use crate::doc_host::DocHost;
use crate::sessions::{SessionsEngine, SteerOutcome};
use crate::workspace_host::WorkspaceHost;

pub struct RunnerProvider {
    /// The configured instance's actual adapter (executable/environment/auth).
    pub harness: Arc<dyn Harness>,
    pub capabilities: OrchestrationV2ProviderCapabilities,
}

#[async_trait]
pub trait RunnerInstances: Send + Sync {
    async fn resolve(
        &self,
        instance: &ProviderInstanceId,
    ) -> std::result::Result<RunnerProvider, ToolError>;
}

#[async_trait]
pub trait RunnerMcp: Send + Sync {
    /// Must mint a fresh session-scoped credential for this exact child run,
    /// register it through `register_session_mcp`, and attach its revoker.
    /// Called before provider dispatch; failure refuses the start.
    async fn bind(
        &self,
        scope: CallerScope,
        sessions: &SessionsEngine,
    ) -> std::result::Result<(), ToolError>;
}

#[derive(Clone)]
pub struct RunnerBridge {
    pub kernel: Kernel,
    pub sessions: SessionsEngine,
    pub doc_host: DocHost,
    pub workspace: WorkspaceHost,
    pub device_id: String,
    pub instances: Arc<dyn RunnerInstances>,
    pub mcp: Arc<dyn RunnerMcp>,
}

impl RunnerBridge {
    pub(crate) async fn observe_external(
        &self,
        thread: ThreadId,
        run: OrchestrationV2Run,
        capabilities: OrchestrationV2ProviderCapabilities,
    ) -> Result<()> {
        let (_, receiver) = self
            .sessions
            .subscribe(&thread.0, u64::MAX)
            .map_err(|e| Error::Invariant(e.to_string()))?;
        let bridge = self.clone();
        let (accepted, _) = oneshot::channel();
        tokio::spawn(async move {
            if let Err(error) = bridge
                .observe(thread, run, capabilities, receiver, accepted)
                .await
            {
                tracing::error!(%error, "ordinary orchestration run observation failed");
            }
        });
        Ok(())
    }
    /// Call before legacy journal recovery, and before enabling V2 adoption.
    pub fn attach_recovery_gate(&self) {
        self.sessions
            .set_orchestration_store(self.kernel.store.clone());
    }

    /// Startup only, under the engine's InstanceLock, before MCP tools/workers
    /// become callable. Process-bound uncertain starts are disposed, never
    /// replayed; undelivered mail becomes a continuation with the same ID.
    pub async fn recover_mailbox(&self) -> Result<super::recovery::RecoverySummary> {
        if self.sessions.any_active() {
            return Err(Error::Invariant(
                "Startup recovery cannot run over live sessions.".into(),
            ));
        }
        self.attach_recovery_gate();
        let summary = self.kernel.recover(crate::now_ms()).await?;
        for effect in self.kernel.store.effects()? {
            if effect.status == super::effects::EffectStatus::Uncertain
                && matches!(
                    effect.request,
                    EffectRequest::ProviderTurnStart { .. }
                        | EffectRequest::ManagedRunInterrupt { .. }
                )
            {
                // Owning process is gone and kernel recorded terminal disposal;
                // no provider-specific replay/acceptance inference is needed.
                self.kernel
                    .store
                    .resolve_uncertain(&effect.id, false, crate::now_ms())?;
            }
        }
        self.kernel.reconcile_delegation().await?;
        let threads = self.kernel.store.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads ORDER BY id")?;
            Ok(stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })?;
        for thread in threads {
            let thread = ThreadId(thread);
            let projection = self.kernel.store.thread(&thread)?.unwrap();
            for run in &projection.runs {
                let delivery = super::mailbox::cohort(run)["delivery"].clone();
                if delivery.is_null() {
                    continue;
                }
                let message_id = MessageId(delivery["messageId"].as_str().unwrap().into());
                let completed = projection
                    .runs
                    .iter()
                    .find(|run| run.user_message_id == message_id && run_terminal(&run.status));
                let action = if let Some(completed) = completed {
                    DeliveryAction::Completed {
                        cancelled: completed.status == OrchestrationV2RunStatus::Cancelled,
                    }
                } else {
                    DeliveryAction::Recover
                };
                self.kernel
                    .task_command(
                        &thread,
                        CommandId(format!("command:mailbox-recover:{}", uuid::Uuid::new_v4())),
                        TaskOperation::Delivery(DeliveryCommand {
                            parent_run_id: run.id.clone(),
                            generation: delivery["generation"].as_i64().unwrap(),
                            message_id,
                            action,
                        }),
                    )
                    .await?;
            }
            self.kernel
                .task_command(
                    &thread,
                    CommandId(format!(
                        "command:mailbox-recover-drain:{}",
                        uuid::Uuid::new_v4()
                    )),
                    TaskOperation::DrainQueue,
                )
                .await?;
        }
        Ok(summary)
    }

    /// Four acceptance workers, not a four-session limit. The owner shuts down
    /// this token before retiring sessions/the engine graph.
    pub fn spawn_workers(
        self: &Arc<Self>,
        stop: CancellationToken,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        (0..super::effects::DEFAULT_WORKER_CONCURRENCY).map(|index| {
            let bridge = self.clone();
            let stop = stop.clone();
            tokio::spawn(async move {
                let worker = EffectWorker::new(bridge.kernel.store.clone(),bridge,format!("delegation-{index}"));
                loop {
                    if stop.is_cancelled() {break;}
                    let step = tokio::select! {
                        _ = stop.cancelled() => break,
                        step = worker.step(crate::now_ms()) => step,
                    };
                    match step {
                        Ok(true) => {}
                        Ok(false) => tokio::select! {
                            _ = stop.cancelled() => break,
                            _ = tokio::time::sleep(std::time::Duration::from_millis(25)) => {},
                        },
                        Err(error) => {
                            tracing::error!(%error,"orchestration effect worker failed");
                            tokio::select! {
                                _ = stop.cancelled() => break,
                                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {},
                            }
                        }
                    }
                }
            })
        }).collect()
    }

    async fn delivery_command(
        &self,
        thread: &ThreadId,
        effect: &Effect,
        input: DeliveryCommand,
        suffix: &str,
    ) -> Result<()> {
        let receipt = self.kernel.store.dispatch(
            &Command {
                thread_id: thread.clone(),
                id: CommandId(format!(
                    "command:mailbox:{}:{suffix}",
                    encode_component(&effect.id)
                )),
                operation: super::command::Operation::Task(Box::new(TaskOperation::Delivery(
                    input,
                ))),
            },
            crate::now_ms(),
        )?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(())
    }

    async fn start(
        &self,
        effect: &Effect,
        run_id: &RunId,
        cancellation: CancellationToken,
    ) -> Result<EffectOutcome> {
        let projection = self
            .kernel
            .store
            .thread(&effect.thread_id)?
            .ok_or_else(|| Error::Invariant("Runner thread missing.".into()))?;
        let Some(run) = projection
            .runs
            .iter()
            .find(|run| &run.id == run_id && run.status == OrchestrationV2RunStatus::Starting)
        else {
            return Ok(EffectOutcome::Succeeded);
        };
        let run = run.clone();
        let provider = self
            .instances
            .resolve(&run.provider_instance_id)
            .await
            .map_err(|error| Error::Invariant(error.to_string()))?;
        let harness = provider.harness.clone();
        zeron_harness::policy::compile(
            harness.id(),
            projection.thread.runtime_mode,
            projection.thread.interaction_mode,
        )
        .map_err(|error| Error::Invariant(error.to_string()))?;
        if self.sessions.turn_in_flight(&effect.thread_id.0) {
            // SQL and process disagree: never replace a live parent to run mail.
            return Ok(EffectOutcome::Retry);
        }
        // Retire only idle wrappers before changing the exact instance binding
        // or its session credential. Native conversation resume stays intact.
        self.sessions
            .interrupt(&effect.thread_id.0)
            .await
            .map_err(|error| Error::Invariant(error.to_string()))?;
        self.sessions
            .bind_provider_instance(
                &effect.thread_id.0,
                &run.provider_instance_id.0,
                harness.clone(),
            )
            .map_err(|error| Error::Invariant(error.to_string()))?;
        let cwd = projection.thread.worktree_path.as_deref().ok_or_else(|| {
            Error::Invariant("Runner requires a resolved workspace binding.".into())
        })?;
        let parent_space = projection
            .thread
            .lineage
            .parent_thread_id
            .as_ref()
            .and_then(|id| self.workspace.chat(&id.0).ok().flatten())
            .and_then(|chat| chat.space_id);
        let options_value = serde_json::to_value(&run.model_selection)?;
        let mut options = serde_json::Map::new();
        if let Some(selections) = options_value["options"].as_array() {
            for selection in selections {
                if let Some(id) = selection["id"].as_str() {
                    options.insert(id.into(), selection["value"].clone());
                }
            }
        }
        let reasoning_key = match harness.id() {
            zeron_proto::HarnessId::ClaudeCode => "effort",
            zeron_proto::HarnessId::Pi => "thinking",
            _ => "reasoningEffort",
        };
        let reasoning = options
            .get(reasoning_key)
            .filter(|value| value.is_string())
            .map(|value| serde_json::from_value::<zeron_proto::ReasoningLevel>(value.clone()))
            .transpose()?;
        let config = ChatConfig {
            harness: harness.id(),
            model: Some(run.model_selection.model.clone()),
            reasoning,
            model_options: options.clone(),
            sandbox: SandboxLevel::WorkspaceWrite,
            runtime_mode: projection.thread.runtime_mode,
            interaction_mode: projection.thread.interaction_mode,
        };
        self.workspace
            .create_chat(
                &effect.thread_id.0,
                parent_space.as_deref(),
                Some(&self.device_id),
                Some(config.clone()),
                Some(cwd.into()),
            )
            .map_err(|error| Error::Invariant(error.to_string()))?;
        self.workspace
            .set_chat_config(&effect.thread_id.0, &config)
            .map_err(|error| Error::Invariant(error.to_string()))?;
        self.workspace
            .rename_chat(&effect.thread_id.0, &projection.thread.title)
            .map_err(|error| Error::Invariant(error.to_string()))?;
        if let Some(branch) = &projection.thread.branch {
            self.workspace
                .set_chat_branch(&effect.thread_id.0, branch)
                .map_err(|error| Error::Invariant(error.to_string()))?;
        }
        let scope = CallerScope {
            thread_id: effect.thread_id.clone(),
            run_id: run.id.clone(),
            session_id: format!("provider-session:{}", encode_component(&run.id.0)),
            project_id: projection.thread.project_id.clone(),
            workspace_root: cwd.into(),
            runtime_mode: projection.thread.runtime_mode,
            interaction_mode: projection.thread.interaction_mode,
            provider_instance_id: run.provider_instance_id.clone(),
        };
        self.mcp
            .bind(scope, &self.sessions)
            .await
            .map_err(|error| Error::Invariant(error.to_string()))?;
        let prompt = records(&projection, "message")
            .iter()
            .find(|message| message["id"] == run.user_message_id.0)
            .and_then(|message| message["text"].as_str())
            .ok_or_else(|| Error::Invariant("Run input missing.".into()))?;
        let request: RunRequest = serde_json::from_value(json!({
            "prompt":prompt,"harness":harness.id(),"model":run.model_selection.model,"reasoning":reasoning,
            "modelOptions":options,"cwd":cwd,"sandbox":"workspace-write","autoApprove":false,
            "runtimeMode":projection.thread.runtime_mode,"interactionMode":projection.thread.interaction_mode,"resume":null
        }))?;
        if cancellation.is_cancelled() {
            return Ok(EffectOutcome::Succeeded);
        }
        // Subscribe before dispatch. A zero-latency mock/provider can finish
        // before dispatch returns, so acceptance must already have a consumer.
        let (_, receiver) = self
            .sessions
            .subscribe(&effect.thread_id.0, u64::MAX)
            .map_err(|error| Error::Invariant(error.to_string()))?;
        let (accepted_tx, accepted_rx) = oneshot::channel();
        let bridge = self.clone();
        let thread = effect.thread_id.clone();
        let observer_run = run.clone();
        tokio::spawn(async move {
            if let Err(error) = bridge
                .observe(
                    thread,
                    observer_run,
                    provider.capabilities,
                    receiver,
                    accepted_tx,
                )
                .await
            {
                tracing::error!(%error,"delegated run observation failed");
            }
        });
        if let Err(error) = self
            .sessions
            .dispatch_orchestrated(
                &effect.thread_id.0,
                harness.id(),
                request,
                Some(run.user_message_id.0.clone()),
            )
            .await
        {
            self.record_event(
                &effect.thread_id,
                &run,
                0,
                AgentEvent::Done {
                    status: DoneStatus::Errored,
                    result: None,
                    error: Some(error.to_string()),
                    session_id: None,
                },
                None,
            )
            .await?;
            self.settle(&effect.thread_id, &run).await?;
            return Ok(EffectOutcome::Failed);
        }
        // SessionStarted (or a native error/Done) is provider acceptance, not
        // the completion of its task. The observer remains independently live.
        tokio::select! {
            response = accepted_rx => Ok(response.unwrap_or(EffectOutcome::Uncertain)),
            _ = cancellation.cancelled() => Ok(EffectOutcome::Uncertain),
        }
    }

    async fn record_event(
        &self,
        thread: &ThreadId,
        run: &OrchestrationV2Run,
        sequence: u64,
        event: AgentEvent,
        capabilities: Option<OrchestrationV2ProviderCapabilities>,
    ) -> Result<()> {
        self.kernel
            .task_command(
                thread,
                CommandId(format!(
                    "command:runner:{}:{sequence}",
                    encode_component(&run.id.0)
                )),
                TaskOperation::RunnerEvent {
                    run_id: run.id.clone(),
                    attempt_id: run
                        .active_attempt_id
                        .clone()
                        .ok_or_else(|| Error::Invariant("Run attempt missing.".into()))?,
                    event,
                    capabilities: capabilities.map(Box::new),
                },
            )
            .await
    }

    async fn observe(
        &self,
        thread: ThreadId,
        run: OrchestrationV2Run,
        capabilities: OrchestrationV2ProviderCapabilities,
        mut receiver: tokio::sync::broadcast::Receiver<crate::sessions::JournaledEvent>,
        accepted: oneshot::Sender<EffectOutcome>,
    ) -> Result<()> {
        let mut accepted = Some(accepted);
        let mut sequence = 0;
        let mut pending = std::collections::VecDeque::new();
        loop {
            let event = if let Some(event) = pending.pop_front() {
                event
            } else {
                match receiver.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let (replay, new_receiver) =
                            self.sessions
                                .subscribe(&thread.0, sequence)
                                .map_err(|error| Error::Invariant(error.to_string()))?;
                        receiver = new_receiver;
                        // Replayed terminal/acceptance/native-progress events must
                        // take exactly the same settlement path as live events.
                        pending.extend(replay);
                        continue;
                    }
                }
            };
            if event.seq <= sequence {
                continue;
            }
            sequence = event.seq;
            let terminal = matches!(event.event, AgentEvent::Done { .. });
            let native_progress = matches!(event.event, AgentEvent::Subagent { .. });
            let is_acceptance = matches!(
                event.event,
                AgentEvent::SessionStarted { .. }
                    | AgentEvent::TextDelta { .. }
                    | AgentEvent::Done { .. }
            );
            self.record_event(
                &thread,
                &run,
                event.seq,
                event.event,
                Some(capabilities.clone()),
            )
            .await?;
            if is_acceptance && let Some(accepted) = accepted.take() {
                let _ = accepted.send(EffectOutcome::Succeeded);
            }
            if terminal {
                self.settle(&thread, &run).await?;
                // Native background observations may arrive after root Done.
                // Keep the receiver until another app run takes ownership.
            }
            let current = self.kernel.store.thread(&thread)?.unwrap();
            if native_progress {
                self.kernel.reconcile_ancestors(&thread).await?;
            }
            if super::task::progress(&current).0 == "result_available" {
                break;
            }
            if current
                .runs
                .last()
                .is_some_and(|latest| latest.id != run.id)
            {
                break;
            }
        }
        Ok(())
    }

    async fn settle(&self, thread: &ThreadId, run: &OrchestrationV2Run) -> Result<()> {
        let projection = self.kernel.store.thread(thread)?.unwrap();
        if let Some(message) = records(&projection, "message")
            .iter()
            .find(|message| message["id"] == run.user_message_id.0)
            && let Some(parent) = message["delegatedCompletion"]["parentRunId"].as_str()
        {
            let input = DeliveryCommand {
                parent_run_id: RunId(parent.into()),
                generation: message["delegatedCompletion"]["generation"]
                    .as_i64()
                    .unwrap(),
                message_id: run.user_message_id.clone(),
                action: DeliveryAction::Completed {
                    cancelled: projection
                        .runs
                        .iter()
                        .find(|candidate| candidate.id == run.id)
                        .is_some_and(|run| run.status == OrchestrationV2RunStatus::Cancelled),
                },
            };
            self.kernel
                .task_command(
                    thread,
                    CommandId(format!(
                        "command:delivery-complete:{}",
                        encode_component(&run.id.0)
                    )),
                    TaskOperation::Delivery(input),
                )
                .await?;
        }
        self.kernel.reconcile_ancestors(thread).await?;
        self.kernel
            .task_command(
                thread,
                CommandId(format!(
                    "command:terminal-mailbox:{}:{}",
                    encode_component(&run.id.0),
                    uuid::Uuid::new_v4()
                )),
                TaskOperation::Reconcile,
            )
            .await?;
        self.kernel
            .task_command(
                thread,
                CommandId(format!(
                    "command:queue-drain:{}:{}",
                    encode_component(&run.id.0),
                    uuid::Uuid::new_v4()
                )),
                TaskOperation::DrainQueue,
            )
            .await
    }

    async fn execute_inner(
        &self,
        effect: &Effect,
        cancellation: CancellationToken,
    ) -> Result<EffectOutcome> {
        match &effect.request {
            EffectRequest::ProviderTurnStart { run_id } => {
                self.start(effect, run_id, cancellation).await
            }
            EffectRequest::ManagedRunInterrupt { run_id } => {
                let projection = self.kernel.store.thread(&effect.thread_id)?.unwrap();
                let Some(run) = projection
                    .runs
                    .iter()
                    .find(|run| &run.id == run_id && !run_terminal(&run.status))
                else {
                    return Ok(EffectOutcome::Succeeded);
                };
                let run = run.clone();
                self.sessions
                    .interrupt(&effect.thread_id.0)
                    .await
                    .map_err(|error| Error::Invariant(error.to_string()))?;
                self.record_event(
                    &effect.thread_id,
                    &run,
                    u64::MAX,
                    AgentEvent::Done {
                        status: DoneStatus::Interrupted,
                        result: None,
                        error: None,
                        session_id: None,
                    },
                    None,
                )
                .await?;
                self.settle(&effect.thread_id, &run).await?;
                Ok(EffectOutcome::Succeeded)
            }
            EffectRequest::DelegatedCompletionContinue {
                parent_run_id,
                generation,
                message_id,
            } => {
                // Serialize cohort generation/membership/liveness and provider
                // acceptance under the parent lock, without holding SQLite.
                let _guards = self.kernel.locks.acquire([effect.thread_id.clone()]).await;
                let input = DeliveryCommand {
                    parent_run_id: parent_run_id.clone(),
                    generation: *generation,
                    message_id: message_id.clone(),
                    action: DeliveryAction::Queue,
                };
                let projection = self.kernel.store.thread(&effect.thread_id)?.unwrap();
                if current_delivery(&projection, &input).is_none() {
                    return Ok(EffectOutcome::Succeeded);
                }
                let can_steer = super::mailbox::can_steer_delivery(&projection, &input)
                    && self.sessions.turn_in_flight(&effect.thread_id.0);
                if can_steer {
                    self.delivery_command(
                        &effect.thread_id,
                        effect,
                        DeliveryCommand {
                            action: DeliveryAction::BeginSteer,
                            ..input.clone()
                        },
                        "steer",
                    )
                    .await?;
                    let projection = self.kernel.store.thread(&effect.thread_id)?.unwrap();
                    if current_delivery(&projection, &input).is_none() {
                        return Ok(EffectOutcome::Succeeded);
                    }
                    let text = records(&projection, "message")
                        .iter()
                        .find(|message| message["id"] == message_id.0)
                        .unwrap()["text"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    if self
                        .sessions
                        .steer_notification(&effect.thread_id.0, &text, message_id.0.clone())
                        .await
                        .map_err(|error| Error::Invariant(error.to_string()))?
                        == SteerOutcome::Accepted
                    {
                        self.delivery_command(
                            &effect.thread_id,
                            effect,
                            DeliveryCommand {
                                action: DeliveryAction::Accepted,
                                ..input
                            },
                            "accepted",
                        )
                        .await?;
                        return Ok(EffectOutcome::Succeeded);
                    }
                }
                self.delivery_command(&effect.thread_id, effect, input, "queue")
                    .await?;
                Ok(EffectOutcome::Succeeded)
            }
            // Other workstreams add these executors. Refuse, never silently
            // mark unrelated provider/control/cleanup work as successful.
            _ => Err(Error::Invariant(
                "Effect executor is unavailable for this request.".into(),
            )),
        }
    }
}

#[async_trait]
impl EffectExecutor for RunnerBridge {
    async fn execute(&self, effect: &Effect, cancellation: CancellationToken) -> EffectOutcome {
        match self.execute_inner(effect, cancellation).await {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::error!(effect=%effect.id,%error,"orchestration runner effect failed");
                if matches!(
                    effect.request,
                    EffectRequest::DelegatedCompletionContinue { .. }
                ) {
                    return EffectOutcome::Retry; // same mail identity, at-least-once acceptance
                }
                if let EffectRequest::ProviderTurnStart { run_id } = &effect.request
                    && let Ok(Some(projection)) = self.kernel.store.thread(&effect.thread_id)
                    && let Some(run) = projection.runs.iter().find(|run| &run.id == run_id)
                {
                    if let Err(report_error) = self
                        .record_event(
                            &effect.thread_id,
                            run,
                            0,
                            AgentEvent::Done {
                                status: DoneStatus::Errored,
                                result: None,
                                error: Some(error.to_string()),
                                session_id: None,
                            },
                            None,
                        )
                        .await
                    {
                        tracing::error!(%report_error,"failed start terminal report failed");
                    } else if let Err(settle_error) = self.settle(&effect.thread_id, run).await {
                        tracing::error!(%settle_error,"failed start mailbox settlement failed");
                    }
                }
                EffectOutcome::Failed
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_event(
    _conn: &Connection,
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    run_id: &RunId,
    attempt_id: &RunAttemptId,
    event: &AgentEvent,
    capabilities: Option<&OrchestrationV2ProviderCapabilities>,
    now: i64,
) -> Result<()> {
    let Some(run) = projection
        .runs
        .iter()
        .find(|run| &run.id == run_id && run.active_attempt_id.as_ref() == Some(attempt_id))
    else {
        return Ok(());
    };
    // Persistent drivers emit init once, then confirm later turns with native
    // user/steer boundaries. Attach the retained actual session to this new
    // logical attempt only upon that confirmation, not at mailbox enqueue.
    if run.status == OrchestrationV2RunStatus::Starting
        && matches!(
            event,
            AgentEvent::Steered { .. } | AgentEvent::UserMessage { .. }
        )
        && let Some(previous) = records(projection, "provider-thread")
            .iter()
            .find(|p| p["id"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str()))
        && let Some(native) = previous["nativeThreadRef"]["nativeId"].as_str()
    {
        return plan_event(
            _conn,
            projection,
            command,
            plan,
            run_id,
            attempt_id,
            &AgentEvent::SessionStarted {
                harness: zeron_proto::HarnessId::Mock, // not stored; driver is the exact binding above
                model: run.model_selection.model.clone(),
                tools: vec![],
                cwd: projection.thread.worktree_path.clone().unwrap_or_default(),
                session_id: native.into(),
                assistant_message_id: String::new(),
            },
            capabilities,
            now,
        );
    }
    let Some(attempt) = projection
        .attempts
        .iter()
        .find(|attempt| &attempt.id == attempt_id)
    else {
        return Ok(());
    };
    if run_terminal(&run.status)
        && !(run.status == OrchestrationV2RunStatus::Completed
            && matches!(event, AgentEvent::Subagent { .. }))
    {
        return Ok(());
    }
    let mut run = serde_json::to_value(run)?;
    let mut attempt = serde_json::to_value(attempt)?;
    let root = projection
        .nodes
        .iter()
        .find(|node| node.id.0 == run["rootNodeId"].as_str().unwrap())
        .unwrap();
    let mut node = serde_json::to_value(root)?;
    let mut provider = records(projection, "provider-thread")
        .iter()
        .find(|provider| provider["id"] == run["providerThreadId"])
        .cloned()
        .ok_or_else(|| Error::Invariant("Runner provider binding missing.".into()))?;
    if provider["lastRunOrdinal"] != run["ordinal"] {
        return Ok(());
    }
    let turn_id = format!("provider-turn:{}", encode_component(&attempt_id.0));
    let mut turn = json!({"id":turn_id,"providerThreadId":provider["id"],"nodeId":node["id"],
        "runAttemptId":attempt_id,"nativeTurnRef":null,"ordinal":run["ordinal"],"status":"running","startedAt":iso(now)?,"completedAt":null});
    match event {
        AgentEvent::SessionStarted {
            session_id,
            model,
            cwd,
            ..
        } => {
            let session = format!("provider-session:{}", encode_component(&run_id.0));
            let capabilities = capabilities
                .ok_or_else(|| Error::Invariant("Live provider capabilities missing.".into()))?;
            let session = json!({"id":session,"driver":provider["driver"],"providerInstanceId":run["providerInstanceId"],
                "status":"running","cwd":cwd,"model":model,"capabilities":capabilities,"createdAt":iso(now)?,"updatedAt":iso(now)?,"lastError":null});
            plan.emit(command, "provider-session.attached", &session, now)?;
            provider["providerSessionId"] = session["id"].clone();
            provider["nativeThreadRef"] =
                json!({"driver":provider["driver"],"nativeId":session_id,"strength":"weak"});
            provider["status"] = json!("active");
            provider["updatedAt"] = json!(iso(now)?);
            run["startedAt"] = json!(iso(now)?);
            attempt["startedAt"] = json!(iso(now)?);
            node["startedAt"] = json!(iso(now)?);
            attempt["nativeThreadId"] = json!(session_id);
            attempt["providerTurnId"] = json!(turn_id);
            node["providerTurnId"] = json!(turn_id);
            run["status"] = json!("running");
            attempt["status"] = json!("running");
            node["status"] = json!("running");
            plan.emit(command, "provider-thread.updated", &provider, now)?;
            plan.emit(command, "provider-turn.updated", &turn, now)?;
        }
        AgentEvent::TextDelta { text } => {
            let id = format!("message:assistant:{}", encode_component(&run_id.0));
            let mut message = records(projection, "message")
                .iter()
                .find(|message| message["id"] == id)
                .cloned()
                .unwrap_or(super::task::message(
                    &projection.thread.id,
                    Some(run_id),
                    Some(&root.id),
                    &id,
                    "",
                    "assistant",
                    now,
                )?);
            let next = format!("{}{}", message["text"].as_str().unwrap_or(""), text);
            message["text"] = json!(next);
            message["streaming"] = json!(true);
            message["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "message.updated", &message, now)?;
            return Ok(());
        }
        AgentEvent::InputRequested { .. } | AgentEvent::PermissionRequested { .. } => {
            run["status"] = json!("waiting");
            node["status"] = json!("waiting");
        }
        AgentEvent::InputResolved { .. } | AgentEvent::PermissionUpdated { .. } => {
            run["status"] = json!("running");
            node["status"] = json!("running");
        }
        AgentEvent::Done {
            status,
            result,
            error,
            ..
        } => {
            let status = match status {
                DoneStatus::Completed => "completed",
                DoneStatus::Interrupted => "interrupted",
                DoneStatus::Errored => "failed",
            };
            for row in [&mut run, &mut attempt, &mut node, &mut turn] {
                row["status"] = json!(status);
                row["completedAt"] = json!(iso(now)?);
            }
            if matches!(status, "interrupted" | "failed") {
                // The interrupted provider process can no longer settle its
                // native children/roster. App-owned descendants are separate
                // runtimes and are intentionally not recursively killed here.
                provider["pendingBackgroundTasks"] = json!([]);
                for task in records(projection, "subagent").iter().filter(|task| {
                    task["origin"] == "provider_native"
                        && task["runId"] == run_id.0
                        && !super::task::terminal(task["status"].as_str().unwrap_or("running"))
                }) {
                    let mut task = task.clone();
                    task["status"] = json!(status);
                    task["completedAt"] = json!(iso(now)?);
                    task["updatedAt"] = json!(iso(now)?);
                    plan.emit(command, "subagent.updated", &task, now)?;
                }
            }
            let id = format!("message:assistant:{}", encode_component(&run_id.0));
            let mut message = records(projection, "message")
                .iter()
                .find(|message| message["id"] == id)
                .cloned()
                .unwrap_or(super::task::message(
                    &projection.thread.id,
                    Some(run_id),
                    Some(&root.id),
                    &id,
                    "",
                    "assistant",
                    now,
                )?);
            if let Some(result) = result {
                message["text"] = json!(result);
            }
            message["streaming"] = json!(false);
            message["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "message.updated", &message, now)?;
            if let Some(error) = error {
                let item = json!({"id":format!("turn-item:error:{}",encode_component(&run_id.0)),"type":"error",
                    "threadId":projection.thread.id,"runId":run_id,"nodeId":root.id,"providerThreadId":provider["id"],
                    "providerTurnId":attempt["providerTurnId"],"nativeItemRef":null,"parentItemId":null,
                    "ordinal":records(projection,"turn-item").len()+1,"status":"failed","title":null,
                    "startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
                    "failure":{"class":"provider_error","message":error,"code":null,"retryable":null}});
                plan.emit(command, "turn-item.updated", &item, now)?;
            }
            provider["status"] = json!("idle");
            provider["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "provider-thread.updated", &provider, now)?;
            if !attempt["providerTurnId"].is_null() {
                plan.emit(command, "provider-turn.updated", &turn, now)?;
            }
        }
        AgentEvent::Subagent {
            parent_tool_use_id,
            event,
        } => {
            // Observational provider-native identity. Never create a delegated
            // child run, transfer, receipt, or automatic completion cohort.
            let id = format!(
                "node:provider-native:{}:{}",
                encode_component(&run_id.0),
                encode_component(parent_tool_use_id)
            );
            let mut task = records(projection,"subagent").iter().find(|task| task["id"] == id).cloned().unwrap_or(json!({
                "id":id,"threadId":projection.thread.id,"runId":run_id,"parentNodeId":root.id,
                "origin":"provider_native","createdBy":"agent","driver":provider["driver"],
                "providerInstanceId":run["providerInstanceId"],"providerThreadId":provider["id"],"childThreadId":null,
                "nativeTaskRef":null,"prompt":"","title":null,"model":run["modelSelection"]["model"],"status":"running",
                "result":null,"startedAt":iso(now)?,"completedAt":null,"updatedAt":iso(now)?
            }));
            if let AgentEvent::Done {
                status,
                result,
                error,
                ..
            } = event.as_ref()
            {
                task["status"] = json!(match status {
                    DoneStatus::Completed => "completed",
                    DoneStatus::Interrupted => "interrupted",
                    DoneStatus::Errored => "failed",
                });
                task["result"] = json!(error.as_ref().or(result.as_ref()));
                task["completedAt"] = json!(iso(now)?);
            }
            task["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "subagent.updated", &task, now)?;
            return Ok(());
        }
        _ => return Ok(()),
    }
    plan.emit(command, "run.updated", &run, now)?;
    plan.emit(command, "run-attempt.updated", &attempt, now)?;
    plan.emit(command, "node.updated", &node, now)?;
    Ok(())
}
