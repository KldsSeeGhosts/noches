use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatLifecycle {
    pub pinned_at: Option<DateTime<Utc>>,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub settled_by: Option<SettleSource>,
    pub woke_at: Option<DateTime<Utc>>,
}

impl ChatLifecycle {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn pinned(&self) -> bool {
        self.pinned_at.is_some()
    }

    /// Snoozed until a wake instant still in the future.
    pub fn snoozed(&self, now: DateTime<Utc>) -> bool {
        self.snoozed_until.is_some_and(|until| until > now)
    }

    pub fn settled(&self) -> bool {
        self.settled_at.is_some()
    }

    /// A snooze elapsed and the user has not acknowledged it yet.
    pub fn woke(&self) -> bool {
        self.woke_at.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettleSource {
    User,
    Auto,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueUiState {
    /// Zero denotes an older publisher without document provenance.
    pub schema_version: u32,
    pub thread_id: String,
    pub version: i64,
    pub queue: Vec<QueueUiEntry>,
    pub pending_questions: Vec<PendingQuestionUi>,
    pub lifecycle: ChatLifecycle,
    pub active_run_id: Option<String>,
    /// Completed foreground output can still own native background work.
    /// This is a passive hint; Stop rechecks ownership on the host.
    pub background_run_id: Option<String>,
    /// Legacy hint for non-interrupting promotion only.
    pub can_promote_to_steer: bool,
    /// The UI names an interrupting restart explicitly. Host admission checks
    /// the observed mode again, so stale Steer clicks cannot become restarts.
    pub promotion_mode: Option<QueuePromotionMode>,
    /// The saved next-turn selection the promotion would run on (restart modes),
    /// or would leave waiting (`promotion_selection_deferred`). Clients echo it
    /// back so the host can refuse an action reviewed against a stale selection.
    pub promotion_selection: Option<crate::provider_instance::ModelSelection>,
    /// Active steering keeps the live selection; the saved one applies next turn.
    pub promotion_selection_deferred: bool,
    /// Why a selection change cannot be delivered into the running turn.
    pub promotion_blocked: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueuePromotionMode {
    ActiveSteering,
    InterruptRestart,
    /// The saved selection needs a new provider generation seeded with context.
    InterruptRestartWithHandoff,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueUiEntry {
    pub queued_run_id: String,
    pub message_id: String,
    pub text: String,
    pub attachments: Vec<serde_json::Value>,
    pub attachment_paths: Vec<String>,
    pub held: bool,
    pub delivery_gate: Option<serde_json::Value>,
    pub automatic: bool,
    /// Loro rows keep their edit leases and attachment transport. Other rows
    /// are presentation-only; clients must never insert them into the document.
    pub document_backed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutateQueuedRunParams {
    pub chat_id: String,
    pub queued_run_id: String,
    pub client_request_id: String,
    pub action: QueuedRunAction,
    #[serde(default)]
    pub target_device_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QueuedRunAction {
    /// Text-only edit: attachments and context remain on the original message.
    Edit {
        text: String,
        expected_text: String,
    },
    Cancel,
    Reorder {
        before_run_id: Option<String>,
    },
    PromoteToSteer {
        target_run_id: String,
        #[serde(default)]
        expected_selection: Option<crate::provider_instance::ModelSelection>,
    },
    PromoteToRestart {
        target_run_id: String,
        /// The restart replaces the provider generation and carries a handoff.
        #[serde(default)]
        handoff: bool,
        #[serde(default)]
        expected_selection: Option<crate::provider_instance::ModelSelection>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutateQueuedRunResult {
    pub sequence: i64,
    pub refusal: Option<String>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingQuestionUi {
    pub request_id: String,
    pub questions: Vec<serde_json::Value>,
    pub response_type: String,
    pub answerable: bool,
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_queue_snapshots_do_not_claim_document_provenance_or_steering() {
        let state: super::QueueUiState =
            serde_json::from_str(r#"{"threadId":"old","queue":[{"messageId":"m","text":"work"}]}"#)
                .unwrap();
        assert_eq!(state.schema_version, 0);
        assert!(!state.can_promote_to_steer);
        assert!(state.active_run_id.is_none());
        assert!(state.background_run_id.is_none());
        assert!(state.promotion_mode.is_none());
        assert!(state.promotion_selection.is_none());
        assert!(!state.promotion_selection_deferred);
        assert!(state.promotion_blocked.is_none());
    }

    #[test]
    fn canonical_text_edits_require_the_original_text_and_stable_identity() {
        let value = serde_json::json!({"chatId":"thread","queuedRunId":"run",
            "clientRequestId":"stable", "action":{"type":"edit","text":"new"}});
        assert!(serde_json::from_value::<super::MutateQueuedRunParams>(value.clone()).is_err());
        let mut value = value;
        value["action"]["expectedText"] = serde_json::json!("old");
        let request: super::MutateQueuedRunParams = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.action,
            super::QueuedRunAction::Edit {
                text: "new".into(),
                expected_text: "old".into(),
            }
        );
    }

    #[test]
    fn old_lifecycle_loads_with_defaults() {
        let value: super::ChatLifecycle = serde_json::from_str("{}").unwrap();
        assert!(value.is_default());
        assert_eq!(
            serde_json::to_value(super::SettleSource::User).unwrap(),
            "User"
        );
    }

    #[test]
    fn promotion_actions_default_to_unreviewed_and_name_the_handoff_mode() {
        let steer: super::QueuedRunAction =
            serde_json::from_str(r#"{"type":"promoteToSteer","targetRunId":"run"}"#).unwrap();
        assert_eq!(
            steer,
            super::QueuedRunAction::PromoteToSteer {
                target_run_id: "run".into(),
                expected_selection: None,
            }
        );
        let restart: super::QueuedRunAction = serde_json::from_str(
            r#"{"type":"promoteToRestart","targetRunId":"run","handoff":true,
                "expectedSelection":{"instanceId":"claude","model":"opus"}}"#,
        )
        .unwrap();
        assert!(matches!(
            restart,
            super::QueuedRunAction::PromoteToRestart {
                handoff: true,
                expected_selection: Some(_),
                ..
            }
        ));
        assert_eq!(
            serde_json::to_string(&super::QueuePromotionMode::InterruptRestartWithHandoff).unwrap(),
            r#""interrupt_restart_with_handoff""#
        );
    }
}
