//! Queue-only effect adapters. No fallback dispatch, restart, or interruption.
use async_trait::async_trait;
use serde_json::json;
use zeron_proto::orchestration::*;

use crate::orchestration::effects::{Effect, EffectOutcome, EffectRequest};
use crate::orchestration::runner::RunnerBridge;
use crate::orchestration::{Error, Result, task};
use crate::{SessionsEngine, SteerOutcome};

/// TODO(merge-threads): ThreadService has no existing-message delivery,
/// question-answer, or detach primitive. Its send creates new activity and
/// permits late-steer recovery/restart, unlike queue promotion. Retain this
/// strict adapter until the shared boundary exposes these lower-level effects.
#[async_trait]
pub trait QueueThreadDelivery: Send + Sync {
    async fn steer(&self, thread: &str, message_id: String, text: &str) -> Result<bool>;
    fn detach(&self, thread: &str, revoke_mcp: bool);
    fn answer(
        &self,
        thread: &str,
        request: &str,
        answers: &ProviderUserInputAnswers,
    ) -> Result<bool>;
}

pub struct HostThreadDelivery(pub SessionsEngine);

pub(crate) fn is_promotion(store: &crate::orchestration::Store, effect: &Effect) -> Result<bool> {
    Ok(store
        .receipt(&effect.command_id)?
        .is_some_and(|receipt| receipt.command_type == "queued-message.promote-to-steer"))
}

