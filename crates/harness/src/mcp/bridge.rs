//! MCP stdio ↔ streamable-HTTP forwarding and ACP terminal fallback. No
//! orchestration tools are defined here; results/errors are opaque JSON-RPC.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

use super::{
    ACP_AUTHORIZATION_ENV, ACP_ENDPOINT_ENV, MCP_ENTRIES_ENV, McpServerEntry, McpTransport,
};
use crate::{
    HarnessError,
    jsonrpc::{Incoming, RpcClient},
    process::{Child, Command},
};

const MAX_FRAME: usize = 4 * 1024 * 1024;
const PROTOCOL: &str = "2025-06-18";

#[derive(Default)]
struct HttpState {
    session_id: Option<String>,
    protocol: Option<String>,
}

#[derive(Clone)]
struct HttpPeer {
    client: reqwest::Client,
    url: String,
    headers: std::collections::BTreeMap<String, String>,
    state: Arc<Mutex<HttpState>>,
}

impl HttpPeer {
    fn new(
        url: String,
        headers: std::collections::BTreeMap<String, String>,
    ) -> Result<Self, HarnessError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_millis(super::TOOL_TIMEOUT_MS))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| protocol_error("could not create MCP HTTP client"))?,
            url,
            headers,
            state: Arc::default(),
        })
    }

    async fn request(
        &self,
        message: Value,
        output: &mpsc::Sender<Value>,
    ) -> Result<(), HarnessError> {
        let mut request = self
            .client
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .json(&message);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        {
            let state = self.state.lock().await;
            if let Some(id) = &state.session_id {
                request = request.header("mcp-session-id", id);
            }
            if let Some(version) = &state.protocol {
                request = request.header("mcp-protocol-version", version);
            }
        }
        let response = request
            .send()
            .await
            .map_err(|_| protocol_error("MCP HTTP transport failed"))?;
        if !response.status().is_success() {
            return Err(protocol_error(&format!(
                "MCP endpoint responded with HTTP {}",
                response.status().as_u16()
            )));
        }
        if let Some(id) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
        {
            self.state.lock().await.session_id = Some(id.into());
        }
        if matches!(response.status().as_u16(), 202 | 204) {
            return Ok(());
        }
        let sse = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|ct| ct.contains("text/event-stream"));
        let mut bytes = response.bytes_stream();
        let mut buffered = Vec::new();
        while let Some(chunk) = bytes.next().await {
            buffered.extend_from_slice(
                &chunk.map_err(|_| protocol_error("MCP response stream failed"))?,
            );
            if sse {
                while let Some((end, width)) = separator(&buffered) {
                    if end > MAX_FRAME {
                        return Err(protocol_error("MCP SSE frame exceeds limit"));
                    }
                    let event = String::from_utf8(buffered.drain(..end + width).collect())
                        .map_err(|_| protocol_error("MCP SSE is not UTF-8"))?;
                    let data = event
                        .lines()
                        .filter_map(|line| line.strip_prefix("data:"))
                        .map(str::trim_start)
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !data.is_empty() {
                        self.emit(
                            serde_json::from_str(&data)
                                .map_err(|_| protocol_error("invalid MCP SSE JSON"))?,
                            output,
                        )
                        .await?;
                    }
                }
            }
            if buffered.len() > MAX_FRAME {
                return Err(protocol_error("MCP response frame exceeds limit"));
            }
        }
        if !sse && !buffered.is_empty() {
            self.emit(
                serde_json::from_slice(&buffered)
                    .map_err(|_| protocol_error("invalid MCP response JSON"))?,
                output,
            )
            .await?;
        }
        Ok(())
    }

    async fn emit(&self, message: Value, output: &mpsc::Sender<Value>) -> Result<(), HarnessError> {
        if let Some(version) = message["result"]["protocolVersion"].as_str() {
            self.state.lock().await.protocol = Some(version.into());
        }
        output
            .send(message)
            .await
            .map_err(|_| protocol_error("MCP stdout closed"))
    }

    async fn close(&self) {
        let state = self.state.lock().await;
        let Some(id) = state.session_id.clone() else {
            return;
        };
        let mut request = self
            .client
            .delete(&self.url)
            .header("mcp-session-id", id)
            .timeout(Duration::from_secs(2));
        if let Some(version) = &state.protocol {
            request = request.header("mcp-protocol-version", version);
        }
        drop(state);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        let _ = request.send().await;
    }
}

