// Noches runtime policy for Pi. Pi deliberately leaves permission policy to
// extensions; this one uses Pi's public blocking `tool_call` hook so the
// shared runtime modes keep their normal meaning without replacing or
// shadowing Pi's own tools. Confirmations travel as ordinary `confirm`
// dialogs over the RPC extension-UI protocol; the host recognizes them by the
// JSON marker in the message and routes them through its permission gate.
// Never throw: a failing hook must not take the session down.
import { realpathSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const READ_ONLY_TOOLS = new Set(["read", "grep", "find", "ls"]);
const FILE_CHANGE_TOOLS = new Set(["edit", "write"]);
const MAX_FIELD = 2_000;

type Mode = "approval-required" | "auto-accept-edits" | "full-access";

// Fail closed: the host always sets the mode, so a missing or unrecognised
// value must never widen access.
function mode(): Mode {
  const value = process.env.NOCHES_PI_RUNTIME_MODE;
  return value === "full-access" || value === "auto-accept-edits" ? value : "approval-required";
}

const UNICODE_SPACES = /[  -   　]/g;
const CASE_INSENSITIVE = process.platform === "darwin" || process.platform === "win32";

// The path a tool will touch, resolved the way Pi's own file tools do.
function targetOf(raw: string, cwd: string): string | undefined {
  let path = raw.replace(UNICODE_SPACES, " ");
  if (path.startsWith("@")) path = path.slice(1);
  if (path === "~") path = homedir();
  else if (path.startsWith("~/") || (process.platform === "win32" && path.startsWith("~\\"))) {
    path = join(homedir(), path.slice(2));
  }
  if (/^file:\/\//.test(path)) {
    try {
      path = fileURLToPath(path);
    } catch {
      return undefined;
    }
  }
  return resolve(cwd, path);
}

// Follow symlinks through the deepest ancestor that exists, so a link inside
// the workspace cannot lead a "workspace" write outside it.
function canonical(path: string): string {
  const tail: string[] = [];
  let current = path;
  for (;;) {
    try {
      return join(realpathSync.native(current), ...[...tail].reverse());
    } catch {
      const parent = dirname(current);
      if (parent === current) return path;
      tail.push(basename(current));
      current = parent;
    }
  }
}

function inside(child: string, parent: string): boolean {
  const norm = (value: string) => (CASE_INSENSITIVE ? value.toLowerCase() : value);
  const rel = relative(norm(parent), norm(child));
  return rel === "" || (rel !== ".." && !rel.startsWith(`..${sep}`) && !isAbsolute(rel));
}

function agentDir(): string {
  const env = process.env.PI_CODING_AGENT_DIR;
  if (env) {
    if (env === "~") return homedir();
    return env.startsWith("~/") ? join(homedir(), env.slice(2)) : resolve(env);
  }
  return join(homedir(), ".pi", "agent");
}

// Auto-accept covers edits inside the working directory only. Pi runs code
// from its agent dir, ~/.pi and a project's own .pi directory (extensions),
// so those always ask, and anything that cannot be placed asks too.
function editStaysInWorkspace(input: unknown, cwd: unknown): boolean {
  if (typeof cwd !== "string" || cwd === "" || typeof input !== "object" || input === null) {
    return false;
  }
  const fields = input as Record<string, unknown>;
  const raw = fields.path ?? fields.file_path;
  if (typeof raw !== "string" || raw === "") return false;
  const resolved = targetOf(raw, cwd);
  if (resolved === undefined) return false;
  const target = canonical(resolved);
  const root = canonical(resolve(cwd));
  if (!inside(target, root)) return false;
  for (const guarded of [agentDir(), join(homedir(), ".pi")]) {
    if (inside(target, canonical(guarded))) return false;
  }
  return !relative(root, target)
    .split(sep)
    .some((part) => (CASE_INSENSITIVE ? part.toLowerCase() : part) === ".pi");
}

// MCP tools the host pre-approved (exact names or a trailing-`*` wildcard).
// Pre-approval is a prompt-skip, never an authorization boundary.
function preApproved(toolName: string): boolean {
  try {
    const patterns = JSON.parse(process.env.NOCHES_SESSION_MCP_ALLOWED_TOOLS || "[]");
    return (
      Array.isArray(patterns) &&
      patterns.some(
        (pattern) =>
          typeof pattern === "string" &&
          (pattern.endsWith("*")
            ? toolName.startsWith(pattern.slice(0, -1))
            : pattern === toolName),
      )
    );
  } catch {
    return false;
  }
}

// Truncate individual string fields instead of the whole JSON, so the host
// can always parse the summary and fingerprint the exact tool input.
function summarize(value: unknown, depth = 0): unknown {
  if (typeof value === "string") {
    return value.length > MAX_FIELD ? `${value.slice(0, MAX_FIELD)}…` : value;
  }
  if (depth >= 4 || value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.slice(0, 50).map((item) => summarize(item, depth + 1));
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .slice(0, 50)
      .map(([key, item]) => [key, summarize(item, depth + 1)]),
  );
}

export default function nochesPolicy(pi: ExtensionAPI) {
  // Workaround for an upstream Pi context-budgeting bug: pi-ai reuses the
  // previous response's usage even when a fork's instructions/tools differ,
  // then reserves almost all remaining context for output, and OpenRouter can
  // reject even a short conversation. Remove when Pi accounts for the current
  // request prefix reliably.
  pi.on("before_provider_request", (event, ctx) => {
    if (ctx.model?.provider !== "openrouter") return;
    const payload = event.payload;
    if (typeof payload !== "object" || payload === null || Array.isArray(payload)) return;
    const replacement = { ...payload } as Record<string, unknown>;
    let changed = false;
    for (const key of ["max_tokens", "max_completion_tokens"]) {
      const limit = replacement[key];
      if (typeof limit === "number" && Number.isFinite(limit) && limit > 32_768) {
        replacement[key] = 32_768;
        changed = true;
      }
    }
    if (changed) return replacement;
  });

  pi.on("tool_call", async (event, ctx) => {
    const current = mode();
    if (current === "full-access" || READ_ONLY_TOOLS.has(event.toolName)) return;
    if (
      current === "auto-accept-edits" &&
      FILE_CHANGE_TOOLS.has(event.toolName) &&
      editStaysInWorkspace(event.input, ctx.cwd)
    ) {
      return;
    }
    if (preApproved(event.toolName)) return;
    let approved = false;
    try {
      approved = await ctx.ui.confirm(
        `Allow ${event.toolName}?`,
        JSON.stringify({
          noches: "permission",
          tool: event.toolName,
          input: summarize(event.input),
        }),
      );
    } catch {
      approved = false;
    }
    if (!approved) {
      return { block: true, reason: `${event.toolName} was declined in Noches.` };
    }
  });
}
