#!/usr/bin/env node
// T3 Tools MIT source is evaluated ONLY in an isolated temporary copy.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const TAG = "v0.0.46-nightly.20261003.2632";
const COMMIT = "f391794a35c604d57e166a3ab48d56fc6e4e469a";
const replayRoot="apps/server/src/orchestration-v2";
const replayFixtures=["claude_nested_background_subagent_wake","claude_background_wake_before_queued_prompt","claude_background_task_after_root","claude_background_task_interrupt"];
const replayArchivePaths=[
  ...replayFixtures.map((name)=>`${replayRoot}/testkit/fixtures/${name}`),
  ...["SubagentProjection.ts","QueuedRunOrder.ts","NotificationMailbox.ts"].map((name)=>`${replayRoot}/${name}`),
];
const here = dirname(fileURLToPath(import.meta.url));
const hash = (v) => createHash("sha256").update(v).digest("hex");
const canonical = (v) => Array.isArray(v) ? v.map(canonical)
  : v && typeof v === "object"
    ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canonical(v[k])])) : v;
const json = (v) => JSON.stringify(canonical(v), null, 2) + "\n";

if (process.argv[2] !== "--worker") {
  const source = resolve(process.argv[2] ?? "/Volumes/DevDrive/AiStack/t3code-ref");
  const output = resolve(process.argv[3] ?? join(here, "fixtures"));
  const commit = execFileSync("git", ["-C", source, "rev-parse", `${TAG}^{commit}`], { encoding: "utf8" }).trim();
  if (commit !== COMMIT) throw new Error(`Pinned tag moved: expected ${COMMIT}, found ${commit}`);
  const scratch = mkdtempSync("/tmp/noches-t3-oracle-");
  const archive = execFileSync("git", ["-C", source, "archive", commit, "packages/contracts", "apps/server/src/mcp", "LICENSE",...replayArchivePaths], { maxBuffer: 32 * 1024 * 1024 });
  execFileSync("tar", ["-x", "-C", scratch], { input: archive });
  writeFileSync(join(scratch, "package.json"), json({
    name: "noches-t3-oracle-extraction", private: true, type: "module", dependencies: { effect: "4.0.0-rc.115" },
  }));
  const lockPath = join(here,"fixtures/extraction-package-lock.json");
  if (existsSync(lockPath)) writeFileSync(join(scratch,"package-lock.json"),readFileSync(lockPath));
  execFileSync("npm", [existsSync(lockPath) ? "ci" : "install", "--ignore-scripts", "--no-audit", "--no-fund"], { cwd: scratch, stdio: "inherit" });
  const worker = join(scratch, "extract.mjs");
  writeFileSync(worker, readFileSync(fileURLToPath(import.meta.url)));
  execFileSync(process.execPath, [worker, "--worker", scratch, output, commit], { stdio: "inherit" });
  console.log(`Oracle written to ${output}; isolated source/dependencies retained at ${scratch}`);
  process.exit(0);
}

const [scratch, output, commit] = process.argv.slice(3);
const Schema = await import("effect/Schema");
const Context = await import("effect/Context");
const Option = await import("effect/Option");
const Tool = await import("effect/unstable/ai/Tool");
const AiError = await import("effect/unstable/ai/AiError");
const McpSchema = await import("effect/unstable/ai/McpSchema");
const jsonSchema = (s) => Tool.getJsonSchemaFromSchema(s);
const contractsDir = join(scratch, "packages/contracts/src");
const contracts = await import(pathToFileURL(join(contractsDir, "index.ts")).href);
const sources = {};
sources.LICENSE=hash(readFileSync(join(scratch,"LICENSE")));
const modules = {};
for (const file of readdirSync(contractsDir).filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts")).sort()) {
  sources[`packages/contracts/src/${file}`] = hash(readFileSync(join(contractsDir, file)));
  modules[file] = await import(pathToFileURL(join(contractsDir, file)).href);
}

