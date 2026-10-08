// Explicit opt-in integration test. Uses the existing OAuth account and a
// temporary workspace. Never prints child stderr or credential-bearing frames.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, symlink, unlink, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { StringDecoder } from "node:string_decoder";
import { credentialPath } from "./oauth.mjs";

const cwd = await mkdtemp(join(tmpdir(), "zcode-oauth-acp-smoke-"));
const token = `fixture-${randomUUID()}`;
await writeFile(join(cwd, "fixture.txt"), token);
const childEnv = { ...process.env, ZCODE_ACP_MODE: "build" };
const links = [];
if (process.argv.includes("--without-legacy-config")) {
  // No config.json, settings-derived entitlement, or credential copies. The
  // native reader follows a private temporary link to the existing login.
  const zcodeHome = join(cwd, ".zcode");
  await mkdir(join(zcodeHome, "v2"), { recursive: true });
  const originalCredentials = credentialPath();
  for (const name of ["credentials.json", "provider_config.json"]) {
    const source = join(dirname(originalCredentials), name);
    if (!existsSync(source)) continue;
    const target = join(zcodeHome, "v2", name);
    await symlink(source, target);
    links.push(target);
  }
  childEnv.ZCODE_HOME = zcodeHome;
}
const child = spawn(process.execPath, [fileURLToPath(new URL("./launch.mjs", import.meta.url))], {
  cwd,
  env: childEnv,
  stdio: ["pipe", "pipe", "pipe"],
});
const pending = new Map();
let nextId = 1;
let buffer = "";
let assistantText = "";
let toolCalls = 0;
let permissionRequests = 0;
let cancelOnText;
const decoder = new StringDecoder("utf8");
let failure;
child.stderr.resume();
child.stdout.on("data", (chunk) => {
  buffer += decoder.write(chunk);
  let end;
  while ((end = buffer.indexOf("\n")) !== -1) {
    const line = buffer.slice(0, end);
    buffer = buffer.slice(end + 1);
    if (!line.trim()) continue;
    let frame;
    try { frame = JSON.parse(line); } catch { failure = new Error("Non-JSON ACP output"); continue; }
    if (pending.has(frame.id)) {
      const flight = pending.get(frame.id);
      pending.delete(frame.id);
      clearTimeout(flight.timer);
      if (frame.error) flight.reject(new Error(`ACP request failed with code ${frame.error.code}`));
      else flight.resolve(frame.result);
    } else if (frame.method === "session/update") {
      const update = frame.params.update;
      if (update.sessionUpdate === "agent_message_chunk") {
        assistantText += update.content?.text ?? "";
        if (cancelOnText && update.content?.text) {
          child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method: "session/cancel", params: { sessionId: cancelOnText } })}\n`);
          cancelOnText = undefined;
        }
      }
      if (update.sessionUpdate === "tool_call") toolCalls++;
    } else if (frame.method === "session/request_permission") {
      permissionRequests++;
      const option = frame.params.options?.find((item) => item.kind === "allow_once");
      assert.ok(option, "Bridge must offer an allow-once permission");
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id: frame.id, result: {
        outcome: { outcome: "selected", optionId: option.optionId },
      } })}\n`);
    } else if (frame.id !== undefined && frame.method) {
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id: frame.id, error: {
        code: -32601, message: "Smoke client does not provide this capability",
      } })}\n`);
    }
  }
});
child.on("error", (error) => { failure = error; });
child.on("exit", () => {
  for (const flight of pending.values()) {
    clearTimeout(flight.timer);
    flight.reject(new Error("ACP process exited"));
  }
  pending.clear();
});
function request(method, params, timeout = 90000) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`ACP ${method} timed out`));
    }, timeout);
    pending.set(id, { resolve, reject, timer });
    child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  });
}
try {
  const initialized = await request("initialize", {
    protocolVersion: 1,
    clientInfo: { name: "noches-oauth-smoke", version: "1" },
    clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
  });
  assert.equal(initialized.authMethods[0].name, "ZCode OAuth subscription");
  console.log("OAuth authentication metadata: PASS");
  const session = await request("session/new", { cwd, mcpServers: [] });
  const modelOption = session.configOptions.find((option) => option.category === "model");
  assert.ok(modelOption.options.length >= 2);
  assert.ok(modelOption.options.every((option) => option.value.startsWith("account:")));
  console.log("Native OAuth model catalog:", modelOption.options.map((option) => option.name).join(", "));
  const sessionId = session.sessionId;
  await request("session/set_mode", { sessionId, modeId: "build" });
  let result = await request("session/prompt", { sessionId, prompt: [{
    type: "text", text: "Reply with exactly pong. Do not use tools.",
  }] });
  assert.equal(result.stopReason, "end_turn");
  assert.ok(/\bpong\b/.test(assistantText));
  console.log("OAuth model turn: PASS");
  const flash = modelOption.options.find((option) => option.name.includes("Flash"));
  assert.ok(flash);
  await request("session/set_config_option", { sessionId, configId: "model", value: flash.value });
  assistantText = "";
  result = await request("session/prompt", { sessionId, prompt: [{
    type: "text", text: "Use a read-only tool to read fixture.txt in this workspace. Reply with its exact contents. Do not change files or access other directories.",
  }] });
  assert.equal(result.stopReason, "end_turn");
  assert.ok(assistantText.includes(token));
  assert.ok(toolCalls > 0);
  console.log(`Model switching and native tools: PASS (${toolCalls} tool calls, ${permissionRequests} permissions)`);
  assistantText = "";
  await request("session/load", { sessionId, cwd, mcpServers: [] });
  assert.ok(assistantText.includes("pong"));
  console.log("Session replay: PASS");
  assistantText = "";
  cancelOnText = sessionId;
  result = await request("session/prompt", { sessionId, prompt: [{
    type: "text", text: "Write a 5000-word essay about Rust ownership. Do not use tools.",
  }] });
  assert.equal(result.stopReason, "cancelled");
  console.log("Native turn cancellation: PASS");
  if (links.length) console.log("No legacy config.json or settings-derived entitlement: PASS");
  if (failure) throw failure;
} finally {
  child.kill("SIGTERM");
  child.stdin.end();
  for (const flight of pending.values()) clearTimeout(flight.timer);
  if (child.exitCode === null && child.signalCode === null) {
    await new Promise((resolve) => {
      const timeout = setTimeout(() => child.kill("SIGKILL"), 10000);
      child.once("exit", () => { clearTimeout(timeout); resolve(); });
    });
  }
  for (const link of links) await unlink(link);
  // Keep the tiny fixture and native session for inspection, rather than
  // recursively deleting a workspace the native harness has written into.
  console.log("Smoke fixture retained:", cwd);
}
