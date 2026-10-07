// Noches runtime policy for Pi. Pi deliberately leaves permission policy to
// extensions; this one uses Pi's public blocking `tool_call` hook so the
// shared runtime modes keep their normal meaning without replacing or
// shadowing Pi's own tools. Confirmations travel as ordinary `confirm`
// dialogs over the RPC extension-UI protocol; the host recognizes them by the
// JSON marker in the message and routes them through its permission gate.
// Never throw: a failing hook must not take the session down.
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const READ_ONLY_TOOLS = new Set(["read", "grep", "find", "ls"]);
const FILE_CHANGE_TOOLS = new Set(["edit", "write"]);
const MAX_FIELD = 2_000;

type Mode = "approval-required" | "auto-accept-edits" | "full-access";

function mode(): Mode {
  const value = process.env.NOCHES_PI_RUNTIME_MODE;
  return value === "approval-required" || value === "auto-accept-edits" ? value : "full-access";
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
    if (current === "auto-accept-edits" && FILE_CHANGE_TOOLS.has(event.toolName)) return;
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
