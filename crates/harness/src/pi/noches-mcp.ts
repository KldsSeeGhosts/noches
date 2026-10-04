// Loaded privately for one Pi runtime. No credentials are written to this
// extension file; server bindings arrive only in the host-local environment.
import { spawn } from "node:child_process";
import * as readline from "node:readline";
import { Type } from "typebox";

export default async function nochesMcp(pi: any) {
  const executable = process.env.NOCHES_ACP_MCP_EXECUTABLE;
  const entries = JSON.parse(process.env.NOCHES_SESSION_MCP_ENTRIES || "[]");
  const instructions = process.env.NOCHES_SESSION_MCP_INSTRUCTIONS || "";
  const peers: any[] = [];
  const registered = new Set<string>();

  pi.on("before_agent_start", (event: any) => ({
    systemPrompt: event.systemPrompt + (instructions ? "\n\n" + instructions : ""),
  }));

  async function start(entry: any) {
    const child = spawn(executable!, ["acp-mcp-bridge"], {
      stdio: ["pipe", "pipe", "ignore"],
      env: { ...process.env, NOCHES_SESSION_MCP_ENTRIES: JSON.stringify([entry]) },
    });
    peers.push(child);
    let nextId = 0;
    const pending = new Map<number, any>();
    const lines = readline.createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      let message: any;
      try { message = JSON.parse(line); } catch { return; }
      const waiter = pending.get(message.id);
      if (waiter && !message.method) {
        pending.delete(message.id);
        // Never put foreign errors (which can echo credentials) in Pi logs.
        message.error ? waiter.reject(new Error("MCP request failed")) : waiter.resolve(message.result);
      } else if (message.method && message.id !== undefined) {
        child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: message.id,
          error: { code: -32601, message: "Unsupported MCP server request" } }) + "\n");
      }
    });
    child.on("error", () => { for (const p of pending.values()) p.reject(new Error("MCP transport failed")); pending.clear(); });
    child.on("exit", () => { for (const p of pending.values()) p.reject(new Error("MCP transport closed")); pending.clear(); });
    child.stdin.on("error", () => { for (const p of pending.values()) p.reject(new Error("MCP input closed")); pending.clear(); });
    function request(method: string, params: any, signal?: AbortSignal) {
      return new Promise<any>((resolve, reject) => {
        const id = ++nextId;
        const abort = () => {
          pending.delete(id);
          child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/cancelled",
            params: { requestId: id, reason: "cancelled" } }) + "\n");
          reject(new Error("MCP request cancelled"));
        };
        if (signal?.aborted) return abort();
        signal?.addEventListener("abort", abort, { once: true });
        pending.set(id, {
          resolve: (result: any) => { signal?.removeEventListener("abort", abort); resolve(result); },
          reject: (error: any) => { signal?.removeEventListener("abort", abort); reject(error); },
        });
        child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
      });
    }
    try {
      await request("initialize", { protocolVersion: "2025-06-18", capabilities: {},
        clientInfo: { name: "noches-pi-mcp", version: "1.0.0" } }, AbortSignal.timeout(10_000));
      child.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');
      let cursor: string | undefined;
      const seen = new Set<string>();
      const tools: any[] = [];
      do {
        const page = await request("tools/list", cursor ? { cursor } : {}, AbortSignal.timeout(10_000));
        tools.push(...(page.tools || []));
        cursor = page.nextCursor;
        if (cursor && seen.has(cursor)) throw new Error("MCP pagination loop");
        if (cursor) seen.add(cursor);
        if (seen.size > 128 || tools.length > 10_000) throw new Error("MCP discovery limit exceeded");
      } while (cursor);
      // Register only after complete discovery, so a startup retry cannot
      // leave tool closures attached to a failed peer.
      for (const tool of tools) {
        const name = `mcp__${entry.name}__${tool.name}`;
        if (registered.has(name)) continue;
        pi.registerTool({
          name, label: tool.name, description: tool.description || tool.name,
          parameters: Type.Unsafe ? Type.Unsafe(tool.inputSchema || { type: "object" }) : Type.Object({}, { additionalProperties: true }),
          async execute(_id: string, params: any, signal: AbortSignal) {
            const result = await request("tools/call", { name: tool.name, arguments: params || {} }, signal);
            return { content: result.content || [{ type: "text", text: JSON.stringify(result.structuredContent ?? result) }],
              details: { server: entry.name, tool: tool.name, structuredContent: result.structuredContent },
              ...(result.isError ? { isError: true } : {}) };
          },
        });
        registered.add(name);
      }
    } catch {
      child.stdin.end();
      child.kill();
      throw new Error("MCP initialization failed");
    }
  }

  const started = new Set<string>();
  async function ensureStarted() {
    if (!executable) return;
    for (const entry of entries) {
      if (started.has(entry.name)) continue;
      await start(entry);
      started.add(entry.name);
    }
  }
  await ensureStarted().catch(() => undefined);
  pi.on("session_start", async (_event: any, ctx: any) => {
    try { await ensureStarted(); }
    catch { ctx.ui.notify("Noches MCP unavailable; retry the session.", "warning"); }
  });
  pi.on("session_shutdown", () => {
    for (const child of peers) {
      child.stdin.end();
      // Let the Rust bridge reap its MCP child before escalating.
      setTimeout(() => { if (child.exitCode === null) child.kill("SIGKILL"); }, 2_000).unref();
    }
  });
}
