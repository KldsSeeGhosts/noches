//! Opt-in session MCP over the existing engine IPC. Browser MCP stays separate.
mod jsonrpc;
#[cfg(test)]
mod tests;
mod tools;
mod zeron;

pub use jsonrpc::serve_stdio;
pub use tools::Tools;
pub use zeron::{Origin, Zeron};

pub async fn run() -> anyhow::Result<()> {
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
