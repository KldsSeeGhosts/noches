# Managed computer use

Noches owns computer use for Pi sessions on Linux. A Pi run loads `crates/harness/src/pi/noches-cua.ts` through a private `pi` wrapper (`PI_ACP_PI_COMMAND`) so the community `pi-acp` adapter still launches Pi. The extension exposes `noches_cua`. The engine authorizes requests and launches `cua-driver`. The driver draws the session-colored agent cursor and performs desktop inspection and input. Cua's synthetic seat stays off the physical Wayland seat because `vendor/gpui_linux` is part of every Noches build.

The driver patches in `scripts/cua/` are applied to a reviewed Cua source checkout. They are not compiled into the Noches package. Build and install from a checkout that includes `examples/linux-host/install.mjs`:

```bash
bash scripts/cua/build_native.sh --cua /path/to/cua --install
```

`--install` keeps a content-addressed native binary, installs the shared CLI launcher, and updates `~/.config/cua-driver/host.env`. It preserves the exact package compatibility pins and backs up replaced host files. It never replaces the launcher with a raw-binary symlink. Restart an idle Noches engine afterward. The script does not rebuild Noches or replace the loaded compositor plugin. A plugin update needs a matching Hyprland ABI and a fresh compositor session. The first inspection or control call through `noches_cua` asks for approval inside the app.

```text
Pi tool adapter -> private Unix socket -> Noches engine -> cua-driver mcp
                                                              | (proxied)
                                                  cua-driver serve (per-run daemon)
```

The adapter is bundled at `crates/harness/src/pi/noches-cua.ts`. The engine
implementation is `crates/engine/src/computer_use/linux.rs`, with host
configuration and ownership in `crates/engine/src/computer_use/host.rs`.

## Permissions and ownership

- The first app inspection or control request asks through Noches' native
  question UI. One approval covers the engine host for the current session:
  inspection, screenshots, clipboard reads and supported background input.
  Physical focus, mouse and keyboard must remain untouched. Foreground
  delivery, desktop input, clipboard writes, app launch and automatic browser
  setup remain forbidden after approval. Unsupported background actions
  refuse rather than escalating. A denial lasts until the turn ends; a fresh
  turn may ask again.
- One chat at a time can hold the desktop lease. It covers the
  whole active turn, including pauses between tool calls, rather than just
  individual clicks. Other chats receive a busy error. A private advisory
  lock at `/run/user/<uid>/cua-driver/control.lock` also coordinates separate
  Noches engines, standalone Pi and the shared CLI launcher. The file stays
  in place; process lifetime, not file deletion, releases ownership.
- Turn completion closes the driver session, reaps the driver and releases
  the lease. A parked Pi process keeps its bridge socket and its approval,
  but no desktop lease or driver. The next turn re-acquires both without
  asking again.
- On a real host the engine runs `cua-driver serve` behind the MCP child for
  the length of the turn. Metadata asked before approval starts a daemon
  without `--grant existing-profile`; that daemon never has the grant, and
  it is replaced by a granted daemon once approval exists. The daemon owns
  the agent cursor overlay runloop, so the synthetic
  cursor renders on screen instead of only updating in memory. Both
  processes are reaped before the lease is released.
- Stop, a disconnected tool caller, transport failure or timeout cancels
  outstanding work. The Linux driver process is killed and reaped before
  its lease is released. Already delivered input cannot be undone. Unknown
  outcomes are never replayed automatically.
- The engine stamps its own session label. Session lifecycle tools,
  configuration changes, recordings and unreviewed actions are not exposed.
  `help` and `describe` use MCP `tools/list`, not nonexistent driver tools.
- `help` and `health_report` expose the MCP handshake contract the engine
  retained: negotiated protocol version, server identity, capabilities, and
  the exact executable path and SHA-256 actually spawned. Optional server
  instructions are retained as a digest only and never reach a model prompt.

Driver authorization stays in `standard` mode. A daemon started after
approval carries the narrow `--grant existing-profile`, which only admits
attaching DevTools to an existing logged-in Chromium-family profile after
the user approved computer use for the session; a pre-approval metadata
daemon never carries the grant and is replaced by a granted one when
approval arrives. Inherited `CUA_*` environment settings are removed before
launching the child, then standard mode, that grant, Wayland support, and the
host's exact `CUA_HYPRLAND_LOCAL_PACKAGES` compatibility pins are set explicitly.
The removed `CUA_HYPRLAND_OPEN_INPUT` switch is not used. The test-only
Hyprland input protocol remains disabled. Driver-level permission refusals
remain errors; a Noches grant does not bypass them.

The socket directory is private, mode 0700, and the socket is mode 0600.
Each run gets a distinct connection and identity. No computer-use endpoint
is exposed through the remote UI RPC API. A remote viewer's approval applies
to the named engine host, not the viewer's own desktop.

## Pi integration

The adapter disables the direct `cua` tool in Noches-managed Pi processes
and blocks calls to it if another extension reactivates it. Other extensions
and model providers remain enabled. Global Pi configuration is unchanged,
so standalone Pi keeps its separate CUA extension. That extension uses the
same host settings and advisory lock, starts a private Linux MCP runtime,
and disconnects at turn end, session switch or shutdown. It retains the
handshake, caches live schemas, and reports structured refusals as errors.
Cancellation ends its private process group and never replays uncertain input.
Standalone Pi is not governed by Noches' action allowlist. It must still
respect driver permissions and obtain authorization for foreground control.