const groups = ["orchestrator", "thread", "worktree", "pullRequests", "project", "environment", "attachment", "preview", "previewControls", "device"];
const refusalPaths=[
  ...groups.map((group)=>`apps/server/src/mcp/toolkits/${group}/handlers.ts`),
  ...["OrchestratorMcpService.ts","ThreadMetadataMcpService.ts","WorktreeMcpService.ts","threadAccess.ts"].map((file)=>`apps/server/src/mcp/${file}`),
];
const refusalSources=refusalPaths.filter((p)=>existsSync(join(scratch,p))).map((source)=>{
  const text=readFileSync(join(scratch,source),"utf8");
  const sha256=hash(text);
  sources[source]=sha256;
  return {source,sha256,text};
});
const scenarioPaths=[
  ...replayFixtures.flatMap((name)=>["input.ts","output.ts","claude_transcript.ndjson"].map((file)=>`${replayRoot}/testkit/fixtures/${name}/${file}`)),
  ...replayArchivePaths.filter((p)=>p.endsWith(".ts")),
];
const scenarioSources=scenarioPaths.map((source)=>{
  const text=readFileSync(join(scratch,source),"utf8");
  const sha256=hash(text);
  sources[source]=sha256;
  return {source,sha256,text};
});
const tools = [];
const toolSchemas = new Map();
const attachmentInputPath = "apps/server/src/mcp/toolkits/attachment/input.ts";
const attachmentInput = readFileSync(join(scratch, attachmentInputPath), "utf8");
sources[attachmentInputPath] = hash(attachmentInput);
writeFileSync(join(scratch, attachmentInputPath), attachmentInput.replaceAll('"@t3tools/contracts"', JSON.stringify(pathToFileURL(join(contractsDir, "index.ts")).href)));
const httpSourcePath = "apps/server/src/mcp/McpHttpServer.ts";
const httpSource = readFileSync(join(scratch,httpSourcePath),"utf8");
sources[httpSourcePath] = hash(httpSource);
for (const group of groups) {
  const file = `apps/server/src/mcp/toolkits/${group}/tools.ts`;
  const original = readFileSync(join(scratch, file), "utf8");
  sources[file] = hash(original);
  // Only replace service-token imports. No schema or Tool.make expression is modified.
  const transformed = original.replace(/import \* as (\w+) from "(\.[^"]+)";/g,
    (_, name) => `const ${name} = new Proxy({}, {get: (_, key) => String(key)});`)
    .replaceAll('"@t3tools/contracts"', JSON.stringify(pathToFileURL(join(contractsDir, "index.ts")).href));
  writeFileSync(join(scratch, file), transformed);
  const module = await import(pathToFileURL(join(scratch, file)).href);
  const seen = new Set();
  for (const value of Object.values(module)) {
    const candidates = value?.tools ? Object.values(value.tools) : value?.name && value?.parameters ? [value] : [];
    for (const tool of candidates) {
      if (seen.has(tool.name)) continue;
      seen.add(tool.name);
      const annotations = {};
      for (const [key, tag] of [["title", Tool.Title], ["readOnlyHint", Tool.Readonly], ["destructiveHint", Tool.Destructive], ["idempotentHint", Tool.Idempotent], ["openWorldHint", Tool.OpenWorld]]) {
        const value = Option.getOrUndefined(Context.getOption(tool.annotations, tag));
        if (value !== undefined) annotations[key] = value;
      }
      const handRegistered = ["preview_snapshot","device_screenshot"].includes(tool.name);
      const {title,...hints}=annotations;
      // Match the pinned registration literally. Effect's standard registration
      // spreads the title string instead of wrapping it in {title}; encoding
      // removes those character-index keys. T3's two manual registrations fix it.
      const wireAnnotations=handRegistered ? annotations : {...title,...hints};
      const resultSchema=jsonSchema(tool.successSchema);
      const descriptor=Schema.encodeSync(McpSchema.Tool)(new McpSchema.Tool({
        name:tool.name, description:Tool.getDescription(tool),inputSchema:Tool.getJsonSchema(tool),
        ...(!handRegistered && resultSchema.type==="object" ? {outputSchema:resultSchema} : {}),
        annotations:wireAnnotations,
      }));
      tools.push({ name: tool.name, group, phase: ["preview", "previewControls", "device"].includes(group) ? "later" : "core",
        description: Tool.getDescription(tool), annotations:descriptor.annotations, sourceAnnotations:annotations, descriptor, failureMode: tool.failureMode,
        inputSchema: Tool.getJsonSchema(tool),
        decodedInputSchema: jsonSchema(Schema.toCodecJson(Schema.toType(tool.parametersSchema))),
        resultSchema,
        errorSchema: jsonSchema(Schema.toCodecJson(tool.failureSchema)),
        failureResultSchema: jsonSchema(Tool.failureResultSchema(tool)),
        framing: {
          normalResult: { isError: false, structuredContent: "object result; omitted for non-objects", content: "JSON text of encodedResult; empty for undefined" },
          declaredFailure: tool.failureMode === "return" ? "encoded failureResultSchema in normalResult (isError:false)" : "error text MCP result",
          invalidParameters: tool.failureMode === "return" ? "encoded AiError/ToolParameterValidationError in normalResult" : "JSON-RPC InvalidParams",
        },
      });
      toolSchemas.set(tool.name, tool);
      if (tool.name === "preview_snapshot" || tool.name === "device_screenshot") {
        tools.at(-1).transport = {
          registration: "hand-registered image content",
          source: httpSourcePath,
          structuredContent: "success metadata with screenshot.data removed; PNG bytes are an image content block",
          errorEnvelopeFields: tool.name === "preview_snapshot"
            ? ["_tag","operation","failureCount","message?"] : ["_tag","operation","failureCount"],
          fallbackErrorTag: tool.name === "preview_snapshot" ? "PreviewSnapshotError" : "device_screenshotError",
          additionalFailureTags: tool.name === "preview_snapshot" ? ["PreviewScreenshotSaveError"] : [],
        };
      }
    }
  }
}
if (tools.length !== 72 || tools.filter((t) => t.phase === "core").length !== 52
    || new Set(tools.map((t) => t.name)).size !== 72) throw new Error("Pinned 72/52/20 tool inventory changed");

