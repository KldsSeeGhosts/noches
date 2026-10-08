import assert from "node:assert/strict";
import { createCipheriv, createHash, randomBytes } from "node:crypto";
import { setImmediate as nextTick } from "node:timers/promises";
import test from "node:test";
import {
  accountAuth, credentialPath, decryptCredential, installOAuthBridge,
  LOGIN_REQUIRED, normalizeModelOptions, oauthEntitlements, oauthModels,
} from "./oauth.mjs";

const providerId = "account:zai-individual-coding-plan";
const env = { ZCODE_CREDENTIAL_SECRET: "test-secret-not-a-real-credential" };
function encrypt(value) {
  const iv = randomBytes(12);
  const cipher = createCipheriv("aes-256-gcm", createHash("sha256").update(env.ZCODE_CREDENTIAL_SECRET).digest(), iv);
  const ciphertext = Buffer.concat([cipher.update(value), cipher.final()]);
  return `enc:v1:${[iv, cipher.getAuthTag(), ciphertext].map((part) => part.toString("base64url")).join(".")}`;
}
function record(identity = "fixture / account") {
  return {
    [`account-provider:${providerId}:identity`]: encrypt(identity),
    [`account-provider:coding-plan:${providerId}:account:${encodeURIComponent(identity)}:api-key`]: encrypt("fixture-subscription-credential"),
    // Unrelated OAuth tokens must never be read as model request credentials.
    "oauth:zai:access_token": "unrelated-token",
  };
}

