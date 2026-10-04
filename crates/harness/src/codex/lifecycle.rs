use super::*;
use crate::session_lifecycle::{NativeForkRequest, SessionLifecycle};
use async_trait::async_trait;
use serde_json::json;

#[async_trait]
impl SessionLifecycle for CodexHarness {
    fn can_fork_from_turn(&self) -> bool {
        true
    }
    async fn fork_thread(&self, request: NativeForkRequest) -> Result<String, HarnessError> {
        // Never fork at moving native head if the stable turn cursor is missing.
        // Existing pre-V2 histories are handled through portable context.
        let turn = request.source_turn_id.ok_or_else(|| {
            HarnessError::Protocol(
                "Cannot fork Codex at a specific turn without a native turn reference.".into(),
            )
        })?;
        let exe = self.resolve_executable()?;
        let mut command = Command::new(&exe);
        crate::compose_child_environment(&mut command, &exe);
        command
            .arg("app-server")
            .current_dir(&request.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            shutdown_child(&mut child, self.kill_grace).await;
            return Err(HarnessError::Protocol(
                "codex fork child has no stdio".into(),
            ));
        };
        let (client, mut incoming) = RpcClient::new(stdin, stdout);
        // Drain notifications and refuse unexpected provider->client requests.
        let drain_client = client.clone();
        let drain = tokio::spawn(async move {
            while let Some(event) = incoming.recv().await {
                if let Incoming::Request { id, .. } = event {
                    drain_client.respond_error(
                        &id,
                        -32601,
                        "No interactive fork request is supported",
                    );
                }
            }
        });
        let fork = async {
            client.request("initialize",json!({
                "clientInfo":{"name":"zeron-native","title":"Noches","version":env!("CARGO_PKG_VERSION")},
                "capabilities":{"experimentalApi":true}
            })).await?;
            client.notify("initialized", None);
            let policy = crate::policy::compile(
                HarnessId::Codex,
                request.runtime_mode,
                request.interaction_mode,
            )?;
            let result = client
                .request(
                    "thread/fork",
                    json!({
                        "threadId":request.source_thread_id,"lastTurnId":turn,"cwd":request.cwd,
                        "model":request.model,"approvalPolicy":policy.codex_approval,
                        "sandbox":sandbox_mode(policy.codex_sandbox),
                        "config":request.mcp.codex_thread_overrides(),
                        "developerInstructions":request.mcp.instructions()
                    }),
                )
                .await?;
            result["thread"]["id"]
                .as_str()
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    HarnessError::Protocol("Codex thread/fork returned no native thread ID.".into())
                })
        };
        let result = tokio::time::timeout(Duration::from_secs(30), fork).await;
        shutdown_child(&mut child, self.kill_grace).await;
        drain.abort();
        result.unwrap_or_else(|_| {
            Err(HarnessError::Protocol(
                "Codex thread/fork acceptance is uncertain.".into(),
            ))
        })
    }
}
