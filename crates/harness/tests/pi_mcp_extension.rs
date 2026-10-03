#![cfg(unix)]

use std::time::Duration;

/// Execute the generated extension against Pi's public extension API shape,
/// using the real Rust bridge and a real stdio MCP child (no LLM credentials).
#[tokio::test]
async fn pi_extension_lists_registers_calls_and_injects_real_system_instructions() {
    let dir = tempfile::tempdir().unwrap();
    let extension = dir.path().join("noches-mcp.ts");
    std::fs::write(&extension, include_str!("../src/pi/noches-mcp.ts")).unwrap();
    let typebox = dir.path().join("node_modules/typebox");
    std::fs::create_dir_all(&typebox).unwrap();
    std::fs::write(
        typebox.join("package.json"),
        r#"{"type":"module","exports":"./index.js"}"#,
    )
    .unwrap();
    std::fs::write(
        typebox.join("index.js"),
        "export const Type = { Unsafe: value => value, Object: () => ({type:'object'}) };",
    )
    .unwrap();
    let stub = dir.path().join("stub.py");
    std::fs::write(&stub, include_str!("fixtures/mcp-peer.py")).unwrap();
    let record = dir.path().join("methods");
    let runner = dir.path().join("runner.mjs");
    std::fs::write(&runner, r#"
import assert from "node:assert/strict";
import extension from "./noches-mcp.ts";
const tools = new Map();
const handlers = new Map();
const pi = { registerTool: tool => tools.set(tool.name, tool), on: (name, fn) => handlers.set(name, fn) };
await extension(pi);
try {
  assert.equal(tools.size, 1);
  const tool = tools.get("mcp__stub__echo");
  assert.equal(tool.parameters.required[0], "value");
  const result = await tool.execute("test", {value:"pi-extension"}, AbortSignal.timeout(10_000));
  assert.equal(result.content[0].text, "MCP_RESULT:pi-extension");
  assert.equal(result.details.structuredContent.ok, true);
  const instructions = handlers.get("before_agent_start")({systemPrompt:"original"});
  assert.equal(instructions.systemPrompt, "original\n\nSESSION_INSTRUCTIONS");
  console.log("PI_MCP_OK");
} finally {
  handlers.get("session_shutdown")();
}
"#).unwrap();
    let mut command = tokio::process::Command::new("node");
    command
        .args(["--experimental-strip-types"])
        .arg(&runner)
        .env(
            "NOCHES_ACP_MCP_EXECUTABLE",
            env!("CARGO_BIN_EXE_noches-mcp"),
        )
        .env("NOCHES_SESSION_MCP_INSTRUCTIONS", "SESSION_INSTRUCTIONS")
        .env(
            "NOCHES_SESSION_MCP_ENTRIES",
            serde_json::json!([{"name":"stub","config":{
            "command":"python3","args":[stub,"--mcp","--paginated","--record",record],"env":{}}}])
            .to_string(),
        )
        .kill_on_drop(true);
    let result = tokio::time::timeout(Duration::from_secs(20), command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("PI_MCP_OK"));
    let methods = std::fs::read_to_string(record).unwrap();
    assert!(methods.contains("tools/list") && methods.contains("tools/call"));
    assert_eq!(methods.lines().filter(|m| *m == "tools/list").count(), 2);
}
