import { createDecipheriv, createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { homedir, platform, userInfo } from "node:os";
import { join } from "node:path";

// Mirrors ZCode CLI 0.16.9's standalone account credential port. OAuth login
// obtains this subscription credential itself; users never supply an API key.
const ACCOUNT = /^account:(zai|bigmodel)-individual-coding-plan$/;
export const LOGIN_REQUIRED = "ZCode OAuth credentials are unavailable. Run this launcher's 'login' command in your terminal, then reconnect this local ACP provider.";

export function credentialPath(env = process.env) {
  if (env.ZCODE_HOME) return join(env.ZCODE_HOME, "v2", "credentials.json");
  return join(env.ZCODE_DATA_BASE_DIR || homedir(), ".zcode", "v2", "credentials.json");
}

export function decryptCredential(value, env = process.env) {
  if (typeof value !== "string") throw new Error(LOGIN_REQUIRED);
  if (!value.startsWith("enc:v1:")) return value;
  const parts = value.slice(7).split(".");
  if (parts.length !== 3) throw new Error(LOGIN_REQUIRED);
  const [iv, tag, ciphertext] = parts.map((part) => Buffer.from(part, "base64url"));
  if (iv.length !== 12 || tag.length !== 16) throw new Error(LOGIN_REQUIRED);
  let username = "unknown";
  try { username = userInfo().username; } catch {}
  const secret = env.ZCODE_CREDENTIAL_SECRET?.trim()
    || `zcode-credential-fallback:${platform()}:${homedir()}:${username}`;
  try {
    const cipher = createDecipheriv("aes-256-gcm", createHash("sha256").update(secret).digest(), iv);
    cipher.setAuthTag(tag);
    return Buffer.concat([cipher.update(ciphertext), cipher.final()]).toString("utf8");
  } catch {
    // Never expose the store, ciphertext, identities, or underlying exceptions.
    throw new Error(LOGIN_REQUIRED);
  }
}

export function accountAuth(providerId, record, env = process.env) {
  if (!ACCOUNT.test(providerId ?? "")) return null;
  const identity = decryptCredential(record[`account-provider:${providerId}:identity`], env).trim();
  if (!identity) throw new Error(LOGIN_REQUIRED);
  const key = `account-provider:coding-plan:${providerId}:account:${encodeURIComponent(identity)}:api-key`;
  const apiKey = decryptCredential(record[key], env).trim();
  if (!apiKey) throw new Error(LOGIN_REQUIRED);
  return { apiKey };
}

export function readAccountAuth(providerId, env = process.env) {
  if (!ACCOUNT.test(providerId ?? "")) return null;
  try {
    return accountAuth(providerId, JSON.parse(readFileSync(credentialPath(env), "utf8")), env);
  } catch {
    throw new Error(LOGIN_REQUIRED);
  }
}

export function oauthModels(table, getAuth = readAccountAuth) {
  return (table.config?.providerConfigRules?.providerRules ?? []).flatMap((rule) => {
    if (rule.config?.access?.mode !== "individual-coding-plan") return [];
    try {
      if (!getAuth(rule.providerId)) return [];
    } catch { return []; }
    return (rule.config.builtinModelIds ?? []).map((modelId) => ({
      value: `${rule.providerId}\\${modelId}`,
      name: modelId,
    }));
  });
}

export function oauthEntitlements(snapshot, getAuth = readAccountAuth) {
  const providers = {};
  const states = {};
  for (const [id, provider] of Object.entries(snapshot.providers ?? {})) {
    let entitled = false;
    try { entitled = Boolean(getAuth(id)); } catch {}
    providers[id] = { ...provider, access: { ...provider.access, entitled } };
    states[id] = {
      ...snapshot.states?.[id],
      entitled,
      current: entitled,
      availability: entitled ? "available" : "unavailable",
    };
  }
  return { ...snapshot, providers, states };
}

export function normalizeModelOptions(options, models) {
  if (!Array.isArray(options)) return options;
  return options.map((option) => {
    if (option.category !== "model" || option.type !== "select") return option;
    const current = option.currentValue?.replace(
      /^builtin:(zai|bigmodel)-coding-plan\\/,
      "account:$1-individual-coding-plan\\",
    );
    return {
      ...option,
      options: models,
      currentValue: models.some((model) => model.value === current) ? current : (models[0]?.value ?? ""),
    };
  });
}

// This compatibility extension is deliberately version-pinned. It changes
// only the account auth seam; upstream retains tools, MCP, streaming, session
// persistence, replay, permission requests, reasoning, fork and cancellation.
export function installOAuthBridge(Server, getAuth = readAccountAuth) {
  const original = Server.prototype.ensureBackend;
  const initialized = new WeakSet();
  Server.prototype.ensureBackend = async function (...args) {
    const backend = await original.apply(this, args);
    if (initialized.has(backend)) return backend;
    initialized.add(backend);
    const request = backend.request.bind(backend);
    backend.request = (id, method, params, ...rest) => {
      if (method === "provider/updateAccountConfig") params = oauthEntitlements(params, getAuth);
      // Do not push legacy provider keys into the account registry. Each native
      // account request is authenticated by the OAuth responder below.
      if (method === "workspace/updateProviderRegistry") {
        return Promise.resolve({ id, result: { status: "skipped" } });
      }
      return request(id, method, params, ...rest);
    };
    backend.providerRuntimeHeadersResponder = (id, params) => {
      setImmediate(() => {
        try {
          const providerId = params.modelSelection?.providerId ?? params.providerId;
          if (params.providerId && params.providerId !== providerId) throw new Error(LOGIN_REQUIRED);
          if (params.accountAccess?.mode && params.accountAccess.mode !== "individual-coding-plan") {
            throw new Error("This local ACP launcher supports OAuth individual Coding Plan accounts only.");
          }
          const requestAuth = getAuth(providerId);
          if (!requestAuth) throw new Error("This local ACP launcher supports OAuth individual Coding Plan accounts only.");
          backend.sendReply(id, { headersApplied: true, requestAuth });
        } catch {
          try {
            backend.sendReply(id, { headersApplied: false, errorMessage: LOGIN_REQUIRED });
          } catch { /* Backend closed while the response was scheduled. */ }
        }
      });
      return true;
    };
    return backend;
  };
  const initialize = Server.prototype.initialize;
  Server.prototype.initialize = async function (...args) {
    const result = await initialize.apply(this, args);
    return {
      ...result,
      agentInfo: { ...result.agentInfo, name: "zcode-oauth-acp", title: "ZCode (OAuth)" },
      authMethods: [{
        id: "zcode-credentials",
        name: "ZCode OAuth subscription",
        description: "Uses the existing ZCode CLI OAuth login. No API key configuration. Run this launcher's 'login' command in your terminal if disconnected.",
      }],
    };
  };
}