The adapter is loaded even when bridge startup fails. In that case managed
calls fail explicitly; the legacy tool does not silently take over. Failure
to install the adapter prevents that Pi process from starting.

Full MCP text, images and structured results reach the adapter. Structured
results are both retained in tool details and included in model-readable
text, so accessibility tokens and delivery facts survive. Combined text is
capped at 50KB or 2000 lines. A truncated result is exported as two private
files: `result.txt` holds the human-readable text and `result.json` holds a
valid JSON document with the action and full structured content. The
directory is mode 0700 and both files are mode 0600, and the tool result
carries both paths. These local result files remain available until removed
or cleaned by the operating system. Treat them as sensitive app content.

The adapter normalizes the driver's structured outcome model. A structured
refusal (`status: refused`, `effect: refused`, or `refused: true`) becomes a
failed Pi tool execution while the full refusal payload, reason code,
permitted next action and approval metadata stay in the result. The audited
driver nests those fields under `refusal: {code, message, detail: {reason,
next_action, supported_strategies}}`; the adapter reads that layout and the
older flat one. A refusal classification outranks the generic `isError` flag,
while partial, unknown and unverifiable deliveries stay non-errors only when
the driver did not set `isError`. Uncertain deliveries report uncertain
delivery and are never replayed automatically.

## Installation and limits

Install a compatible `cua-driver` on the engine host. Noches resolves an
explicit `CUA_DRIVER_PATH`, then the literal settings in
`${XDG_CONFIG_HOME:-~/.config}/cua-driver/host.env`, then PATH, then
`~/.local/bin/cua-driver`. The selected path must resolve to a native ELF
executable, not a shell launcher. The launcher and native path are deliberately
different so `/proc/<pid>/exe` attestation hashes the actual running driver.
An invalid explicit path fails rather than choosing another executable.
Noches does not download or update the driver automatically.

The host settings accept only two keys, without quoting or shell expansion:

```text
CUA_DRIVER_PATH=/absolute/path/to/native/cua-driver
CUA_HYPRLAND_LOCAL_PACKAGES=package-name=exact-version,another-package=exact-version
```

Only list packages supported by the reviewed native driver. These pins are
compatibility checks, not permission grants. After a package upgrade,
`cua-driver host-status` reports mismatches. Validate the app's native route
before accepting the new version; the installer does not auto-admit it.
Noches and Pi recover current display connection variables from the user
systemd manager, with a two-second bound, so a headless engine can precede
graphical login or survive relogin. Run as the desktop user.

Useful checks are `cua-driver host-status`, `cua-driver doctor`, and Pi's
`/cua-status`. The CLI launcher requires Node, systemd and util-linux `flock`.
Restart Pi to load extension code changed on disk.

This first integration supports Linux and Pi. Other agent backends do not
yet connect to this service. macOS needs an app-owned driver lifecycle and
permission implementation before it can be enabled here.

This is advisory application-level coordination, not a sandbox. Agents with shell
access and trusted extensions running as the same desktop user retain
those capabilities. Direct calls to the native binary, older engines,
unrelated tools and direct daemon clients can bypass the lock. Dev and
production remain separate installations but target the same desktop.

Wayland has no general raw-background-input protocol for arbitrary apps.
AT-SPI actions cover accessible controls; the Hyprland input-v3 plugin adds
qualified native routes with package, surface, input and keymap checks.
Canvas-heavy apps, arbitrary Electron apps, Unicode/IME and unsupported
gestures are not covered by the current raw route. A GTK fixture passing
does not certify Zen, LibreOffice or every operation in an eligible package.
See [the local validation record](computer-use-validation-2026-10-02.md).

## Development verification

Use the dev channel when building a local dev binary. Bundled adapter and
context-extension files honor `ZERON_DATA_DIR`; the installed dev engine
uses `~/.noches-dev`. Do not point it at production data.

```sh
cargo +stable test --locked -p zeron-engine --lib computer_use
node --experimental-vm-modules --test crates/harness/tests/noches-cua.test.mjs
NOCHES_CHANNEL=dev NOCHES_VERSION=0.1.62-dev.1.cua.20261002 \
  cargo +stable build --locked --release -p zeron
# Opt-in metadata-only smoke against the real installed driver:
cargo +stable test --locked -p zeron-engine --lib installed_driver_metadata_smoke -- --ignored --nocapture
```

The adapter tests require Node 22 with `stripTypeScriptTypes` support. Engine
tests use injected Python MCP fixtures, never a real driver or process-global
environment mutation. They check full results, permissions, lease release
between turns, interruption during blocked input, client disconnection,
frame limits and partial-client teardown.

Install the candidate into a separate version directory, retain the previous
version, and switch the dev install's `app/current` symlink before restarting
the idle `noches-dev.service`. Do not overwrite a running executable.

For a manual smoke test, open a new Pi chat in Noches dev and ask it to use
`noches_cua` for `help`, `describe` and `health_report`. Then ask it to inspect
only the dev window and move its synthetic cursor. Approve the native prompt
only for that test. Verify the cursor disappears when the turn ends and
that another chat can request control. Do not approve production-window
input as part of a dev smoke test.
