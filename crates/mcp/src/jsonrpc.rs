use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::Tools;

pub async fn serve_stdio(tools: Arc<Tools>) -> anyhow::Result<()> {
    let (out, mut replies) = tokio::sync::mpsc::channel::<Value>(64);
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(reply) = replies.recv().await {
            stdout.write_all(reply.to_string().as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        Ok::<_, std::io::Error>(())
    });
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut tasks = tokio::task::JoinSet::new();
    loop {
        // Bound each incoming frame and the number of concurrent waits.
        let mut line = Vec::new();
        let count = (&mut reader)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut line)
            .await?;
        if count == 0 {
            break;
        }
        anyhow::ensure!(count <= 1024 * 1024, "MCP frame exceeds 1 MiB");
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let message = match serde_json::from_slice::<Value>(&line) {
            Ok(message) => message,
            Err(error) => {
                out.send(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error.to_string()}})).await?;
                continue;
            }
        };
        if message.get("id").is_none() {
            continue;
        }
        while tasks.len() >= 32 {
            let _ = tasks.join_next().await;
        }
        while tasks.try_join_next().is_some() {}
        let tools = tools.clone();
        let out = out.clone();
        tasks.spawn(async move {
            let reply = handle_request(&tools, message).await;
            let _ = out.send(reply).await;
        });
    }
    while tasks.join_next().await.is_some() {}
    drop(out);
    writer.await??;
    Ok(())
}

pub(crate) async fn handle_request(tools: &Tools, message: Value) -> Value {
    let id = message["id"].clone();
    let result = match message["method"].as_str() {
        Some("initialize") => {
            let requested = message["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = if ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&requested) {
                requested
            } else {
                "2025-06-18"
            };
            json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"noches-sessions","version":env!("CARGO_PKG_VERSION")},"instructions":"Opt-in Noches session control. Discover listed device/project ids before creating kind chat sessions. Side chats and arbitrary working directories are not supported. Catalogs and transcripts target the selected host. Approval remains interactive. Launch with wait false before collecting independent turns. Browser tools remain in the separate browser-mcp server."})
        }
        Some("ping") => json!({}),
        Some("tools/list") => json!({"tools":tools.list()}),
        Some("tools/call") => {
            let name = message["params"]["name"].as_str().unwrap_or("");
            match tools
                .call(
                    name,
                    message["params"]
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                )
                .await
            {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
                }
                Err(error) => json!({"content":[{"type":"text","text":error}],"isError":true}),
            }
        }
        _ => {
            return json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}});
        }
    };
    json!({"jsonrpc":"2.0","id":id,"result":result})
}