test("reads exactly the native encrypted identity/credential pair", () => {
  assert.deepEqual(accountAuth(providerId, record(), env), { apiKey: "fixture-subscription-credential" });
});
test("requires both halves, with no OAuth token or legacy-key fallback", () => {
  assert.throws(() => accountAuth(providerId, { "oauth:zai:access_token": "token" }, env), { message: LOGIN_REQUIRED });
  const data = record();
  delete data[Object.keys(data).find((key) => key.endsWith(":api-key"))];
  assert.throws(() => accountAuth(providerId, data, env), { message: LOGIN_REQUIRED });
});
test("rejects wrong encryption secrets and corrupt stores without exposing values", () => {
  for (const value of ["enc:v1:invalid", "enc:v1:a.b.c", encrypt("never-log-this")]) {
    assert.throws(() => decryptCredential(value, { ZCODE_CREDENTIAL_SECRET: "wrong" }), { message: LOGIN_REQUIRED });
  }
});
test("does not serve Start Plan, team, off-peak, or third-party providers", () => {
  for (const id of ["account:zai-start-plan", "account:zai-team-coding-plan", "account:zai-off-peak", "custom:example"]) {
    assert.equal(accountAuth(id, record(), env), null);
  }
});
test("respects native and bridge data-root overrides", () => {
  assert.equal(credentialPath({ ZCODE_HOME: "/tmp/fixture-zcode" }), "/tmp/fixture-zcode/v2/credentials.json");
  assert.equal(credentialPath({ ZCODE_DATA_BASE_DIR: "/tmp/fixture-base" }), "/tmp/fixture-base/.zcode/v2/credentials.json");
});
test("catalog uses installed native models and only authenticated accounts", () => {
  const table = { config: { providerConfigRules: { providerRules: [
    { providerId, config: { access: { mode: "individual-coding-plan" }, builtinModelIds: ["GLM-5.3", "GLM-5.3-Flash"] } },
    { providerId: "account:bigmodel-individual-coding-plan", config: { access: { mode: "individual-coding-plan" }, builtinModelIds: ["unheld-model"] } },
    { providerId: "account:zai-start-plan", config: { access: { mode: "start-plan" }, builtinModelIds: ["unsupported-model"] } },
  ] } } };
  const models = oauthModels(table, (id) => id === providerId ? { apiKey: "fixture" } : null);
  assert.deepEqual(models.map((model) => model.name), ["GLM-5.3", "GLM-5.3-Flash"]);
});
test("entitlements use saved credentials, not stale settings or legacy placeholders", () => {
  const other = "account:bigmodel-individual-coding-plan";
  const snapshot = { basedOnZCodeBuiltinRevision: "native-revision", providers: {
    [providerId]: { builtinModelIds: ["model"], access: { type: "zhipu-account", entitled: false } },
    [other]: { access: { type: "zhipu-account", entitled: true } },
  }, states: {} };
  const result = oauthEntitlements(snapshot, (id) => id === providerId ? { apiKey: "not-in-snapshot" } : null);
  assert.equal(result.providers[providerId].access.entitled, true);
  assert.equal(result.providers[other].access.entitled, false);
  assert.equal(result.basedOnZCodeBuiltinRevision, "native-revision");
  assert.equal(JSON.stringify(result).includes("not-in-snapshot"), false);
  assert.equal(snapshot.providers[providerId].access.entitled, false);
});
test("normalizes only model metadata and preserves reasoning and mode options", () => {
  const models = [{ value: `${providerId}\\GLM-5.3`, name: "GLM-5.3" }];
  const reasoning = { id: "thought", category: "thought_level", type: "select", currentValue: "max", options: ["max", "high", "low"] };
  const result = normalizeModelOptions([
    { id: "model", category: "model", type: "select", currentValue: "builtin:zai-coding-plan\\GLM-5.3" }, reasoning,
  ], models);
  assert.equal(result[0].currentValue, models[0].value);
  assert.equal(result[1], reasoning);
});
test("auth hook rereads credentials on every request, handles respawns, and is installed once per backend", async () => {
  const replies = [];
  const calls = [];
  const makeBackend = () => ({
    request: async (...args) => { calls.push(args); return { result: {} }; },
    sendReply: (id, result) => replies.push({ id, result }),
  });
  class Server {
    backend = makeBackend();
    async ensureBackend() { return this.backend; }
    async initialize() { return { agentInfo: { version: "fixture" }, authMethods: [], agentCapabilities: { loadSession: true } }; }
  }
  let key = "first";
  installOAuthBridge(Server, (id) => id === providerId ? { apiKey: key } : null);
  const server = new Server();
  const backend = await server.ensureBackend();
  const request = backend.request;
  await server.ensureBackend();
  assert.equal(backend.request, request);
  backend.providerRuntimeHeadersResponder(1, { providerId, accountAccess: { mode: "individual-coding-plan" } });
  await nextTick();
  key = "second";
  backend.providerRuntimeHeadersResponder(2, { providerId });
  await nextTick();
  assert.equal(replies[0].result.requestAuth.apiKey, "first");
  assert.equal(replies[1].result.requestAuth.apiKey, "second");
  await backend.request(3, "workspace/updateProviderRegistry", { providers: [{ apiKey: "legacy-key" }] });
  assert.equal(calls.length, 0);
  await backend.request(4, "session/send", { sessionId: "fixture" });
  assert.equal(calls[0][1], "session/send");
  server.backend = makeBackend();
  const replacement = await server.ensureBackend();
  assert.equal(typeof replacement.providerRuntimeHeadersResponder, "function");
  const init = await server.initialize();
  assert.equal(init.authMethods[0].name, "ZCode OAuth subscription");
  assert.equal(init.agentCapabilities.loadSession, true);
});
test("mismatched providers and missing credentials decline safely", async () => {
  const replies = [];
  class Server {
    async ensureBackend() { return this.backend; }
    async initialize() { return {}; }
    backend = { request: async () => ({}), sendReply: (id, result) => replies.push(result) };
  }
  installOAuthBridge(Server, () => { throw new Error("secret must not be shown"); });
  const backend = await new Server().ensureBackend();
  backend.providerRuntimeHeadersResponder(1, { providerId });
  backend.providerRuntimeHeadersResponder(2, { providerId, modelSelection: { providerId: "different" } });
  await nextTick();
  assert.deepEqual(replies, [
    { headersApplied: false, errorMessage: LOGIN_REQUIRED },
    { headersApplied: false, errorMessage: LOGIN_REQUIRED },
  ]);
});
