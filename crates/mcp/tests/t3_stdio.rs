use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn stdio_facade_preserves_http_negotiation_and_refusal_envelope() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
    let refusal = json!({"content":[{"type":"text","text":"{\"_tag\":\"OrchestratorMcpFailure\",\"code\":\"orchestration_error\",\"message\":\"The operation could not be completed.\"}"}],
        "structuredContent":{"_tag":"OrchestratorMcpFailure","code":"orchestration_error","message":"The operation could not be completed."},"isError":false});
    let expected = refusal.clone();
    let server = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            let mut request_line = String::new();
            socket.read_line(&mut request_line).await.unwrap();
            let mut length = 0;
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                socket.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
                headers.push(line.to_lowercase());
            }
            assert!(
                headers
                    .iter()
                    .any(|h| h == "authorization: bearer fixture-private-token\r\n")
            );
            if request_line.starts_with("DELETE") {
                socket
                    .get_mut()
                    .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
                break;
            }
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let message: Value = serde_json::from_slice(&body).unwrap();
            let response = match message["method"].as_str() {
                Some("initialize") => json!({"jsonrpc":"2.0","id":message["id"],
                    "result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"t3-code","version":"fixture"}}}),
                Some("notifications/initialized") => {
                    socket.get_mut().write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                    continue;
                }
                Some("tools/list") => {
                    assert!(
                        headers
                            .iter()
                            .any(|h| h == "mcp-session-id: fixture-session\r\n")
                    );
                    assert!(
                        headers
                            .iter()
                            .any(|h| h == "mcp-protocol-version: 2025-06-18\r\n")
                    );
                    json!({"jsonrpc":"2.0","id":message["id"],"result":{"tools":[]}})
                }
                Some("tools/call") => json!({"jsonrpc":"2.0","id":message["id"],"result":refusal}),
                _ => panic!("unexpected MCP method"),
            };
            let body = response.to_string();
            socket.get_mut().write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: fixture-session\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_t3-code"))
        .env(zeron_harness::mcp::MCP_ENTRIES_ENV, json!([{
            "name":"t3-code","config":{"type":"http","url":endpoint,"headers":{"Authorization":"Bearer fixture-private-token"}}
        }]).to_string())
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    for (id, method, params) in [
        (
            1,
            "initialize",
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}),
        ),
        (2, "tools/list", json!({})),
        (
            3,
            "tools/call",
            json!({"name":"delegate_task","arguments":{"task":"fixture"}}),
        ),
    ] {
        input
            .write_all(
                format!(
                    "{}\n",
                    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), output.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["id"], id);
        if id == 1 {
            assert_eq!(value["result"]["serverInfo"]["name"], "t3-code");
            input
                .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
                .await
                .unwrap();
        } else if id == 3 {
            assert_eq!(value["result"], expected);
        }
    }
    drop(input);
    let status = tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
    server.await.unwrap();
}