// AST identities retain branded IDs and named contracts, unlike expanded JSON Schema.
const names = new Map();
const exported = new Map();
for (const [file, module] of Object.entries(modules)) {
  for (const [name, value] of Object.entries(module)) {
    if (!Schema.isSchema(value)) continue;
    exported.set(name, { file, value });
    if (!names.has(value.ast)) names.set(value.ast, name);
  }
}
for (const [name,value] of [["ToolFrameworkAiError",AiError.AiError],["ToolFrameworkExecutionFailure",Tool.ExecutionFailure]]) {
  exported.set(name,{file:"effect-framework",value});
  names.set(value.ast,name);
}
const roots = ["orchestrationV2.ts", "orchestratorMcp.ts", "providerInstance.ts", "model.ts", "modelSelection.ts", "scheduledTask.ts", "threadPullRequest.ts", "applicationEvent.ts"];
const definitions = {};
const origins = {};
function define(name) {
  if (name in definitions) return;
  definitions[name] = null;
  const { file, value } = exported.get(name);
  origins[name] = file;
  definitions[name] = shape(value.ast, name);
}
function shape(ast, own, ancestors = new Set()) {
  if (ancestors.has(ast)) return { kind: "unknown", recursiveJson: true };
  const next = new Set(ancestors).add(ast);
  const branded = ast.checks?.flatMap((c) => c.annotations?.brands ?? []).find((n) => exported.has(n));
  const ref = branded ?? names.get(ast);
  if (ref && ref !== own && ast._tag !== "Undefined") {
    define(ref);
    return { kind: "ref", name: ref };
  }
  switch (ast._tag) {
    case "String": return { kind: "string" };
    case "Number": return { kind: "number", integer: ast.checks?.some((c) => c.annotations?.toJsonSchema?.().type === "integer") ?? false };
    case "Boolean": return { kind: "boolean" };
    case "Literal": return { kind: "literal", value: ast.literal };
    case "Null": return { kind: "null" };
    case "Undefined": return { kind: "undefined" };
    case "Union": return { kind: "union", members: ast.types.map((a) => shape(a, own, next)) };
    case "Arrays": {
      if (ast.elements.length) {
        const elements=ast.elements.map((e)=>shape(e.type??e,own,next));
        if (!ast.rest.length) return {kind:"tuple",elements};
        const element=shape(ast.rest[0].type??ast.rest[0],own,next);
        if(elements.every((e)=>JSON.stringify(e)===JSON.stringify(element)))
          return {kind:"array",element,minItems:elements.length};
        throw new Error("Heterogeneous variadic tuple needs explicit support");
      }
      return { kind: "array", element: shape(ast.rest[0].type ?? ast.rest[0], own, next) };
    }
    case "Objects": {
      const fields = ast.propertySignatures.map((p) => {
        let defaultValue;
        try { defaultValue = Schema.decodeUnknownSync(Schema.make(p.type))(undefined); } catch {}
        return { name: p.name, optional: p.type.context?.isOptional ?? false,
          ...(defaultValue === undefined ? {} : { default: defaultValue }),
          shape: shape(p.type, own, next) };
      });
      if (ast.indexSignatures.length && fields.length) throw new Error("Mixed index/field contract needs explicit support");
      return ast.indexSignatures.length
        ? { kind: "record", key:shape(ast.indexSignatures[0].parameter,own,next), value: shape(ast.indexSignatures[0].type, own, next) }
        : { kind: "object", fields };
    }
    case "Declaration":
      if (ast.typeParameters[0]?._tag === "Objects" && ast.annotations?.["~sentinels"]) return shape(ast.typeParameters[0], own, next);
      // The domain stores DateTimeUtc; JSON transport encodes ISO strings.
      if (ast.annotations?.typeConstructor?._tag === "DateTimeUtc" || jsonSchema(Schema.toCodecJson(Schema.make(ast))).type === "string") return { kind: "string" };
      return { kind: "unknown", declaration: ast.annotations?.identifier ?? "opaque" };
    case "Suspend": return shape(ast.thunk(), own, next);
    case "Unknown": case "Any": case "ObjectKeyword": return { kind: "unknown" };
    case "Never": return { kind: "never" };
    default: throw new Error(`Unsupported AST ${ast._tag}`);
  }
}
for (const [name, {file}] of exported) if (roots.includes(file)) define(name);
define("ToolFrameworkAiError");
define("ToolFrameworkExecutionFailure");
for (const tool of tools) {
  const value = toolSchemas.get(tool.name);
  const prefix = tool.name.split("_").map((s) => s[0].toUpperCase() + s.slice(1)).join("");
  tool.rust = {};
  for (const [suffix, schema] of [["Input", Schema.toEncoded(value.parametersSchema)], ["Result", value.successSchema], ["Error", Tool.failureResultSchema(value)]]) {
    const name = prefix + suffix;
    definitions[name] = shape(schema.ast, name);
    origins[name] = `toolkit:${tool.group}`;
    tool.rust[suffix.toLowerCase()] = name;
  }
  // Failure codes and tags are both retained, including non-central error families.
  const tags = new Set();
  const codes = new Set();
  function errorFamilies(schema, root=schema, seen=new Set()) {
    if (schema.$ref) {
      if(seen.has(schema.$ref)) return [];
      const ref=schema.$ref.split("/").slice(1).reduce((v,k)=>v?.[k.replaceAll("~1","/").replaceAll("~0","~")],root);
      return ref ? errorFamilies(ref,root,new Set([...seen,schema.$ref])) : [];
    }
    const tag=schema.properties?._tag;
    const families=tag ? (tag.enum ?? (tag.const ? [tag.const] : [])) : [];
    const execution=(schema.properties?.type?.enum??[]).filter((t)=>t.startsWith("execution-"));
    return [...families,...execution,
      ...(schema.anyOf??schema.oneOf??[]).flatMap((s)=>errorFamilies(s,root,seen)),
      ...(schema.properties?.reason && typeof schema.properties.reason==="object" ? errorFamilies(schema.properties.reason,root,seen) : [])];
  }
  function errors(s) {
    if (!s || typeof s !== "object") return;
    if (s.properties?._tag?.const) tags.add(s.properties._tag.const);
    for (const tag of s.properties?._tag?.enum ?? []) tags.add(tag);
    for (const tag of s.properties?.type?.enum ?? [])
      if (["execution-denied","execution-interrupted"].includes(tag)) tags.add(tag);
    for (const c of s.properties?.code?.enum ?? []) codes.add(c);
    for (const v of Object.values(s)) if (v && typeof v === "object") Array.isArray(v) ? v.forEach(errors) : errors(v);
  }
  errors(tool.errorSchema);
  tool.declaredErrorTags = [...new Set(errorFamilies(tool.errorSchema))].sort();
  tool.refusalCodes = [...codes].sort();
  errors(tool.failureResultSchema);
  tool.errorTags = [...new Set(errorFamilies(tool.failureResultSchema))].sort();
  tool.schemaTags = [...tags].sort();
}
mkdirSync(output, { recursive: true });
// Execute real pinned codecs (not a JS reimplementation of their rules).
const codecInputs = [
  ["ProviderInstanceEnvironmentVariable", { name: "EXAMPLE" }],
  ["ProviderOptionSelections", { " effort ": " high ", fastMode: true, invalid: 42, empty: "" }],
  ["ModelSelection", { provider: "codex", model: " example ", options: { effort: "high", invalid: null } }],
  ["OrchestratorMcpTargetOptions", { effort: "high", fastMode: true }],
  ["OrchestratorMcpTargetOptions", { effort: 42 }],
  ["OrchestratorMcpScheduleTaskInput", { prompt: " hello ", schedule: {type:"interval",everyMs:60000} }],
  ["OrchestratorMcpScheduleTaskInput", { prompt: "hello", schedule: '{"type":"interval","everyMs":60000}' }],
  ["OrchestratorMcpScheduleTaskInput", { prompt: "hello", schedule: {type:"interval",everyMs:59999} }],
  ["OrchestratorMcpScheduleTaskInput", { prompt: "hello", schedule: {type:"fixed_time",timeOfDay:"9:00",weekdays:[1,2,3]} }],
  ["ScheduledTaskSchedule", {type:"interval",everyMs:1}],
  ["ScheduledTaskUpsertSchedule", {type:"interval",everyMs:1}],
  ["OrchestrationV2PendingBackgroundTask", {taskId:"task"}],
  ["OrchestrationV2PendingBackgroundTask", {taskId:"task",kind:"future_kind",description:"example"}],
  ["OrchestrationV2PendingBackgroundTask", {kind:"subagent"}],
  ["OrchestrationV2NotificationSource", {kind:"subagent",childThreadId:"child"}],
  ["OrchestrationV2NotificationSource", {kind:"command"}],
  ["OrchestrationV2NotificationSource", {kind:"future_kind"}],
  ["OrchestrationV2NotificationSource", {}],
];
const codecCases = codecInputs.map(([name,input]) => {
  const schema = exported.get(name).value;
  try {
    const decoded = Schema.decodeUnknownSync(schema)(input);
    const encoded = Schema.encodeSync(schema)(decoded);
    return { name, input, accepted: true, encoded };
  } catch (error) { return { name, input, accepted: false, errorTag: error._tag }; }
});
writeFileSync(join(output,"codec-cases.json"),json(codecCases));
writeFileSync(join(output,"refusal-sources.json"),json(refusalSources));
writeFileSync(join(output,"scenario-sources.json"),json(scenarioSources));
writeFileSync(join(output, "tools.json"), json(tools));
writeFileSync(join(output, "contracts.json"), json({ definitions, origins }));
writeFileSync(join(output,"extraction-package-lock.json"),readFileSync(join(scratch,"package-lock.json")));
writeFileSync(join(output,"LICENSE.effect"),readFileSync(join(scratch,"node_modules/effect/LICENSE")));
writeFileSync(join(output, "mcp-framing.ts"),httpSource);
for (const name of ["orchestrationV2.ts", "orchestratorMcp.ts"]) {
  writeFileSync(join(output, name), readFileSync(join(contractsDir, name)));
}
writeFileSync(join(output, "provenance.json"), json({ tag: TAG, commit, license: "MIT", copyright: "Copyright (c) 2026 T3 Tools Inc.", effect: "4.0.0-rc.115", node: ">=24.13.1 (native TypeScript stripping)", sources }));
console.log(`Extracted ${tools.length} tools`);
