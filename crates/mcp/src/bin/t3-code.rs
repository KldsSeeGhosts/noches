//! Logging-free stdio facade. Credentials arrive only through the private
//! session environment; neither a token nor a caller id is accepted as argv.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    zeron_mcp::run_t3_code().await
}