fn separator(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes
        .windows(2)
        .position(|b| b == b"\n\n")
        .map(|n| (n, 2))
        .into_iter()
        .chain(
            bytes
                .windows(4)
                .position(|b| b == b"\r\n\r\n")
                .map(|n| (n, 4)),
        )
        .min_by_key(|(n, _)| *n)
}

fn protocol_error(message: &str) -> HarnessError {
    HarnessError::Protocol(message.into())
}

fn entries_from_environment() -> Result<Vec<McpServerEntry>, HarnessError> {
    if let Ok(raw) = std::env::var(MCP_ENTRIES_ENV) {
        let entries: Vec<Value> = serde_json::from_str(&raw)
            .map_err(|_| protocol_error("invalid host-local MCP bindings"))?;
        return entries.into_iter().map(|entry| {
            let name = entry["name"].as_str().ok_or_else(|| protocol_error("MCP binding has no name"))?;
            let config = &entry["config"];
            let map = |field: &str| -> Result<std::collections::BTreeMap<String, String>, HarnessError> {
                config.get(field).and_then(Value::as_object).into_iter().flatten()
                    .map(|(k,v)| Ok((k.clone(), v.as_str().ok_or_else(|| protocol_error("invalid MCP environment/header"))?.into())))
                    .collect()
            };
            let mut server = if let Some(url) = config["url"].as_str() {
                McpServerEntry::http(name, url, map("headers")?)
            } else {
                let command = config["command"].as_str().ok_or_else(|| protocol_error("MCP binding has no command"))?;
                let args = config["args"].as_array().into_iter().flatten()
                    .map(|v| v.as_str().map(str::to_owned).ok_or_else(|| protocol_error("invalid MCP argument"))).collect::<Result<_,_>>()?;
                McpServerEntry::stdio(name, command, args)
            };
            if let McpTransport::Stdio { env, .. } = &mut server.transport { *env = map("env")?; }
            Ok(server)
        }).collect();
    }
    let endpoint = std::env::var(ACP_ENDPOINT_ENV).map_err(|_| {
        protocol_error("NOCHES_SESSION_MCP_ENTRIES or NOCHES_ACP_MCP_ENDPOINT is required")
    })?;
    let authorization = std::env::var(ACP_AUTHORIZATION_ENV)
        .map_err(|_| protocol_error("NOCHES_ACP_MCP_AUTHORIZATION is required"))?;
    Ok(vec![McpServerEntry::http(
        "noches",
        endpoint,
        [("Authorization".into(), authorization)].into(),
    )])
}

/// Fast-path CLI used by both the shipped app and the headless helper.
pub async fn cli(args: &[String]) -> Result<(), HarnessError> {
    let entries = entries_from_environment()?;
    // Validate and register opaque credentials before any subprocess can echo
    // them. The context also makes CLI diagnostics use exact-value redaction.
    let _context = super::SessionMcpContext::new(entries.clone(), String::new(), Vec::new())?;
    match args.first().map(String::as_str) {
        Some("acp-mcp-bridge") if args.len() == 1 => {
            let entry = entries
                .into_iter()
                .next()
                .ok_or_else(|| protocol_error("no MCP server registered"))?;
            match entry.transport {
                McpTransport::StreamableHttp { url, headers } => {
                    run_http_bridge(HttpPeer::new(url, headers)?).await
                }
                McpTransport::Stdio { command, args, env } => {
                    let mut command_builder = Command::new(&command);
                    command_builder.args(args).envs(env);
                    crate::compose_child_environment(&mut command_builder, &command);
                    command_builder
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::null())
                        .kill_on_drop(true);
                    let mut child = command_builder.spawn()?;
                    let mut input = child.stdin.take().expect("piped stdin");
                    let mut output = child.stdout.take().expect("piped stdout");
                    let mut stdin = tokio::io::stdin();
                    let mut stdout = tokio::io::stdout();
                    tokio::select! {
                        _ = tokio::io::copy(&mut stdin, &mut input) => {},
                        _ = tokio::io::copy(&mut output, &mut stdout) => {},
                    }
                    crate::shutdown_child(&mut child, Duration::from_secs(1)).await;
                    Ok(())
                }
            }
        }
        Some("acp-mcp-call") if args.len() == 3 => {
            let arguments: Value = serde_json::from_str(&args[2])
                .map_err(|_| protocol_error("tool arguments must be JSON"))?;
            if !arguments.is_object() {
                return Err(protocol_error("tool arguments must be a JSON object"));
            }
            let (entry, tool) = select_server(entries, &args[1]).await?;
            let mut peer = ToolPeer::connect(entry).await?;
            let result = async {
                peer.request("initialize", json!({"protocolVersion":PROTOCOL,"capabilities":{},"clientInfo":{"name":"noches-acp-cli","version":env!("CARGO_PKG_VERSION")}})).await?;
                peer.notify("notifications/initialized").await?;
                peer.request("tools/call", json!({"name":tool,"arguments":arguments})).await
            }.await;
            peer.close().await;
            println!("{}", result?);
            Ok(())
        }
        _ => Err(protocol_error(
            "usage: acp-mcp-bridge | acp-mcp-call <tool> '<json>'",
        )),
    }
}

