//! Runtime authority is independent of planning interaction.
//! Wire spellings and defaults match T3 `contracts/providerPolicy.ts`.
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    ApprovalRequired,
    AutoAcceptEdits,
    Auto,
    // Also the explicit legacy migration: old booleans did not constrain runs.
    #[default]
    FullAccess,
}

impl RuntimeMode {
    pub const ALL: [Self; 4] = [
        Self::ApprovalRequired,
        Self::AutoAcceptEdits,
        Self::Auto,
        Self::FullAccess,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ApprovalRequired => "Supervised",
            Self::AutoAcceptEdits => "Auto-accept",
            Self::Auto => "Auto",
            Self::FullAccess => "Full access",
        }
    }

    pub fn permits(self, child: Self) -> bool {
        child <= self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionMode {
    #[default]
    Default,
    Plan,
}

impl InteractionMode {
    pub fn permits(self, child: Self) -> bool {
        self == Self::Default || child == Self::Plan
    }
}

/// Exact T3 approval decisions. Persistent grants are never synthesized from
/// session grants; adapters must expose only decisions they actually support.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionDecision {
    Accept,
    AcceptForSession,
    AcceptAlways,
    Decline,
    #[default]
    Cancel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionScope {
    #[default]
    Once,
    Session,
    Persistent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestState {
    #[default]
    Pending,
    Resolved,
    Expired,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PermissionOption {
    /// Stable provider option identity, never the display label.
    pub id: String,
    pub label: String,
    pub decision: PermissionDecision,
    pub scope: DecisionScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PermissionRequest {
    /// Engine-minted, scoped to one live callback. Native IDs stay host-local.
    pub id: String,
    pub tool: String,
    pub description: String,
    pub options: Vec<PermissionOption>,
    pub state: RequestState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_option_id: Option<String>,
}

impl PermissionRequest {
    pub fn standard(
        tool: impl Into<String>,
        description: impl Into<String>,
        session: bool,
    ) -> Self {
        let mut options = vec![PermissionOption {
            id: "allow-once".into(),
            label: "Allow once".into(),
            decision: PermissionDecision::Accept,
            ..Default::default()
        }];
        if session {
            options.push(PermissionOption {
                id: "allow-session".into(),
                label: "Allow for session".into(),
                decision: PermissionDecision::AcceptForSession,
                scope: DecisionScope::Session,
                ..Default::default()
            });
        }
        options.push(PermissionOption {
            id: "deny".into(),
            label: "Deny".into(),
            decision: PermissionDecision::Decline,
            ..Default::default()
        });
        Self {
            tool: tool.into(),
            description: description.into(),
            options,
            ..Default::default()
        }
    }
}

/// Content questions are not permissions and cannot approve a tool request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UserInputRequest {
    pub id: String,
    pub questions: Vec<crate::UserInputQuestion>,
    pub state: RequestState,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t3_wire_modes_defaults_and_narrowing() {
        for (mode, wire) in RuntimeMode::ALL.into_iter().zip([
            "approval-required",
            "auto-accept-edits",
            "auto",
            "full-access",
        ]) {
            assert_eq!(serde_json::to_value(mode).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<RuntimeMode>(wire.into()).unwrap(),
                mode
            );
            assert!(RuntimeMode::FullAccess.permits(mode));
        }
        assert_eq!(RuntimeMode::default(), RuntimeMode::FullAccess);
        assert_eq!(
            serde_json::to_value(InteractionMode::default()).unwrap(),
            "default"
        );
        assert!(!RuntimeMode::ApprovalRequired.permits(RuntimeMode::Auto));
        assert!(!InteractionMode::Plan.permits(InteractionMode::Default));
        assert_eq!(
            serde_json::from_str::<PermissionRequest>("{}")
                .unwrap()
                .state,
            RequestState::Pending
        );
    }

    #[test]
    fn legacy_booleans_preserve_effective_full_access_and_unknown_modes_refuse() {
        for sandbox in ["read-only", "workspace-write", "danger-full-access"] {
            for auto in [false, true] {
                let old = serde_json::json!({
                    "prompt":"old","model":null,"reasoning":null,"cwd":"/tmp",
                    "sandbox":sandbox,"autoApprove":auto,"resume":null
                });
                let request: crate::RunRequest = serde_json::from_value(old).unwrap();
                assert_eq!(request.runtime_mode, RuntimeMode::FullAccess);
                assert_eq!(request.interaction_mode, InteractionMode::Default);
            }
        }
        let old: crate::ChatConfig = serde_json::from_value(serde_json::json!({
            "harness":"codex","model":null,"reasoning":null,"sandbox":"workspace-write"
        }))
        .unwrap();
        assert_eq!(old.runtime_mode, RuntimeMode::FullAccess);
        assert!(serde_json::from_value::<RuntimeMode>("yolo".into()).is_err());
        let question = UserInputRequest {
            id: "q".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::from_value::<UserInputRequest>(serde_json::to_value(&question).unwrap())
                .unwrap(),
            question
        );
    }
}
