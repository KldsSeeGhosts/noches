//! Headless MCP transport helper; the desktop binary exposes the same commands.
#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Err(error) = zeron_harness::mcp::bridge::cli(&args).await {
        eprintln!(
            "{}",
            zeron_harness::redact::redact_output(&error.to_string())
        );
        std::process::exit(1);
    }
}