async fn select_server(
    entries: Vec<McpServerEntry>,
    tool: &str,
) -> Result<(McpServerEntry, String), HarnessError> {
    if tool.starts_with("mcp__") {
        let mut matches = entries.into_iter().filter_map(|entry| {
            let prefix = format!("mcp__{}__", entry.name);
            tool.strip_prefix(&prefix)
                .map(|tool| (entry.clone(), tool.to_owned()))
        });
        let candidate = matches
            .next()
            .ok_or_else(|| protocol_error("MCP server is not registered"))?;
        if matches.next().is_some() {
            return Err(protocol_error("ambiguous qualified MCP tool name"));
        }
        return Ok(candidate);
    }
    if entries.len() == 1 {
        return Ok((entries.into_iter().next().expect("one entry"), tool.into()));
    }
    // Keep T3's raw `acp-mcp-call <tool> <json>` semantics when this general
    // context also contains a browser. Resolve the unique tool owner first.
    let mut matching = Vec::new();
    for entry in entries {
        let mut peer = ToolPeer::connect(entry.clone()).await?;
        let listed = tokio::time::timeout(Duration::from_secs(10), async {
            peer.request(
                "initialize",
                json!({"protocolVersion":PROTOCOL,"capabilities":{},
                "clientInfo":{"name":"noches-acp-discovery","version":env!("CARGO_PKG_VERSION")}}),
            )
            .await?;
            peer.notify("notifications/initialized").await?;
            let mut cursor: Option<String> = None;
            let mut seen = std::collections::HashSet::new();
            loop {
                let page = peer
                    .request(
                        "tools/list",
                        match &cursor {
                            Some(cursor) => json!({"cursor":cursor}),
                            None => json!({}),
                        },
                    )
                    .await?;
                if page["tools"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|candidate| candidate["name"] == tool)
                {
                    return Ok::<_, HarnessError>(true);
                }
                cursor = page["nextCursor"].as_str().map(str::to_owned);
                let Some(cursor) = &cursor else {
                    return Ok(false);
                };
                if !seen.insert(cursor.clone()) {
                    return Err(protocol_error("MCP tools pagination loop"));
                }
            }
        })
        .await;
        peer.close().await;
        if listed.map_err(|_| {
            protocol_error("MCP discovery timed out; use a server-qualified tool name")
        })?? {
            matching.push(entry);
        }
    }
    if matching.len() != 1 {
        return Err(protocol_error(
            "MCP tool absent or ambiguous; use mcp__<server>__<tool>",
        ));
    }
    Ok((matching.remove(0), tool.into()))
}

