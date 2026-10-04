//! Opt-in session MCP over the existing engine IPC. Browser MCP stays separate.
mod jsonrpc;
#[cfg(test)]
mod tests;
mod tools;
mod zeron;

pub use jsonrpc::serve_stdio;
pub use tools::Tools;
pub use zeron::{Origin, Zeron};

/// The HTTP-backed `t3-code` stdio facade, also used by the shipped CLI's MCP
/// fast path. It preserves opaque JSON-RPC errors/content and negotiation.
pub async fn run_t3_code() -> anyhow::Result<()> {
    zeron_harness::mcp::bridge::cli(&["acp-mcp-bridge".into()])
        .await
        .map_err(|error| anyhow::anyhow!(zeron_harness::redact::redact_output(&error.to_string())))
}

pub async fn run() -> anyhow::Result<()> {
    // Private engine-issued t3-code bindings use the same negotiated HTTP
    // facade as ACP, not the older unscoped session IPC tool server.
    if std::env::var_os(zeron_harness::mcp::MCP_ENTRIES_ENV).is_some()
        || std::env::var_os(zeron_harness::mcp::ACP_ENDPOINT_ENV).is_some()
    {
        return run_t3_code().await;
    }
    let port = std::env::var("ZERON_IPC_PORT")
        .ok()
        .map(|port| port.parse::<u16>())
        .transpose()?
        .unwrap_or(27654);
    let client = zeron_rpc::connect_ws(&format!("ws://127.0.0.1:{port}")).await?;
    let tools = Tools::new(std::sync::Arc::new(Zeron::with_client(
        client,
        Origin {
            chat_id: std::env::var("ZERON_CHAT_ID")
                .ok()
                .filter(|id| !id.trim().is_empty()),
            device_id: std::env::var("ZERON_DEVICE_ID").ok(),
        },
    )));
    serve_stdio(std::sync::Arc::new(tools)).await
}
