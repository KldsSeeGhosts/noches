//! MCP-aware native fork hook for the later engine lifecycle layer. No turn
//! is started here; a resumed child gets a newly registered session scope.

use super::*;
use crate::mcp::SessionMcpContext;

impl CodexHarness {
    pub async fn fork_session_with_mcp(
        &self,
        source_thread: &str,
        cwd: &str,
        mcp: SessionMcpContext,
    ) -> Result<String, HarnessError> {
        let _scope = mcp.run_guard();
        let executable = self.resolve_executable()?;
        let mut command = Command::new(&executable);
        command.arg("app-server");
        crate::compose_child_environment(&mut command, &executable);
        if !cwd.is_empty() {
            command.current_dir(cwd);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let (client, mut incoming) = RpcClient::new(
            child.stdin.take().expect("piped"),
            child.stdout.take().expect("piped"),
        );
        let result = tokio::time::timeout(Duration::from_secs(60), async {
            request(
                &client,
                &mut incoming,
                "initialize",
                json!({
                    "clientInfo":{"name":"zeron-native","version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"experimentalApi":true}
                }),
            )
            .await?;
            client.notify("initialized", None);
            let mut params =
                json!({"threadId":source_thread,"config":mcp.codex_thread_overrides()});
            if !cwd.is_empty() {
                params["cwd"] = cwd.into();
            }
            if !mcp.instructions().is_empty() {
                params["developerInstructions"] = mcp.instructions().into();
            }
            let response = request(&client, &mut incoming, "thread/fork", params).await?;
            response["thread"]["id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    HarnessError::Protocol("thread/fork returned no native thread id".into())
                })
        })
        .await
        .unwrap_or_else(|_| Err(HarnessError::Protocol("thread/fork timed out".into())));
        shutdown_child(&mut child, self.kill_grace).await;
        result
    }
}

async fn request(
    client: &RpcClient,
    incoming: &mut mpsc::Receiver<Incoming>,
    method: &str,
    params: Value,
) -> Result<Value, HarnessError> {
    let result = client.request(method, params);
    tokio::pin!(result);
    loop {
        tokio::select! {
            result = &mut result => return result,
            message = incoming.recv() => match message {
                Some(Incoming::Request { id, method, .. }) if method == "mcpServer/elicitation/request" => client.respond(&id, json!({"action":"decline"})),
                Some(Incoming::Request { id, .. }) => client.respond_error(&id, -32601, "no active turn during thread/fork"),
                Some(Incoming::Eof) | None => return Err(HarnessError::Protocol("app-server exited during thread/fork".into())),
                _ => {},
            }
        }
    }
}
