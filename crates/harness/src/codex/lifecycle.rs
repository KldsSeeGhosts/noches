use super::*;
use crate::session_lifecycle::{NativeForkRequest, SessionLifecycle};
use async_trait::async_trait;
use serde_json::json;

/// Trim a head fork back to a legacy boundary. Count-based rollback no longer
/// exists in Codex: the forked thread must use paginated history, and the
/// boundary is the oldest of the newest `turns` turns (`thread/revert` drops it
/// and everything after). A failure after the fork succeeded is reported as an
/// error so the caller keeps the fork uncertain instead of retrying at head.
async fn revert_forked_turns(
    client: &RpcClient,
    thread_id: &str,
    turns: usize,
) -> Result<String, HarnessError> {
    let read = client
        .request(
            "thread/read",
            json!({"threadId":thread_id,"includeTurns":false}),
        )
        .await?;
    if read["thread"]["historyMode"].as_str() != Some("paginated") {
        return Err(HarnessError::Protocol(
            "Cannot fork Codex at a legacy turn: the forked thread uses legacy history, \
             which Codex cannot revert."
                .into(),
        ));
    }
    let mut remaining = turns;
    let mut before_turn: Option<String> = None;
    let mut cursor: Option<String> = None;
    let mut visited = std::collections::HashSet::new();
    while remaining > 0 {
        if !visited.insert(cursor.clone()) {
            return Err(HarnessError::Protocol(
                "Codex thread history pagination repeated a cursor.".into(),
            ));
        }
        let page = client
            .request(
                "thread/turns/list",
                json!({
                    "threadId":thread_id,"cursor":cursor,"limit":remaining.min(100),
                    "sortDirection":"desc","itemsView":"summary"
                }),
            )
            .await?;
        for turn in page["data"].as_array().into_iter().flatten() {
            before_turn = turn["id"].as_str().map(str::to_owned);
            remaining -= 1;
            if remaining == 0 {
                break;
            }
        }
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    if remaining > 0 {
        return Err(HarnessError::Protocol(
            "Codex fork has fewer turns than the legacy boundary expects.".into(),
        ));
    }
    let Some(before_turn) = before_turn else {
        return Ok(thread_id.to_owned());
    };
    let reverted = client
        .request(
            "thread/revert",
            json!({"threadId":thread_id,"beforeTurnId":before_turn}),
        )
        .await?;
    Ok(reverted["thread"]["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .unwrap_or(thread_id)
        .to_owned())
}

#[async_trait]
impl SessionLifecycle for CodexHarness {
    fn can_fork_from_turn(&self) -> bool {
        true
    }
    fn supports_fork_rollback(&self) -> bool {
        true
    }
    async fn fork_thread(&self, request: NativeForkRequest) -> Result<String, HarnessError> {
        // A stable turn cursor forks atomically at the boundary. Without one,
        // only an explicit, counted legacy boundary may fork at head and then
        // be trimmed; never fork a moving head with an unknown boundary.
        let turn = request.source_turn_id.clone();
        let rollback = request.rollback_turns.filter(|_| turn.is_none());
        if turn.is_none() && rollback.is_none() {
            return Err(HarnessError::Protocol(
                "Cannot fork Codex at a specific turn without a native turn reference.".into(),
            ));
        }
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
            let mut params = json!({
                "threadId":request.source_thread_id,"cwd":request.cwd,
                "model":request.model,"approvalPolicy":policy.codex_approval,
                "sandbox":sandbox_mode(policy.codex_sandbox),
                "config":request.mcp.codex_thread_overrides(),
                "developerInstructions":request.mcp.instructions()
            });
            if let Some(turn) = &turn {
                params["lastTurnId"] = json!(turn);
            }
            let result = client.request("thread/fork", params).await?;
            let forked = result["thread"]["id"]
                .as_str()
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    HarnessError::Protocol("Codex thread/fork returned no native thread ID.".into())
                })?;
            match rollback.filter(|turns| *turns > 0) {
                Some(turns) => revert_forked_turns(&client, &forked, turns).await,
                None => Ok(forked),
            }
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
