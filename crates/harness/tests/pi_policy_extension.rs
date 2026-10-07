#![cfg(unix)]

use std::time::Duration;

/// Execute the runtime-policy extension against Pi's public extension API
/// shape under node (no Pi, no LLM credentials): which tool calls raise a
/// confirmation, what the host receives, and how a refusal blocks the tool.
#[tokio::test]
async fn policy_extension_gates_tools_by_runtime_mode_and_reports_structured_requests() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("noches-policy.ts"),
        include_str!("../src/pi/noches-policy.ts"),
    )
    .unwrap();
    let runner = dir.path().join("runner.mjs");
    std::fs::write(
        &runner,
        r#"
import assert from "node:assert/strict";
import extension from "./noches-policy.ts";

const handlers = new Map();
extension({ on: (name, fn) => handlers.set(name, fn) });
const toolCall = handlers.get("tool_call");

async function call(tool, input, answer) {
  const asked = [];
  const ctx = { ui: { confirm: async (title, message) => { asked.push({ title, message }); return answer; } } };
  const result = await toolCall({ toolName: tool, input }, ctx);
  return { asked, result };
}

const mode = process.env.NOCHES_PI_RUNTIME_MODE;
if (mode === "full-access") {
  const { asked, result } = await call("bash", { command: "rm -rf /" }, false);
  assert.equal(asked.length, 0);
  assert.equal(result, undefined);
} else {
  // Read-only tools never ask.
  for (const tool of ["read", "grep", "find", "ls"]) {
    assert.equal((await call(tool, { path: "." }, false)).asked.length, 0, tool);
  }
  // A shell call asks with a structured, parseable request...
  const long = "x".repeat(5000);
  const bash = await call("bash", { command: "echo hi", note: long }, true);
  assert.equal(bash.asked.length, 1);
  assert.equal(bash.asked[0].title, "Allow bash?");
  const payload = JSON.parse(bash.asked[0].message);
  assert.deepEqual([payload.noches, payload.tool, payload.input.command], ["permission", "bash", "echo hi"]);
  // ...with long fields truncated per field so the JSON stays valid.
  assert.ok(payload.input.note.length < 2100 && payload.input.note.endsWith("…"));
  assert.equal(bash.result, undefined, "an allowed tool is not blocked");
  // A refusal blocks with a reason the model can read.
  const denied = await call("bash", { command: "ls" }, false);
  assert.deepEqual(denied.result, { block: true, reason: "bash was declined in Noches." });
  // Edits are the one thing Auto-accept waves through.
  const edit = await call("edit", { path: "a" }, false);
  assert.equal(edit.asked.length, mode === "auto-accept-edits" ? 0 : 1, "edit in " + mode);
  assert.equal((await call("write", { path: "a" }, false)).asked.length, mode === "auto-accept-edits" ? 0 : 1);
  // Host-preapproved MCP tools skip the prompt: exact names and trailing-* wildcards only.
  assert.equal((await call("mcp__t3-code__task_status", {}, false)).asked.length, 0);
  assert.equal((await call("mcp__t3-code__anything", {}, false)).asked.length, 0);
  assert.equal((await call("mcp__other__task_status", {}, false)).asked.length, 1);
  // A throwing dialog denies rather than crashing the hook.
  const broken = await toolCall({ toolName: "bash", input: {} }, { ui: { confirm: async () => { throw new Error("gone"); } } });
  assert.equal(broken.block, true);
}

// The OpenRouter output cap workaround leaves other providers alone.
const cap = handlers.get("before_provider_request");
const payload = { max_tokens: 100000, other: 1 };
assert.deepEqual(cap({ payload }, { model: { provider: "openrouter" } }), { max_tokens: 32768, other: 1 });
assert.equal(cap({ payload }, { model: { provider: "cpa" } }), undefined);
assert.equal(cap({ payload: { max_tokens: 10 } }, { model: { provider: "openrouter" } }), undefined);
console.log("POLICY_OK");
"#,
    )
    .unwrap();

    for mode in ["full-access", "approval-required", "auto-accept-edits"] {
        let mut command = tokio::process::Command::new("node");
        command
            .args(["--experimental-strip-types"])
            .arg(&runner)
            .env("NOCHES_PI_RUNTIME_MODE", mode)
            .env(
                "NOCHES_SESSION_MCP_ALLOWED_TOOLS",
                r#"["mcp__t3-code__*","mcp__t3-code__task_status"]"#,
            )
            .kill_on_drop(true);
        let result = tokio::time::timeout(Duration::from_secs(20), command.output())
            .await
            .unwrap()
            .unwrap();
        assert!(
            result.status.success() && String::from_utf8_lossy(&result.stdout).contains("POLICY_OK"),
            "{mode}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
