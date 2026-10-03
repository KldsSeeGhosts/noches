// Run against the isolated tree produced by proto/tests/t3_oracle/extract.mjs.
// Never import or install dependencies in the read-only reference checkout.
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const scratch = resolve(process.argv[2]);
const load = (path) => import(pathToFileURL(`${scratch}/${path}`).href);
if (JSON.parse(readFileSync(`${scratch}/node_modules/effect/package.json`)).version !== "4.0.0-rc.115") {
  throw new Error("Pinned Effect version required");
}
const Schema = await load("node_modules/effect/dist/Schema.js");
const AiError = await load("node_modules/effect/dist/unstable/ai/AiError.js");
const Tool = await load("node_modules/effect/dist/unstable/ai/Tool.js");
const tools = {};
for (const group of ["orchestrator", "thread", "worktree", "pullRequests", "project", "environment", "attachment"]) {
  const module = await load(`apps/server/src/mcp/toolkits/${group}/tools.ts`);
  for (const value of Object.values(module)) {
    if (value?.tools) Object.assign(tools, value.tools);
  }
}
const inputs = [
  ["delegate_task", {}],
  ["delegate_task", null],
  ["delegate_task", { task: "   " }],
  ["delegate_task", { task: 3 }],
  ["delegate_task", { task: "x", mode: "invalid" }],
  ["delegate_task", { task: "x", runtimeMode: "invalid" }],
  ["delegate_task", { task: "x", target: null }],
  ["delegate_task", { task: "x", title: null }],
  ["delegate_task", { task: "x", title: "  " }],
  ["delegate_task", { task: "x", target: { driverKind: "?" } }],
  ["delegate_task", { task: "x", target: { options: { effort: 1 } } }],
  ["delegate_task", { task: "x", target: { options: [{ id: "effort", value: 1 }] } }],
  ["task_status", { taskId: " " }],
  ["task_cancel", { taskId: false }],
  ["create_threads", { threads: [] }],
  ["t3_thread_list", { limit: 0 }],
  ["t3_thread_list", { limit: 1.5 }],
  ["t3_thread_list", { limit: 101 }],
  ["t3_thread_list", { statuses: ["invalid"] }],
  ["t3_thread_read", { threadId: "x", afterPosition: -2 }],
  ["t3_thread_update", { action: "rename" }],
  ["t3_thread_update", { action: "regenerate_title", title: "x" }],
  ["t3_thread_update", { action: "link_pull_request" }],
  ["t3_thread_update", { action: "unlink_pull_request", title: "x" }],
  ["t3_thread_update", { action: "link_pull_request", pullRequest: {repository: "o/r", number: 1, url: "not-a-url"} }],
  ["t3_queue_reorder", { queuedRunId: "x", beforeRunId: 0 }],
  ["t3_thread_search", { query: "a" }],
  ["schedule_task", { prompt: "x", schedule: { type: "interval", everyMs: 3 } }],
  ["schedule_task", { prompt: "x", schedule: "garbage" }],
  ["link_pull_request", { number: 0 }],
  ["link_pull_request", { number: 1.5 }],
  ["t3_worktree_handoff", { branch: " " }],
  ["t3_project_update", { projectId: "x", title: null }],
  ["t3_attachment_prepare_upload", { upload: { name: "x", mimeType: "image/png", sizeBytes: -1 } }],
];
const cases = inputs.map(([name, arguments_]) => {
  const tool = tools[name];
  let description;
  try {
    Schema.decodeUnknownSync(tool.parametersSchema)(arguments_);
  } catch (error) {
    description = error.message;
  }
  if (description === undefined) throw new Error(`Expected ${name} to reject ${JSON.stringify(arguments_)}`);
  const error = AiError.make({
    module: "Toolkit",
    method: `${name}.handle`,
    reason: new AiError.ToolParameterValidationError({ toolName: name, description }),
  });
  const encoded = Schema.encodeSync(Tool.failureResultSchema(tool))(error);
  const expected = tool.failureMode === "error"
    ? { error: { code: -32602, message: error.reason.message } }
    : { result: { content: [{ type: "text", text: JSON.stringify(encoded) }], isError: false, structuredContent: encoded } };
  return { name, arguments: arguments_, expected };
});
const here = dirname(fileURLToPath(import.meta.url));
const inventory = JSON.parse(readFileSync(resolve(here, "../../../proto/tests/t3_oracle/fixtures/tools.json")));
const structural = JSON.parse(readFileSync(resolve(here, "../../../proto/tests/t3_oracle/fixtures/serde-cases.json")));
const accepted = inventory.filter((tool) => tool.phase === "core").map((tool) => {
  const arguments_ = structuredClone(structural[tool.rust.input][0]);
  if (tool.name === "create_threads") arguments_.threads = [{}];
  if (tool.name === "t3_thread_update") arguments_.title = "fixture";
  if (tool.name === "t3_thread_configure") arguments_.modelSelection = {instanceId:"mock", model:"mock"};
  if (tool.name === "t3_thread_send_attachments") {
    arguments_.attachments = [{type:"image",id:"fixture",name:"fixture",mimeType:"image/png",sizeBytes:1}];
  }
  const decoded = Schema.decodeUnknownSync(tools[tool.name].parametersSchema)(arguments_);
  return {name:tool.name, group:tool.group, arguments:arguments_, decoded};
});
writeFileSync(`${here}/accepted.json`, JSON.stringify(accepted, null, 2) + "\n");
// JSON Schema's Undefined encoding is null, but the real parameter decoder
// rejects it. Execute each AST node against null, retaining its exact diagnostic.
const nullRefusals = {};
for (const [name, tool] of Object.entries(tools)) {
  const rows = new Map();
  const walk = (ast, path) => {
    try { Schema.decodeUnknownSync(Schema.make(ast))(null); }
    catch (error) {
      if (path.length) rows.set(JSON.stringify(path), { path, description: error.message });
    }
    if (ast._tag === "Objects") {
      for (const field of ast.propertySignatures) walk(field.type, [...path, field.name]);
      for (const field of ast.indexSignatures) walk(field.type, [...path, "*"]);
    } else if (ast._tag === "Arrays") {
      for (const item of ast.rest) walk(item.type ?? item, [...path, "*"]);
    } else if (ast._tag === "Union") {
      // The union's own null diagnostic takes precedence over an arm's.
      for (const item of ast.types) {
        if (item._tag === "Objects") {
          for (const field of item.propertySignatures) walk(field.type, [...path, field.name]);
          for (const field of item.indexSignatures) walk(field.type, [...path, "*"]);
        } else if (item._tag === "Arrays") {
          for (const element of item.rest) walk(element.type ?? element, [...path, "*"]);
        }
      }
    }
  };
  walk(tool.parametersSchema.ast, []);
  nullRefusals[name] = [...rows.values()];
}
writeFileSync(`${dirname(fileURLToPath(import.meta.url))}/null-refusals.json`, JSON.stringify(nullRefusals, null, 2) + "\n");
writeFileSync(`${dirname(fileURLToPath(import.meta.url))}/validation.json`, JSON.stringify({
  tag: "v0.0.46-nightly.20261003.2632",
  commit: "f391794a35c604d57e166a3ab48d56fc6e4e469a",
  effect: "4.0.0-rc.115",
  cases,
}, null, 2) + "\n");
console.log(`Extracted ${cases.length} executed parameter/refusal cases`);
