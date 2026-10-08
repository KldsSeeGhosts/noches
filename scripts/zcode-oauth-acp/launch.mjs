#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { Transform, Writable } from "node:stream";
import { StringDecoder } from "node:string_decoder";
import { pathToFileURL } from "node:url";
import { installOAuthBridge, normalizeModelOptions, oauthModels } from "./oauth.mjs";

// Prefer the isolated, pinned install beside this launcher. An explicit package
// directory is useful for diagnostics without altering the global npm package.
const require = createRequire(import.meta.url);
const packageDir = process.env.ZCODE_ACP_PACKAGE
  || dirname(require.resolve("zcode-acp-server/package.json"));
const pkg = JSON.parse(readFileSync(join(packageDir, "package.json"), "utf8"));
if (pkg.version !== "0.65.1") {
  throw new Error("This OAuth extension requires zcode-acp-server 0.65.1. Install the pinned dependency beside the launcher.");
}
const load = (file) => import(pathToFileURL(join(packageDir, "dist", file)).href);
const [{ ZcodeAcpServer }, { builtinProviderEnv, resolveZcodeCommand, zcodeDataBaseDirEnv }] = await Promise.all([
  load("server.js"), load("backend/resolve.js"),
]);
process.env.ZCODE_NODE ||= process.execPath;
if (process.argv[2] === "login") {
  // Only an explicit user invocation starts OAuth. The native CLI owns the
  // browser flow and all credential writes, with the bundle's provider table.
  const argv = resolveZcodeCommand();
  const index = argv.indexOf("app-server");
  if (index < 1) throw new Error("Unable to locate the native ZCode CLI login command.");
  const result = spawnSync(argv[0], [...argv.slice(1, index), "login", ...process.argv.slice(3)], {
    env: { ...process.env, ...builtinProviderEnv(), ...zcodeDataBaseDirEnv() },
    stdio: "inherit",
  });
  process.exit(result.status ?? 1);
}
installOAuthBridge(ZcodeAcpServer);

const catalog = () => {
  const file = builtinProviderEnv().ZCODE_BUILTIN_PROVIDER_CONFIG_FILE;
  if (!file) return [];
  try { return oauthModels(JSON.parse(readFileSync(file, "utf8"))); } catch { return []; }
};
function normalize(frame) {
  // Only catalog metadata changes. Text, tools, usage and permission requests
  // pass through unchanged. The existing bridge still owns the ACP protocol.
  if (frame.result?.configOptions) {
    frame.result.configOptions = normalizeModelOptions(frame.result.configOptions, catalog());
  }
  if (frame.params?.update?.configOptions) {
    frame.params.update.configOptions = normalizeModelOptions(frame.params.update.configOptions, catalog());
  }
  return frame;
}

// Upstream's stdio entry point builds its own server and transport. Intercept
// only its outbound metadata using its supported web-stream construction seam,
// so the entire upstream ACP method surface remains intact.
const toWeb = Writable.toWeb;
Writable.toWeb = function (stream, ...args) {
  if (stream !== process.stdout) return toWeb.call(this, stream, ...args);
  let buffered = "";
  const decoder = new StringDecoder("utf8");
  const metadata = new Transform({
    transform(chunk, encoding, done) {
      try {
        buffered += decoder.write(chunk);
        let end;
        while ((end = buffered.indexOf("\n")) !== -1) {
          const line = buffered.slice(0, end);
          buffered = buffered.slice(end + 1);
          if (line.trim()) this.push(`${JSON.stringify(normalize(JSON.parse(line)))}\n`);
        }
        done();
      } catch { done(new Error("Unable to encode ZCode ACP metadata.")); }
    },
  });
  metadata.pipe(process.stdout);
  return toWeb.call(this, metadata, ...args);
};
const { main } = await load("index.js");
await main();
