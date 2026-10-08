# ZCode OAuth local ACP

Use ZCode's native app-server, tools and subscription login in T3 Code's
**Local ACP** option. No T3 fork and no manually configured API key.

Requires the ZCode desktop bundle, Node with `node:sqlite`, and an existing
ZCode CLI OAuth login. Desktop login alone may not create the CLI's standalone
account credential pair.

Install this directory outside the repository, for example at
`~/.local/share/noches/zcode-oauth-acp`, then run:

```sh
npm ci --ignore-scripts --prefix ~/.local/share/noches/zcode-oauth-acp
chmod +x ~/.local/share/noches/zcode-oauth-acp/launch.sh
```

In T3 provider settings, edit the existing ZCode local ACP provider and set its
command to the absolute path of `launch.sh`. Leave arguments and API-key
environment variables empty. Existing GLM Agent providers are unaffected.

If the launcher reports missing credentials, run
`~/.local/share/noches/zcode-oauth-acp/launch.sh login` in your terminal, then
reconnect. This explicitly invokes the native CLI's browser OAuth flow with
its bundled provider configuration. The ACP server never opens a login flow
automatically. A working existing CLI login needs no new sign-in.

The extension pins `zcode-acp-server` to 0.65.1 and does not alter global npm
packages or T3's live database. It reuses the bridge's protocol, streaming,
tools, MCP, permissions, session resume, fork, reasoning and cancellation.
Its model list comes from ZCode's installed native provider table, filtered
by the CLI's saved account credentials.

ZCode OAuth itself acquires an internal subscription request credential.
This launcher reads the same encrypted credential pair as CLI 0.16.9 and
passes it only to the native app-server in memory. It does not extract
credentials to files, log them, or write authentication state. The store is
read again for each request so a subsequent login is picked up.

Only individual Z.AI and BigModel Coding Plan accounts are supported.
Start Plan captcha authentication, team plans and third-party API providers
are not implemented. Local ACP also retains T3's local-provider presentation;
it does not become a built-in T3 provider.

Run the focused tests with `node --test oauth.test.mjs`. For an explicit live
subscription test, run `node smoke.mjs --without-legacy-config`. It validates
model switching, a read-only native tool, session replay and cancellation in
a temporary workspace, with no legacy config or settings-derived entitlement.
It follows temporary links to the existing encrypted login and native provider
config, removes those links afterwards, and retains the tiny workspace fixture.
Other ZCode CLI/bridge versions require compatibility verification before
changing the pin.
