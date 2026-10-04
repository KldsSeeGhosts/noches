//! Host-owned project, launch, upload and worktree orchestration.
pub mod attachments;
pub(crate) mod deletion;
pub(crate) mod host_intake;
pub mod mcp;
pub(crate) mod planner;
mod projects;
pub mod setup;
mod workflow;

use super::launch_service::LaunchThreadIntake;
use super::{Kernel, Result, service::ToolError};
use crate::mcp::auth::InvocationScope;
use crate::{HarnessRegistry, ProjectActionsStore, Repos, Terminals, WorkspaceHost};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use zeron_proto::orchestration::OrchestrationV2AppThread;
use zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode as Code;

pub use planner::LaunchOperation;

#[derive(Clone)]
pub struct HostLaunchService {
    pub kernel: Kernel,
    pub workspace: WorkspaceHost,
    pub repos: Repos,
    pub actions: ProjectActionsStore,
    pub terminals: Terminals,
    pub registry: Arc<HarnessRegistry>,
    pub data_dir: PathBuf,
    pub intake: Arc<dyn LaunchThreadIntake>,
    in_flight: Arc<Mutex<std::collections::HashSet<String>>>,
    preparation_locks: Arc<crate::orchestration::ThreadLocks>,
    pub(crate) setup_cancels:
        Arc<Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>>,
    pub(crate) signing_key: Vec<u8>,
}

impl HostLaunchService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel: Kernel,
        workspace: WorkspaceHost,
        repos: Repos,
        actions: ProjectActionsStore,
        terminals: Terminals,
        registry: Arc<HarnessRegistry>,
        data_dir: PathBuf,
        intake: Arc<dyn LaunchThreadIntake>,
    ) -> Result<Self> {
        let key_path = data_dir.join("attachment-upload-signing-key");
        // Private host-local key. Never put secrets in SQL, projections or RPC.
        let signing_key = match std::fs::read(&key_path) {
            Ok(key) if key.len() == 32 => key,
            Ok(_) => {
                return Err(super::Error::Invariant(
                    "Invalid attachment signing key.".into(),
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                use std::io::Write;
                let mut key = vec![0; 32];
                getrandom::fill(&mut key).map_err(invariant)?;
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                match options.open(&key_path) {
                    Ok(mut file) => {
                        file.write_all(&key).map_err(invariant)?;
                        key
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        std::fs::read(&key_path).map_err(invariant)?
                    }
                    Err(e) => return Err(invariant(e)),
                }
            }
            Err(e) => return Err(invariant(e)),
        };
        Ok(Self {
            kernel,
            workspace,
            repos,
            actions,
            terminals,
            registry,
            data_dir,
            intake,
            in_flight: Arc::default(),
            preparation_locks: Arc::default(),
            setup_cancels: Arc::default(),
            signing_key,
        })
    }

    fn caller(
        &self,
        scope: &InvocationScope,
        mutation: bool,
    ) -> std::result::Result<OrchestrationV2AppThread, ToolError> {
        let p = self
            .kernel
            .store
            .thread(&scope.caller.thread_id)
            .map_err(|_| unavailable())?
            .ok_or_else(|| {
                ToolError::new(Code::ThreadNotFound, "The calling thread was not found.")
            })?;
        if p.thread.deleted_at.is_some() {
            return Err(ToolError::new(
                Code::ThreadNotFound,
                "The calling thread was not found.",
            ));
        }
        if mutation
            && (p.thread.archived_at.is_some()
                || p.thread.provider_instance_id != scope.caller.provider_instance_id
                || !p.runs.iter().any(|r| {
                    r.id == scope.caller.run_id && !super::command::run_terminal(&r.status)
                }))
        {
            return Err(ToolError::new(
                Code::ParentNotActive,
                "The calling provider no longer owns an active thread run.",
            ));
        }
        Ok(p.thread)
    }

    fn require_full(
        &self,
        scope: &InvocationScope,
        message: &str,
    ) -> std::result::Result<OrchestrationV2AppThread, ToolError> {
        let caller = self.caller(scope, true)?;
        if caller.archived_at.is_some()
            || caller.runtime_mode != zeron_proto::RuntimeMode::FullAccess
            || caller.interaction_mode != zeron_proto::InteractionMode::Default
        {
            return Err(ToolError::new(Code::CapabilityDenied, message));
        }
        Ok(caller)
    }

    pub(crate) fn rows(&self, table: &str) -> Result<Vec<Value>> {
        self.kernel.store.read(|conn| {
            let mut stmt = conn.prepare(&format!("SELECT payload FROM {table} ORDER BY rowid"))?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
        })
    }

    pub(crate) fn save(&self, table: &str, key: &str, value: &Value) -> Result<()> {
        let column = if table == "orchestration_launch_workflows" {
            "thread_id"
        } else {
            "id"
        };
        self.kernel.store.write(|tx| {
            tx.execute(&format!("INSERT INTO {table}({column},payload) VALUES(?1,?2) ON CONFLICT({column}) DO UPDATE SET payload=excluded.payload"),
                rusqlite::params![key, value.to_string()])?;
            Ok(())
        })
    }

    pub fn preferences(&self) -> Result<Value> {
        Ok(self.rows("orchestration_launch_preferences")?.into_iter().next().unwrap_or_else(|| json!({
            "defaultThreadEnvMode":null,"newWorktreesStartFromOrigin":true,
            "enableProviderUpdateChecks":true,"backgroundActivity":{"profile":"balanced"},
            "sourceControlWritingStyle":{"mode":"repo_conventions","followChangeRequestTemplates":true,"customInstructions":""}
        })))
    }

    fn preference_view(&self) -> std::result::Result<Value, ToolError> {
        let mut p = self.preferences().map_err(|_| unavailable())?;
        let text = p["sourceControlWritingStyle"]["customInstructions"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        p["sourceControlWritingStyle"]["customInstructions"] =
            json!(text.chars().take(4000).collect::<String>());
        p["sourceControlWritingStyle"]["truncated"] = json!(text.chars().count() > 4000);
        Ok(p)
    }
}

pub(crate) fn invariant(e: impl std::fmt::Display) -> super::Error {
    super::Error::Invariant(e.to_string())
}
pub(crate) fn unavailable() -> ToolError {
    ToolError::new(
        Code::OrchestrationError,
        "The operation could not be completed.",
    )
}
pub(crate) fn invalid(message: impl Into<String>) -> ToolError {
    ToolError::new(Code::InvalidRequest, message)
}
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
pub(crate) fn id() -> String {
    crate::new_id()
}
pub(crate) fn failure(error: ToolError) -> Value {
    serde_json::to_value(error.into_failure()).expect("failure")
}
pub(crate) fn worktree_failure(code: &str, message: impl Into<String>) -> Value {
    json!({"_tag":"WorktreeMcpFailure","code":code,"message":message.into()})
}