async fn run_http_bridge(peer: HttpPeer) -> Result<(), HarnessError> {
    let (tx, mut rx) = mpsc::channel::<Value>(128);
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(message) = rx.recv().await {
            stdout.write_all(format!("{message}\n").as_bytes()).await?;
            stdout.flush().await?;
        }
        Ok::<_, std::io::Error>(())
    });
    let mut tasks = tokio::task::JoinSet::new();
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut frame = Vec::new();
    let result = async {
    loop {
        frame.clear();
        // Bound input before allocation, not after .lines() has read it all.
        use tokio::io::AsyncReadExt;
        let n = (&mut stdin)
            .take((MAX_FRAME + 1) as u64)
            .read_until(b'\n', &mut frame)
            .await?;
        if n == 0 {
            break;
        }
        if frame.len() > MAX_FRAME {
            return Err(protocol_error("MCP input frame exceeds limit"));
        }
        let message: Value = match serde_json::from_slice(&frame) {
            Ok(value) => value,
            Err(_) => {
                let _ = tx.send(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})).await;
                continue;
            }
        };
        let id = message.get("id").cloned();
        // Initialize must finish before accepting the next request: headers
        // negotiated by it belong to all following HTTP traffic.
        if message["method"] == "initialize" {
            if let Err(error) = peer.request(message, &tx).await {
                let _ = tx.send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":error.to_string()}})).await;
            }
        } else {
            if tasks.len() >= 128 {
                let _ = tasks.join_next().await;
            }
            let peer = peer.clone();
            let tx = tx.clone();
            tasks.spawn(async move {
                if let Err(error) = peer.request(message, &tx).await && let Some(id) = id {
                    let _ = tx.send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":error.to_string()}})).await;
                }
            });
        }
    }
    Ok::<_, HarnessError>(())
    }.await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    peer.close().await;
    drop(tx);
    let written = writer
        .await
        .map_err(|_| protocol_error("MCP writer failed"));
    result?;
    written??;
    Ok(())
}

enum ToolPeer {
    Http(HttpPeer),
    Stdio {
        child: Child,
        rpc: RpcClient,
        incoming: mpsc::Receiver<Incoming>,
    },
}

impl ToolPeer {
    async fn connect(entry: McpServerEntry) -> Result<Self, HarnessError> {
        match entry.transport {
            McpTransport::StreamableHttp { url, headers } => {
                Ok(Self::Http(HttpPeer::new(url, headers)?))
            }
            McpTransport::Stdio { command, args, env } => {
                let mut cmd = Command::new(&command);
                cmd.args(args).envs(env);
                crate::compose_child_environment(&mut cmd, &command);
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true);
                let mut child = cmd.spawn()?;
                let (rpc, incoming) = RpcClient::new(
                    child.stdin.take().expect("piped"),
                    child.stdout.take().expect("piped"),
                );
                Ok(Self::Stdio {
                    child,
                    rpc,
                    incoming,
                })
            }
        }
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, HarnessError> {
        match self {
            Self::Http(peer) => {
                let (tx, mut rx) = mpsc::channel(128);
                let request = peer.request(
                    json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
                    &tx,
                );
                tokio::pin!(request);
                loop {
                    tokio::select! {
                        result = &mut request => {
                            result?;
                            while let Ok(message) = rx.try_recv() {
                                if message.get("id") == Some(&json!(1)) { return rpc_result(message); }
                            }
                            return Err(protocol_error("MCP response missing"));
                        },
                        Some(message) = rx.recv() => {
                            if message.get("id") == Some(&json!(1)) { return rpc_result(message); }
                        }
                    }
                }
            }
            Self::Stdio { rpc, incoming, .. } => {
                let request = rpc.request(method, params);
                tokio::pin!(request);
                loop {
                    tokio::select! {
                        result = &mut request => return result,
                        Some(message) = incoming.recv() => if let Incoming::Request { id, .. } = message {
                            rpc.respond_error(&id, -32601, "terminal fallback cannot answer server requests");
                        },
                    }
                }
            }
        }
    }

    async fn notify(&self, method: &str) -> Result<(), HarnessError> {
        match self {
            Self::Http(peer) => {
                let (tx, _rx) = mpsc::channel(128);
                peer.request(json!({"jsonrpc":"2.0","method":method}), &tx)
                    .await
            }
            Self::Stdio { rpc, .. } => {
                rpc.notify(method, None);
                Ok(())
            }
        }
    }

    async fn close(&mut self) {
        match self {
            Self::Http(peer) => peer.close().await,
            Self::Stdio { child, .. } => crate::shutdown_child(child, Duration::from_secs(1)).await,
        }
    }
}

fn rpc_result(message: Value) -> Result<Value, HarnessError> {
    if let Some(error) = message.get("error") {
        return Err(protocol_error(&crate::redact::redact_registered(
            &error.to_string(),
        )));
    }
    message
        .get("result")
        .cloned()
        .ok_or_else(|| protocol_error("MCP response missing result"))
}
