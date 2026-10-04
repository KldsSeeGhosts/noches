//! Durable thread parking: pin, snooze and settle (T3 `t3_thread_organize`).
//!
//! Kept beside, not on, [`crate::Chat`]: the registry row stays the same and
//! a chat with no lifecycle entry is simply unpinned, awake and unsettled.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettleSource {
    User,
    Auto,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatLifecycle {
    /// Pinned when set; pinned rows sort by this instant, oldest first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snoozed_until: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_by: Option<SettleSource>,
    /// Set when a snooze elapses; cleared once the user acknowledges it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub woke_at: Option<DateTime<Utc>>,
}

impl ChatLifecycle {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
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

    pub fn woke(&self) -> bool {
        self.woke_at.is_some()
    }
}
