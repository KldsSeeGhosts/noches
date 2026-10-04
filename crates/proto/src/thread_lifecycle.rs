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
    pub thread_id: String,
    pub version: i64,
    pub queue: Vec<QueueUiEntry>,
    pub pending_questions: Vec<PendingQuestionUi>,
    pub lifecycle: ChatLifecycle,
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
    fn old_lifecycle_loads_with_defaults() {
        let value: super::ChatLifecycle = serde_json::from_str("{}").unwrap();
        assert!(value.is_default());
        assert_eq!(
            serde_json::to_value(super::SettleSource::User).unwrap(),
            "User"
        );
    }
}
