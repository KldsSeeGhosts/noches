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
import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import extension from "./noches-policy.ts";

// A workspace, an agent dir and a home, all under one scratch root.
const root = process.env.SCRATCH;
const cwd = join(root, "work");
const outside = join(root, "outside");
const home = process.env.HOME;
mkdirSync(join(cwd, "src"), { recursive: true });
mkdirSync(join(cwd, ".pi", "extensions"), { recursive: true });
mkdirSync(outside, { recursive: true });
mkdirSync(join(home, ".pi", "agent", "extensions"), { recursive: true });
symlinkSync(outside, join(cwd, "link"));
writeFileSync(join(outside, "target.txt"), "x");

const handlers = new Map();
extension({ on: (name, fn) => handlers.set(name, fn) });
const toolCall = handlers.get("tool_call");

async function call(tool, input, answer) {
  const asked = [];
  const ctx = { cwd, ui: { confirm: async (title, message) => { asked.push({ title, message }); return answer; } } };
  const result = await toolCall({ toolName: tool, input }, ctx);
  return { asked, result };
}

// A missing or unknown mode fails closed to approval-required.
const mode = process.env.NOCHES_PI_RUNTIME_MODE === "bogus" || process.env.NOCHES_PI_RUNTIME_MODE === undefined
  ? "approval-required" : process.env.NOCHES_PI_RUNTIME_MODE;
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
  // Edits inside the working directory are the one thing Auto-accept waves through.
  const auto = mode === "auto-accept-edits";
  for (const tool of ["edit", "write"]) {
    for (const path of ["a", "src/main.rs", join(cwd, "src", "new", "deep.rs"), "./src/../a", "@src/main.rs"]) {
      assert.equal((await call(tool, { path }, false)).asked.length, auto ? 0 : 1, `${tool} ${path} in ${mode}`);
    }
    // ...and nothing that leaves it, reaches Pi's own code, or cannot be placed.
    for (const path of [
      "../outside/x.txt",
      join(outside, "x.txt"),
      "link/target.txt",
      "link/new.txt",
      "/etc/hosts",
      "~/.ssh/authorized_keys",
      "~/.pi/agent/extensions/x.ts",
      join(home, ".pi", "agent", "extensions", "x.ts"),
      ".pi/extensions/x.ts",
      "src/../.pi/extensions/x.ts",
      "file://" + join(outside, "x.txt"),
      "",
    ]) {
      assert.equal((await call(tool, { path }, false)).asked.length, 1, `${tool} ${JSON.stringify(path)} in ${mode}`);
    }
    assert.equal((await call(tool, {}, false)).asked.length, 1, "no path");
    assert.equal((await call(tool, { path: 3 }, false)).asked.length, 1, "bad path");
  }
  // A workspace that is the home directory still guards Pi's own dirs.
  const homeCall = async (path) => {
    const asked = [];
    await toolCall({ toolName: "write", input: { path } }, { cwd: home, ui: { confirm: async () => { asked.push(1); return false; } } });
    return asked.length;
  };
  assert.equal(await homeCall("notes.txt"), auto ? 0 : 1);
  assert.equal(await homeCall(".pi/agent/extensions/x.ts"), 1);
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

    for mode in ["full-access", "approval-required", "auto-accept-edits", "bogus", ""] {
        let scratch = tempfile::tempdir().unwrap();
        let home = scratch.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let mut command = tokio::process::Command::new("node");
        command
            .args(["--experimental-strip-types"])
            .arg(&runner)
            .env("SCRATCH", scratch.path())
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env_remove("PI_CODING_AGENT_DIR")
            .env_remove("NOCHES_PI_RUNTIME_MODE")
            .env(
                "NOCHES_SESSION_MCP_ALLOWED_TOOLS",
                r#"["mcp__t3-code__*","mcp__t3-code__task_status"]"#,
            )
            .kill_on_drop(true);
        // "" stands for the variable missing altogether.
        if !mode.is_empty() {
            command.env("NOCHES_PI_RUNTIME_MODE", mode);
        }
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
