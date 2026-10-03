#![cfg(unix)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

struct HttpStub {
    endpoint: String,
    requests: mpsc::UnboundedReceiver<(String, BTreeMap<String, String>, Value)>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for HttpStub {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl HttpStub {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
        let (tx, requests) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let tx = tx.clone();
                tokio::spawn(async move {
                    let mut socket = BufReader::new(socket);
                    let mut first = String::new();
                    socket.read_line(&mut first).await.unwrap();
                    let mut headers = BTreeMap::new();
                    loop {
                        let mut line = String::new();
                        socket.read_line(&mut line).await.unwrap();
                        if line == "\r\n" || line.is_empty() {
                            break;
                        }
                        let (key, value) = line.split_once(':').unwrap();
                        headers.insert(key.to_ascii_lowercase(), value.trim().to_owned());
                    }
                    let length: usize = headers
                        .get("content-length")
                        .map(|v| v.parse().unwrap())
                        .unwrap_or(0);
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).await.unwrap();
                    let message: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    tx.send((first, headers, message.clone())).unwrap();
                    let method = message["method"].as_str().unwrap_or("");
                    let (status, content_type, body, session) = if message.is_null() {
                        ("204 No Content", "application/json", String::new(), "")
                    } else if method == "initialize" {
                        (
                            "200 OK",
                            "application/json",
                            json!({"jsonrpc":"2.0","id":message["id"],
                            "result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},
                            "serverInfo":{"name":"http-stub","version":"1"}}})
                            .to_string(),
                            "mcp-session-id: http-session\r\n",
                        )
                    } else if method == "notifications/initialized" {
                        ("202 Accepted", "application/json", String::new(), "")
                    } else if method == "tools/list" {
                        let result = json!({"jsonrpc":"2.0","id":message["id"],"result":{"tools":[{"name":"echo","inputSchema":{"type":"object"}}]}});
                        (
                            "200 OK",
                            "text/event-stream",
                            format!(
                                ": keepalive\r\n\r\ndata: {{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\r\ndata: \"params\":{{\"level\":\"info\",\"data\":\"stub\"}}}}\r\n\r\ndata: {result}\r\n\r\n"
                            ),
                            "",
                        )
                    } else if message["params"]["name"] == "rpc_error" {
                        (
                            "200 OK",
                            "application/json",
                            json!({"jsonrpc":"2.0","id":message["id"],
                            "error":{"code":-32042,"message":"stub error","data":{"retry":false}}})
                            .to_string(),
                            "",
                        )
                    } else {
                        (
                            "200 OK",
                            "application/json",
                            json!({"jsonrpc":"2.0","id":message["id"],"result":{
                            "content":[{"type":"text","text":"HTTP_MCP_OK"}],
                            "structuredContent":{"ok":true},"isError":false}})
                            .to_string(),
                            "",
                        )
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{session}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    socket.get_mut().write_all(head.as_bytes()).await.unwrap();
                    // Fragment SSE/JSON across arbitrary byte boundaries.
                    for chunk in body.as_bytes().chunks(11) {
                        if socket.get_mut().write_all(chunk).await.is_err() {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                });
            }
        });
        Self {
            endpoint,
            requests,
            task,
        }
    }
}

fn command(stub: &HttpStub) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_noches-mcp"));
    command
        .env_remove("NOCHES_SESSION_MCP_ENTRIES")
        .env("NOCHES_ACP_MCP_ENDPOINT", &stub.endpoint)
        .env(
            "NOCHES_ACP_MCP_AUTHORIZATION",
            "Bearer http-stub-private-token",
        )
        .kill_on_drop(true);
    command
}

#[tokio::test]
async fn http_bridge_preserves_sse_errors_negotiated_headers_and_teardown() {
    let mut stub = HttpStub::start().await;
    let mut child = command(&stub)
        .arg("acp-mcp-bridge")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .await
        .unwrap();
    let init: Value = serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(5), stdout.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n").await.unwrap();
    let notification: Value =
        serde_json::from_str(&stdout.next_line().await.unwrap().unwrap()).unwrap();
    assert_eq!(notification["method"], "notifications/message");
    let listed: Value = serde_json::from_str(&stdout.next_line().await.unwrap().unwrap()).unwrap();
    assert_eq!(listed["result"]["tools"][0]["name"], "echo");
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"rpc_error\",\"arguments\":{}}}\n").await.unwrap();
    let error: Value = serde_json::from_str(&stdout.next_line().await.unwrap().unwrap()).unwrap();
    assert_eq!(
        error["error"],
        json!({"code":-32042,"message":"stub error","data":{"retry":false}})
    );
    drop(stdin);
    drop(stdout);
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("http-stub-private-token"));
    let mut saw_delete = false;
    while let Ok((method, headers, body)) = stub.requests.try_recv() {
        assert_eq!(headers["authorization"], "Bearer http-stub-private-token");
        if body["method"] != "initialize" {
            assert_eq!(headers["mcp-session-id"], "http-session");
            assert_eq!(headers["mcp-protocol-version"], "2025-03-26");
        }
        saw_delete |= method.starts_with("DELETE ");
    }
    assert!(saw_delete, "EOF must remove the negotiated HTTP session");
}

#[tokio::test]
async fn cli_fallback_uses_authenticated_handshake_and_preserves_tool_content() {
    let stub = HttpStub::start().await;
    let result = command(&stub)
        .args(["acp-mcp-call", "echo", "{}"])
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["content"][0]["text"], "HTTP_MCP_OK");
    assert_eq!(result["structuredContent"]["ok"], true);
    assert_eq!(result["isError"], false);
}

#[tokio::test]
async fn malformed_bridge_input_still_removes_the_http_session() {
    let mut stub = HttpStub::start().await;
    let mut child = command(&stub)
        .arg("acp-mcp-bridge")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), stdout.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let oversized = vec![b'a'; 4 * 1024 * 1024 + 1];
    let _ = stdin.write_all(&oversized).await;
    drop(stdin);
    drop(stdout);
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("MCP input frame exceeds limit"));
    let mut deleted = false;
    while let Ok((method, _, _)) = stub.requests.try_recv() {
        deleted |= method.starts_with("DELETE ");
    }
    assert!(deleted, "input errors must also remove the HTTP session");
}
