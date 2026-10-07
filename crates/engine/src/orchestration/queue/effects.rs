//! Queue-only effect adapters. Direct steering never falls back to dispatch.
//! An admitted interrupt/restart promotion uses the shared control executor.
use serde_json::json;
use zeron_proto::orchestration::*;

use crate::SessionsEngine;
use crate::orchestration::effects::{Effect, EffectOutcome, EffectRequest};
use crate::orchestration::runner::RunnerBridge;
use crate::orchestration::{Error, Result, task};

/// TODO(merge-threads): ThreadService has no existing-message delivery,
/// question-answer, or detach primitive. Its send creates new activity and
/// permits late-steer recovery/restart, unlike queue promotion. Retain this
/// strict adapter until the shared boundary exposes these lower-level effects.
pub trait QueueThreadDelivery: Send + Sync {
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

impl QueueThreadDelivery for HostThreadDelivery {
    fn detach(&self, thread: &str, revoke_mcp: bool) {
        self.0.request_orchestration_detach(thread, revoke_mcp);
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
        EffectRequest::ProviderTurnSteer { .. } => {
            super::super::steering::execute(bridge, effect, false).await
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
