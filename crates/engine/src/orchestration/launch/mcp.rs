use super::{HostLaunchService, failure, unavailable};
use crate::mcp::{auth::InvocationScope, codec};
use crate::orchestration::launch_service::LaunchService;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn dispatch(
    service: Option<Arc<dyn LaunchService>>,
    scope: &InvocationScope,
    name: &str,
    input: Value,
) -> Value {
    codec::result(match service {
        Some(service) => service.call(scope, name, input).await,
        None if matches!(name, "t3_worktree_status" | "t3_worktree_handoff") => {
            super::worktree_failure(
                "operation_failed",
                format!(
                    "Unable to read thread {}: The operation could not be completed.",
                    scope.caller.thread_id
                ),
            )
        }
        None => codec::unavailable(),
    })
}

#[async_trait]
impl LaunchService for HostLaunchService {
    async fn call(&self, scope: &InvocationScope, name: &str, input: Value) -> Value {
        if name.starts_with("t3_worktree_") && !scope.capabilities.contains("worktree") {
            return if name == "t3_worktree_list" {
                failure(super::ToolError::new(
                    zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode::CapabilityDenied,
                    "This credential cannot inspect worktrees.",
                ))
            } else {
                super::worktree_failure(
                    "capability_denied",
                    "This MCP credential does not grant worktree capabilities.",
                )
            };
        }
        if !matches!(name, "t3_worktree_handoff" | "t3_worktree_status")
            && !scope.capabilities.contains("orchestration")
        {
            return failure(super::ToolError::new(
                zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode::CapabilityDenied,
                "This credential cannot control threads.",
            ));
        }
        if name == "t3_worktree_handoff" {
            return self.handoff(scope, input).await;
        }
        if name == "t3_worktree_status" {
            return self.worktree_status(scope);
        }
        let result = match name {
            "t3_project_list" | "t3_project_read" | "t3_project_create" | "t3_project_update"
            | "t3_project_delete" | "t3_project_clone" => {
                self.project_call(scope, name, input).await
            }
            "t3_environment_read" | "t3_environment_preferences_update" => {
                self.environment(scope, name, input)
            }
            "t3_attachment_prepare_upload" => self.prepare_upload(scope, &input),
            "t3_attachment_discard" => self.discard(scope, &input),
            "t3_thread_send_attachments" => self.send_attachments(scope, input).await,
            "t3_thread_launch" => self.launch(scope, input).await,
            "t3_worktree_list" => self.worktree_list(scope, input).await,
            _ => Err(unavailable()),
        };
        result.unwrap_or_else(failure)
    }
    async fn upload(&self, token: &str, bytes: &[u8]) -> (u16, Value) {
        self.store_upload(token, bytes)
    }
    async fn cleanup(
        &self,
        thread: &str,
        attachment_ids: Option<Vec<String>>,
    ) -> Result<(), super::ToolError> {
        self.cleanup_owned(thread, attachment_ids)
            .map_err(|_| unavailable())
    }
}

impl HostLaunchService {
    fn environment(
        &self,
        scope: &InvocationScope,
        name: &str,
        mut patch: Value,
    ) -> Result<Value, super::ToolError> {
        if scope.environment_id != self.workspace.device_id() {
            self.caller(scope, name != "t3_environment_read")?;
            return Err(super::ToolError::new(
                zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode::CapabilityDenied,
                "This credential belongs to another environment.",
            ));
        }
        if name == "t3_environment_read" {
            self.caller(scope, false)?;
            return Ok(
                json!({"environmentId":scope.environment_id,"label":gethostname::gethostname().to_string_lossy(),
                "serverVersion":env!("CARGO_PKG_VERSION"),"platform":{
                    "os":match std::env::consts::OS{"macos"=>"darwin","linux"=>"linux","windows"=>"windows",_=>"unknown"},
                    "arch":match std::env::consts::ARCH{"aarch64"=>"arm64","x86_64"=>"x64",_=>"other"}
                },
                "preferences":self.preference_view()?}),
            );
        }
        self.require_full(
            scope,
            "Preference updates require a live full-access/default thread.",
        )?;
        if let Some(text) = patch["sourceControlWritingStyle"]["customInstructions"].as_str() {
            patch["sourceControlWritingStyle"]["customInstructions"] =
                json!(text.trim_matches(|c: char| matches!(c,
                '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' |
                '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' |
                '\u{205F}' | '\u{3000}' | '\u{FEFF}')));
        }
        self.kernel.store.write(|tx| {
            use rusqlite::OptionalExtension;
            let stored: Option<String> = tx.query_row("SELECT payload FROM orchestration_launch_preferences WHERE singleton=1", [], |r| r.get(0)).optional()?;
            let mut current = stored.map(|s| serde_json::from_str(&s)).transpose()?.unwrap_or(json!({
                "defaultThreadEnvMode":null,"newWorktreesStartFromOrigin":true,"enableProviderUpdateChecks":true,
                "backgroundActivity":{"profile":"balanced"},"sourceControlWritingStyle":{"mode":"repo_conventions","followChangeRequestTemplates":true,"customInstructions":""}
            }));
            merge(&mut current, &patch);
            tx.execute("INSERT INTO orchestration_launch_preferences VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET payload=excluded.payload", [current.to_string()])?;
            Ok(())
        }).map_err(|_| unavailable())?;
        self.preference_view()
    }
}

fn merge(current: &mut Value, patch: &Value) {
    if let (Some(current), Some(patch)) = (current.as_object_mut(), patch.as_object()) {
        for (key, value) in patch {
            if value.is_object() && current.get(key).is_some_and(Value::is_object) {
                merge(current.get_mut(key).unwrap(), value);
            } else {
                current.insert(key.clone(), value.clone());
            }
        }
    }
}
