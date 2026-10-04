//! Host-local launch overrides. Never mutate the engine's process environment.
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Default)]
pub struct InstanceLaunch {
    pub environment: BTreeMap<String, String>,
    pub args: Vec<String>,
    pub(crate) secrets: Option<Arc<crate::redact::RegisteredSecrets>>,
}

impl InstanceLaunch {
    pub fn new(environment: BTreeMap<String, String>, args: Vec<String>) -> Self {
        Self {
            environment,
            args,
            ..Default::default()
        }
    }
    pub fn with_secrets(mut self, values: Vec<String>) -> Self {
        self.secrets = Some(Arc::new(crate::redact::register_secrets(values)));
        self
    }

    /// Apply after host/shell environment composition, before app-owned MCP
    /// overrides. Each driver owns a separate value and discovery cache.
    pub(crate) fn apply(&self, command: &mut tokio::process::Command) {
        command.envs(&self.environment);
    }

    pub(crate) fn apply_launch(&self, command: &mut tokio::process::Command) {
        self.apply(command);
        command.args(&self.args);
    }
}

pub fn resolve_binary(value: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(value);
    if path.is_absolute() || path.components().count() > 1 {
        crate::executable::validate_native_override(path).ok()
    } else {
        crate::executable::find_on_paths(value, Vec::new())
    }
}

impl std::fmt::Debug for InstanceLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceLaunch")
            .field(
                "environment_names",
                &self.environment.keys().collect::<Vec<_>>(),
            )
            .field("args", &"[host-local]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_are_command_local_and_secrets_are_redacted() {
        let launch = InstanceLaunch {
            environment: BTreeMap::from([("CODEX_HOME".into(), "/instance/one".into())]),
            ..Default::default()
        }
        .with_secrets(vec!["instance-secret".into()]);
        let mut first = tokio::process::Command::new("codex");
        let second = tokio::process::Command::new("codex");
        launch.apply(&mut first);
        assert!(first.as_std().get_envs().any(|(k, _)| k == "CODEX_HOME"));
        assert!(!second.as_std().get_envs().any(|(k, _)| k == "CODEX_HOME"));
        assert_eq!(
            crate::redact::redact_registered("instance-secret"),
            "[redacted]"
        );
        assert!(!format!("{launch:?}").contains("instance-secret"));
    }
}