#[async_trait]
impl QueueThreadDelivery for HostThreadDelivery {
    fn detach(&self, thread: &str, revoke_mcp: bool) {
        self.0.request_orchestration_detach(thread, revoke_mcp);
    }
    async fn steer(&self, thread: &str, message_id: String, text: &str) -> Result<bool> {
        self.0
            .steer_notification(thread, text, message_id)
            .await
            .map(|result| result == SteerOutcome::Accepted)
            .map_err(|e| Error::Invariant(e.to_string()))
    }
    fn answer(
        &self,
        thread: &str,
        request: &str,
        answers: &ProviderUserInputAnswers,
    ) -> Result<bool> {
        let answers = answers
            .iter()
            .map(|(id, value)| {
                let labels = if let Some(text) = value.as_str() {
                    vec![text.into()]
                } else if let Some(values) = value.as_array() {
                    values
                        .iter()
                        .map(|v| {
                            v.as_str().map(str::to_owned).ok_or_else(|| {
                                Error::Invariant("User-input answer is invalid.".into())
                            })
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    return Err(Error::Invariant("User-input answer is invalid.".into()));
                };
                Ok(zeron_proto::UserInputAnswer {
                    question_id: id.clone(),
                    labels,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.0
            .respond_input(thread, request, answers)
            .map_err(|e| Error::Invariant(e.to_string()))
    }
}

pub(crate) async fn execute(bridge: &RunnerBridge, effect: &Effect) -> Result<EffectOutcome> {
    match &effect.request {
        EffectRequest::ProviderSessionDisconnect { run_id, .. } => {
            let _guards = bridge
                .kernel
                .locks
                .acquire([effect.thread_id.clone()])
                .await;
            let p = bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
            // Canonical replacement admission advances the credential scope
            // before publishing a runtime. This fences active AND idle sessions.
            if super::session_control::target_still_disconnected(&p, &effect.request) {
                bridge
                    .sessions
                    .request_orchestration_disconnect(&effect.thread_id.0, run_id);
            }
            Ok(EffectOutcome::Succeeded)
        }
        EffectRequest::ProviderSessionDetach {
            provider_session_id,
        } => {
            let _guards = bridge
                .kernel
                .locks
                .acquire([effect.thread_id.clone()])
                .await;
            let p = bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
            // A delayed detach for a previous run must not stop a re-engaged
            // session. Queued intents alone never transfer provider ownership.
            let owner = task::records(&p, "provider-thread")
                .iter()
                .find(|provider| provider["providerSessionId"] == provider_session_id.0);
            if owner.is_some()
                && !p.runs.iter().any(|run| {
                    !crate::orchestration::command::run_terminal(&run.status)
                        && run.status != OrchestrationV2RunStatus::Queued
                        && format!(
                            "provider-session:{}",
                            crate::orchestration::event::encode_component(&run.id.0)
                        ) != provider_session_id.0
                })
            {
                HostThreadDelivery(bridge.sessions.clone()).detach(
                    &effect.thread_id.0,
                    p.thread.archived_at.is_some() || p.thread.deleted_at.is_some(),
                );
            }
            Ok(EffectOutcome::Succeeded)
        }
        EffectRequest::ProviderTurnSteer {
            provider_session_id,
            provider_thread_id,
            provider_turn_id,
            message_id,
        } => {
            let _guards = bridge
                .kernel
                .locks
                .acquire([effect.thread_id.clone()])
                .await;
            let p = bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
            let message = task::records(&p, "message")
                .iter()
                .find(|m| m["id"] == message_id.0)
                .ok_or_else(|| Error::Invariant("Steering message was not found.".into()))?;
            let run = p.runs.iter().find(|r| {
                message["runId"] == r.id.0
                    && r.status == OrchestrationV2RunStatus::Running
                    && r.provider_thread_id.as_ref() == Some(provider_thread_id)
            });
            let turn = task::records(&p, "provider-turn").iter().find(|t| {
                t["id"] == provider_turn_id.0
                    && t["status"] == "running"
                    && run.is_some_and(|r| {
                        r.active_attempt_id
                            .as_ref()
                            .is_some_and(|a| t["runAttemptId"] == a.0)
                            && r.root_node_id.as_ref().is_some_and(|n| t["nodeId"] == n.0)
                    })
            });
            let provider = task::records(&p, "provider-thread").iter().find(|t| {
                t["id"] == provider_thread_id.0
                    && t["providerSessionId"] == provider_session_id.0
                    && run.is_some_and(|r| t["lastRunOrdinal"] == r.ordinal)
            });
            if p.thread.archived_at.is_some()
                || p.thread.deleted_at.is_some()
                || !task::records(&p, "provider-session")
                    .iter()
                    .any(|s| s["id"] == provider_session_id.0)
                || run.is_none()
                || turn.is_none()
                || provider.is_none()
                || !bridge.sessions.turn_in_flight(&effect.thread_id.0)
            {
                return Ok(EffectOutcome::Failed);
            }
            let mut text = message["text"].as_str().unwrap_or("").to_owned();
            // Promotion has already consumed its intent. Paths outlive the
            // next sync so a delayed steering effect still retains uploads.
            let paths = bridge.kernel.store.read(|conn| {
                crate::orchestration::ui_queue::attachment_paths(
                    conn,
                    &effect.thread_id,
                    &message_id.0,
                )
            })?;
            if !paths.is_empty() {
                text.push_str("\n\nAttachments:\n");
                text.push_str(
                    &paths
                        .iter()
                        .map(|p| format!("- {p}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            Ok(
                if HostThreadDelivery(bridge.sessions.clone())
                    .steer(&effect.thread_id.0, message_id.0.clone(), &text)
                    .await?
                {
                    EffectOutcome::Succeeded
                } else {
                    EffectOutcome::Failed
                },
            )
        }
        EffectRequest::RuntimeRequestRespond {
            request_id,
            answers,
            ..
        } => {
            let p = bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
            if !task::records(&p, "runtime-request")
                .iter()
                .any(|r| r["id"] == request_id.0 && r["kind"] == "user_input")
            {
                return Err(Error::Invariant(
                    "The pending user-input request was not found.".into(),
                ));
            }
            let answers = answers
                .as_ref()
                .ok_or_else(|| Error::Invariant("User-input answers are missing.".into()))?;
            Ok(
                if HostThreadDelivery(bridge.sessions.clone()).answer(
                    &effect.thread_id.0,
                    &request_id.0,
                    answers,
                )? {
                    EffectOutcome::Succeeded
                } else {
                    EffectOutcome::Failed
                },
            )
        }
        EffectRequest::ThreadTitleGenerate { .. } => {
            let p = bridge
                .kernel
                .store
                .thread(&effect.thread_id)?
                .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
            let thread = serde_json::to_value(&p.thread)?;
            let Some(request) = thread["titleRegeneration"]["requestId"].as_str() else {
                return Ok(EffectOutcome::Succeeded);
            };
            let request = request.to_owned();
            let provider = bridge
                .instances
                .resolve(&p.thread.provider_instance_id)
                .await
                .map_err(|e| Error::Invariant(e.message))?;
            let prompt = super::title::context(task::records(&p, "message"));
            let title = if prompt.is_empty() {
                None
            } else {
                bridge
                    .sessions
                    .generate_orchestration_title(
                        provider.harness.id(),
                        &p.thread.provider_instance_id,
                        &prompt,
                        p.thread.worktree_path.as_deref().unwrap_or(""),
                    )
                    .await
            };
            let domain = super::QueueDomain::new(bridge.kernel.clone());
            let receipt = domain
                .mutate(
                    None,
                    effect.thread_id.clone(),
                    "host.title_generated",
                    json!({"requestId":request,"title":title}),
                    CommandId(format!("title-complete:{}", effect.id)),
                    crate::now_ms(),
                )
                .await?;
            Ok(
                if receipt.status == crate::orchestration::ReceiptStatus::Accepted {
                    EffectOutcome::Succeeded
                } else {
                    EffectOutcome::Failed
                },
            )
        }
        _ => Err(Error::Invariant("Unsupported queue effect.".into())),
    }
}
