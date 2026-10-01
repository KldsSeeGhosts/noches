# Noches mobile companion

The iOS app in `apps/ios` doubles as a Noches companion. Pair it with a Mac or
Linux host over Tailscale and drive that host's sessions from the phone: no
account, no edge deployment, and no engine on the phone. Agents, files, and
transcripts stay on the host, and sessions keep running there while the app is
closed.

The gateway and its transport are in
[Tailnet connections](tailnet-connections.md).
[apps/ios/README.md](../apps/ios/README.md) covers the cloud path: sign-in,
Loro doc rooms, and attachments.

## Host setup

Run Noches on the host first. `zeron daemon install` installs `zeron headless`
as a launchd agent on macOS or a systemd user service on Linux;
`zeron daemon start|stop|restart|status` manages it, and `zeron status` reports
the workspace mode. An open desktop window embeds the same engine, so a
desktop window is also a host.

Provider credentials belong on that host, never on the phone. On macOS and
Linux, every agent launch (including model discovery, titles, and login) fills
missing environment variables from a cached login-shell snapshot, so a
shell-exported `CPA_API_KEY` is available even when Noches starts from Finder or
launchd/systemd. Explicit host or child environment values and removals take
precedence. The snapshot is kept in host memory only, not logged, sent to the
companion, or copied into service files. Restart the host after changing shell
credentials. `ZERON_NO_LOGIN_SHELL=1` disables this fallback; in that case,
configure credentials directly in the host service environment.

The engine serves its WebSocket RPC on loopback only, at
`ws://127.0.0.1:PORT`. `PORT` is whatever `ZERON_IPC_PORT` held when the engine
started, `27654` by default. Since `zeron daemon install` captures that variable
into the service definition, a given host may use another port. Confirm the
listener with `lsof -nP -iTCP:PORT -sTCP:LISTEN` on macOS or
`ss -ltnp | grep PORT` on Linux.

Then join the host to the tailnet. `tailscale ip -4` prints the address the
phone will dial, a `100.x.y.z` address. On a desktop host, skip the rest of this
section and use Settings > Connections ([Pairing from the app](#pairing-from-the-app)).
Otherwise create the pair code and serve:

```sh
cargo build -p zeron-rpc --bin noches-connect
CONNECT=./target/debug/noches-connect
TAILNET_IP=100.x.y.z                                  # tailscale ip -4 on the host
ENGINE_WS="ws://127.0.0.1:${ZERON_IPC_PORT:-27654}"   # this host engine's IPC port
STORE="${XDG_DATA_HOME:-$HOME/.local/share}/noches-connections"
mkdir -p "$STORE"

"$CONNECT" pair --name "Linux studio" --endpoint "ws://$TAILNET_IP:27657" \
  --upstream "$ENGINE_WS" --credentials "$STORE/access.json" \
  --out "$STORE/phone.code"
"$CONNECT" serve --bind "$TAILNET_IP:27657" --upstream "$ENGINE_WS" \
  --credentials "$STORE/access.json"
```

`pair` mints a 64-character hexadecimal key, registers the client in the
credentials file, and writes the code to `--out`. Neither write prints the key
to the terminal, and both files land with mode `0600` through atomic writes.
Keep the code file private, since it grants the phone the same session control
you have, and delete it once the phone has it.

`serve` refuses to start until at least one client is paired, with "Pair a
client before enabling remote access". The gateway ignores requests that carry
a browser `Origin` header, requires the key as a `Bearer` token, and checks the
upstream engine's `deviceId` before it forwards traffic, so a code stops
working if a different engine takes over that address. Management commands:

```sh
"$CONNECT" revoke --credentials "$STORE/access.json" --id CLIENT_ID
"$CONNECT" probe --code-file "$STORE/phone.code"   # host, identity, scope, and row counts
```

`revoke` removes one client's key, and that client's sessions close within five
seconds. `probe` prints the host name, the identity check, the workspace scope,
and watch row counts for a code file.

## Pairing from the app

Settings > Connections in a local desktop window carries a **Pair a phone**
section, so the CLI is not required for a normal pairing. It mints the same
`noches-connect:` code, shows it as a QR (error-correction level M, black on
white with a 4-module quiet zone) next to the endpoint, and offers **Copy code**
for a phone that cannot scan. The code lives in that page's memory only: it is
never logged and never written to disk, and **Done** clears it. The short
instruction on the page is "On your phone, open Noches, tap Pair a computer, and
scan this code. The phone must be on the same tailnet."

The button does everything `pair` plus `serve` did:

- reads this computer's Tailscale address from its interfaces; if Tailscale is
  not connected the page says so and stops, since there is no address to pair
  against;
- names the client from the macOS `ComputerName` (elsewhere the hostname without
  `.local`), and pins the engine's `deviceId` through `EngineInfo` on
  `ws://127.0.0.1:{ipc port}`;
- appends the client to `{data_dir}/remote-access.json`, mode 0600 through the
  same atomic `private_write` the CLI uses, and
- starts the gateway in-process on `{tailnet ip}:27657`, serving that same
  upstream.

The gateway is one per process, shared by every window, and starts automatically
on launch when `remote-access.json` already has a client and the computer is on
the tailnet. If another `noches-connect serve` already owns port 27657 the page
says exactly that instead of failing silently. **Paired devices** lists each
client with its short id and endpoint; **Revoke** removes one, and the gateway
stops when the last client is revoked.

The CLI below stays the right tool for a headless host, a systemd service, or
scripting, and it keeps its own `access.json` so the two stores never collide.

## Pairing the phone

Tap **Pair a computer** on the signed-out companion home, or pair from the
toolbar sheet that also holds appearance settings. Install Tailscale on the
phone, join the same tailnet as the host, and leave its VPN enabled. The
connection works across networks without exposing the engine to the public
internet. Tailnet access rules must allow the phone to reach the gateway port.

Paste the code or tap **Scan connection code**:
`noches-connect:` followed by URL-safe base64 of the profile JSON, which holds
the `id`, `name`, `endpoint`, `token`, and `deviceId`.

The key is stored in the phone's Keychain, under the `noches.companion`
service, accessible only while the device is unlocked, and never in
preferences or logs. The phone accepts a plain `ws://` endpoint only for
private addresses: Tailscale CGNAT `100.64.0.0/10`, the Tailscale ULA prefix
`fd7a:115c:a1e0::/48`, or loopback. Anything else needs `wss://`, and an
endpoint must not carry a path, credentials, or a query. Several computers can
be paired and switched in the same sheet. **Forget** removes one locally;
`revoke` on the host disables its key everywhere.

On macOS, generate the QR locally from the private code file:

```sh
swift scripts/pairing-qr.swift "$STORE/phone.code" "$STORE/phone.png"
open "$STORE/phone.png"
```

The renderer writes a PNG with mode `0600`. The image contains the same key as
the code, so keep it private and delete both after pairing. No QR service
receives it. Camera scanning fills the pairing field; tap **Connect to
computer** to validate and save it. Pasting remains available when the camera
is unavailable. A simulator cannot validate physical camera scanning.

## What the phone supports today

Transport is one WebSocket per computer, using the gateway's version-1 framing:
hello and welcome, sequence-numbered data frames with an `end` flag,
acknowledgements, ping and pong, and reset. Replies arrive as 32 KiB chunks
that the phone reassembles into one UTF-8 payload, capped at 32 MiB, and the
phone marks its own frames `end: true`; a data frame without `end` fails as an
incompatible protocol. A reset frame means the host restarted or reset the
connection, and the app refreshes session state.

Reconnects send a fresh hello with `resume: false`, which the gateway treats as
an explicit new start: it retires the retained session for that slot and dials
a new upstream session. Reads resubscribe and nothing is replayed. A mutation
whose delivery was not confirmed closes the connection with a "check the
session before resending" error instead of being retried. Deadlines are 12
seconds to connect and 20 seconds for a call or an initial snapshot, with a
ping every 8 seconds and a close after 24 seconds without a frame. A dropped
link retries every 4 seconds while foregrounded; backgrounding closes it and
the app reconnects on return.

The home screen follows the desktop control plane (see
[docs/design/mobile.md](design/mobile.md)). Sessions are grouped into **Needs
you** (awaiting input or failed), **Running**, and **Recent**. Cards show:

- the project badge, which is the repository favicon read over
  `ReadWorkspaceFile` or `ReadWorkspaceImage`, or a colored monogram;
- the status, with elapsed time while the session is working;
- the title;
- the branch;
- the harness mark.

Status follows desktop `display_status`. Working and awaiting-input rows older
than 45 seconds count as stale. A finished session that you have not opened is
Completed (emerald) until any device opens it; opening it on the phone sends
`Mutate {op: "markChatSeen"}`. Search matches titles, previews, branches,
paths, and project names. The bottom filter switches between All, Needs you,
Running, and Archived.

Sessions are scoped to the selected computer:

- live catalogs from `WatchChats`, `WatchSpaces`, `WatchSessions`, and
  `ListHarnesses`, with installed and enabled agents only;
- create via `Mutate {op: "createChat"}` against a project or the home folder,
  with the chosen installed agent, sandbox `workspace-write`, and a model and
  reasoning level from `ListModels`. The agent default remains available if
  its catalog cannot load;
- send: `QueueMessage` with `holdForTurnEnd` while a session is working or
  awaiting input, otherwise a `run` command carrying the prompt, working
  directory, sandbox, `autoApprove: false`, and the session's harness, model,
  and reasoning options. Queued rows render above the composer with host-confirmed
  edit, reorder, delivery, and removal actions;
- rename, archive, and restore through host-confirmed `Mutate` operations.
  Long-press a session for actions, use its chat menu, or swipe to archive.
  Restore from the Archived filter;
- unsent drafts stay scoped to the host and session during navigation and
  computer switches. Drafts are in memory and do not survive app termination;
- stop: `interrupt`;
- approvals: `respondInput` with the question panel's `{questionId, labels}`
  answers.

Transcript: tool rows use the desktop labels and tool-family tints. Thinking
shows as a collapsed row, and generated images load through
`ReadAttachmentChunk` and open full screen. `WatchDocMessages` frames apply incrementally as reset, upsert,
append, or remove changes, with byte-length and row-count checks. Text parts
use the same native table, incremental markdown parser, folding user bubbles,
and expandable tool groups as the cloud app. Scrolling away from the latest
message stops automatic following; the jump button returns to it. Input
requests use the native question panel. A frame that fails verification
surfaces "Transcript needs a refresh." before the view resubscribes. The host
writes every entry; the phone never edits the transcript.

The session's **Workspace** menu opens Changes, Files, Terminal, and the
project's actions:

- **Changes** follows `WatchCheckoutDiffs` (seeded by `GetCheckoutDiff`) and
  lists files with status letters, `+N −M`, branch, and ahead/behind from
  `ListGitHistory`. A file opens a unified diff with both line gutters, sliced
  from the fetched patch or, when that patch was truncated, rebuilt from
  `GetCheckoutFileDiffText`. **Discard all changes** calls `DiscardWorkingTree`
  with the diff checksum behind a destructive confirmation. The header's
  `+N −M` opens the same view.
- **Files** browses with `ListWorkspaceDirectory`, searches with
  `SearchWorkspaceFiles`, and previews with syntax colors. Complete UTF-8 files
  with LF or CRLF endings can be edited; Save sends `WriteWorkspaceFile` with
  the read's checkout id and content hash, and a `conflict` reply is surfaced
  instead of overwriting. Previews stop at 200,000 characters and binary files
  must be opened on the host. The host enforces the workspace file boundaries.
- **Terminal** opens a shell in the session's folder with `OpenTerminal`,
  streams `SubscribeTerminal`, and writes base64 input with `WriteTerminal`.
  The view handles `\r`, backspace, erase-line, clear, and 16-color SGR; full
  screen TUIs are not rendered. A key row offers esc, tab, ctrl-c, ctrl-d, and
  arrows. Dismissing the sheet detaches; the toolbar close ends the shell.
- **Run: {action}** entries come from `ListProjectActions` and start through
  `RunProjectAction`, with output in the Terminal view.

Projects: the new-session sheet and the Projects drawer offer **Add
project…**, which browses the host with `ListFolders` (Home plus `ListDrives`
roots, a typed path, and a git badge on repositories) and creates the project
with `Mutate {op: "createSpace"}` on the paired host's device id. Project
settings rename (`renameSpace`) or remove (`deleteSpace`, which deletes the
project's sessions on the host). For a git project, a new session can run on
the current branch or in a new worktree from `CreateWorktree` based on a
branch from `ListBranches`; the engine names the `zeron/…` branch.

Home groups sessions **By status** or **By project** (collapsible groups with a
per-project **+**). On iPad the list is the sidebar of a split view.

Composer: the access chip opens a native sheet to switch the run's `autoApprove`
between Ask and Auto-approve (stored per host and chat on the phone) and set the
sandbox through `setChatConfig`. `@` suggests files from `SearchFiles`; `/` at the
start of the draft suggests `ListCommands` entries, sent as prompt text.
Queued rows can move up, to the top, or down (`MoveQueuedMessage`), and edit
leases renew every 20 seconds.

The Projects drawer distinguishes **All projects** from **No project**. Queue
details are height-bounded and scrollable; opening the composer keyboard hides
agent details and folds queued rows so the transcript retains usable space.
Per-file diffs keep a **Done** action, and entering file editing focuses the editor.

### Overhaul acceptance coverage

`CompanionUITests` exercises pairing/send, project grouping and collapse, folder
browsing and project creation, worktree sessions, file search/editing, stale-hash
conflicts, discard cancellation/confirmation, terminal and project actions,
mention/command suggestions, queue ordering/editing/lease renewal, access/sandbox
dispatch, session management, draft retention, and light/dark themes.

The final iOS 27 simulator verification passed all 235 unit/hosted tests on
iPhone 18 Pro, including the actual Rust gateway and an isolated live host,
and all 19 companion/mobile-polish UI scenarios on iPhone 17e. Screenshots were
reviewed. A smaller-device hosted keyboard test initially received only the
hardware-keyboard accessory bar; its unchanged full-motion assertion passed on
the freshly booted second simulator. Physical camera scanning, actual provider
execution, and real-phone performance are not covered by this pass.

Not implemented today:

- the host engine and the gateway must already be running, so the phone cannot
  wake a sleeping host, start a stopped service, or reach a machine that is off
  the tailnet;
- no PR badges, clone, or new-repository flows;
- one terminal per session, without full-screen TUI rendering.

Simulator rigs can pair and open a session from launch arguments in Debug
builds: `-companion-pair <code>` and `-companion-open <chatId>`.
`scripts/companion-rig.sh` runs a seeded mock-harness engine behind a
loopback `noches-connect` and prints a code for these arguments.

## Theme and appearance

The companion uses the desktop's Geist and Geist Mono fonts, pixel mark,
project badges, agent marks, and theme color roles. The native list keeps project
labels and session metadata compact; search, filtering, and new-session
controls sit at the bottom of the screen. Chat uses a glass composer and the
shared native transcript renderer.

The companion uses the desktop theme catalog.
`apps/ios/Zeron/Theme/DesktopThemes.json` is generated from `crates/theme` and
carries 19 families: Zeron, VS Code Default, Catppuccin, Tokyo Night, Dracula,
GitHub, Ayu, Gruvbox, Rosé Pine, Nord, One Dark Pro, Atom One Dark, Night Owl,
Winter is Coming, Palenight, SynthWave '84, Shades of Purple, Cobalt2,
Andromeda, with their light and/or dark variants, several of them dark-only,
plus resolved color roles, syntax colors, and the seven accent presets:
Noches, orange, amber, green, cyan, blue, and pink. Regenerate it after a
theme change:

```sh
cargo run -q -p zeron-theme --example export-ios > apps/ios/Zeron/Theme/DesktopThemes.json
```

The Appearance screen selects the mode, a light theme, a dark theme, an accent
preset, and a surface treatment. The mode is system, light, or dark; the surface
treatment is theme default, frosted, or opaque. Frosted surfaces use
`ultraThinMaterial` and `glassEffect`, and fall back to opaque when Reduce
Transparency is on. Choices are device-local defaults under `appearance.*`.

Import a resolved theme family JSON through Appearance, Import desktop theme,
to add a theme. The file must be at most 16 MB and match the bundled shape:
non-empty id and name, at least one light or dark variant, every built-in color
role plus the seven accent roles, valid `#rrggbb` or `#rrggbbaa` colors, and no
id collisions with built-ins or installed themes. Imported families without
preset roles fall back to the desktop's accent arithmetic, byte-rounded mixes
with contrast correction. The test suite checks that this derivation reproduces
the exported desktop presets.

The phone's appearance is not synchronized from the host. Matching the host
means exporting the same resolved family on the host and importing it here.

## Versus the cloud path

The same app can also join the mesh. Sign in from Settings with Connect a cloud
account, and the phone becomes a peer device: workspace and session Loro docs
sync in both directions, and queue editing, attachments, and PR badges work
there. Direct pairing does less, one host at a time, with no account, no edge,
and all state on the host.

| | Cloud mesh | Companion |
| --- | --- | --- |
| Sign-in | WorkOS through the edge | None |
| Transport | Edge rooms plus the device relay | Tailscale to the host gateway |
| Scope | Every signed-in device | The paired computers only |
| Phone state | Loro mirrors of workspace and session docs | None; live RPC views |
| Offline host | Commands stay durable in the session doc and a reconnecting host drains them | Nothing is sent; the connection reports the error |
| Attachments, queue edits | Supported | Supported |
| PR badges | Supported | Not implemented |
| Theme | Bundled desktop catalog | Same catalog |

## Build, run, and test

Validation on September 22, 2026 includes iPhone 18 Pro and iPhone 17e
simulators, a real Rust gateway over a fixture engine, and an opt-in read-only
simulator connection to a running Linux Noches host over Tailscale. The live
check verifies host identity and decodes its session catalog. It does not
start an agent, and is not a physical-device or cellular-network test.

The final iPhone 18 Pro run passed 26 unit/integration tests and four UI
tests. The optional live-host test skipped after its private code was removed;
it passed in a separate run. All four UI tests also passed on iPhone 17e.
Debug and Release simulator builds passed. The QR image passed an exact
encode/decode round-trip; camera capture still needs a physical iPhone.

Screens from the fixture, which contains no private conversations:
[dark home](screenshots/mobile-companion/companion-connected-dark.png),
[light home](screenshots/mobile-companion/companion-zeron-light.png),
[transcript](screenshots/mobile-companion/companion-transcript-dark.png),
[approval](screenshots/mobile-companion/companion-approval.png),
[file preview](screenshots/mobile-companion/companion-file-preview.png), and
[changes](screenshots/mobile-companion/companion-changes.png).

Xcode 27 with the iOS 27 simulator runtime; the project's deployment target is
iOS 26.0. `iPhone 18 Pro` is the installed simulator destination on this
machine. From `apps/ios`:

```sh
xcodebuild -project Zeron.xcodeproj -scheme Zeron \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro' build

xcodebuild -project Zeron.xcodeproj -scheme Zeron \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro' test
```

The companion subset:

```sh
xcodebuild -project Zeron.xcodeproj -scheme Zeron \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro' \
  -parallel-testing-enabled NO -collect-test-diagnostics never \
  -only-testing:ZeronTests/CompanionDashboardTests \
  -only-testing:ZeronTests/CompanionConnectionTests \
  -only-testing:ZeronTests/CompanionTransportTests \
  -only-testing:ZeronTests/AppearanceSettingsTests \
  -only-testing:ZeronTests/AppearanceImportTests \
  -only-testing:ZeronUITests/CompanionUITests test
```

- `CompanionDashboardTests` covers attention ordering, host isolation,
  combined search/filtering, draft scoping, same-host reconnect, mutation
  guards, and transcript adaptation for tools and approvals.
- `CompanionConnectionTests` covers code round-trips, the accepted and rejected
  endpoint matrix, transcript frame application including UTF-8 byte lengths,
  and host catalog decoding. No fixture needed.
- `CompanionTransportTests` drives a fixture host: identity and catalog, a
  snapshot that outlives the 20-second deadline, wrong-key and changed-identity
  rejection, ambiguous mutation delivery that is never replayed, and the
  run/interrupt/respondInput wire shapes. Two cases cover reassembly of large
  replies: `testLargeUnicodeReplyReassemblesAcrossGatewayChunks` through the
  fake gateway on 28777, and `testRealRustGatewayChunksAndFreshReconnects`
  through the real gateway over three fresh connections. Each skips with
  `XCTSkip` when its fixture is not running.
- `AppearanceSettingsTests` covers the 19-family catalog, appearance and surface
  selection with persistence, accent derivation against the exported desktop
  roles, and import rejection for built-in ids. No fixture needed.
- `AppearanceImportTests` adds eight cases for the import path: install and
  persistence, derived accents for a family without presets, malformed preset
  cleanup, id collisions, repair of persisted customs, clearing an unreadable
  blob, repairing selections that no longer resolve, and replacing a family
  that a selection points into.
- `CompanionUITests` pairs, opens, sends, and creates sessions against the
  fixture. It covers files/diffs, rename/archive/restore, draft retention,
  approval responses, queued follow-ups, stopping, and appearance screens.
  The tests pass `-appearance.*` launch arguments, and
  `-companion-settings` opens the settings sheet on launch.
  Keep simulator code signing enabled for these UI tests. Pairing writes to
  Keychain, which fails in a build made with `CODE_SIGNING_ALLOWED=NO`.
  `CODE_SIGN_IDENTITY=-` uses an ad-hoc simulator signature without a paid
  developer account. Run with `-parallel-testing-enabled NO` because the
  companion flows share fixture state.

`testLivePairedHostWhenConfigured` is opt-in. Put an existing private pairing
code in the installed simulator app's `Documents/companion-live.code`, run
that test, then remove the file. It never saves the code to Keychain or writes
to the host. With no file it reports a skip.

The fixture host is a Node script with no real host behind it. It emulates the
gateway on `ws://127.0.0.1:28777` (`NOCHES_FIXTURE_PORT` overrides), answering
`EngineInfo`, model and agent catalogs, workspace reads, `BigReply`, and the watch methods, recording
commands for `/commands`, and splitting replies into 32 KiB chunks with `end`
flags. It closes the socket on a client frame without `end: true`, and serves a
fixed profile: id `mobile-fixture`, key `a` repeated 64 times, device
`fixture-mac`. Run `npm install` in `edge/` first if its `node_modules` is
missing, since the script loads `ws` from there. It touches no real files,
hosts, or agents, and prints its own `noches-connect:` code. `BigReply`
answers with 15,000 moon emoji, 60 KB of UTF-8, so it spans several chunks.

```sh
node scripts/fixtures/noches-mobile-host.mjs
```

`--engine` switches the same script to a raw ndjson RPC upstream on
`ws://127.0.0.1:28778`, the host a real gateway expects. To put the Rust
gateway in front of it, build the binary, start the raw fixture, and write a
dummy credentials file for the test profile:

```sh
cargo build -p zeron-rpc --bin noches-connect
node scripts/fixtures/noches-mobile-host.mjs --engine &
FIXTURE=$!
RIG="$(mktemp -d /tmp/noches-gateway.XXXXXX)"
python3 - "$RIG/credentials.json" <<'PY'
import json, sys
profile = {"id": "mobile-fixture", "name": "Studio fixture",
           "endpoint": "ws://127.0.0.1:28779", "token": "a" * 64,
           "deviceId": "fixture-mac"}
json.dump({"clients": [profile]}, open(sys.argv[1], "w"))
PY
./target/debug/noches-connect serve --bind 127.0.0.1:28779 \
  --upstream ws://127.0.0.1:28778 --credentials "$RIG/credentials.json" &
GATEWAY=$!
```

The gateway binds loopback 28779 and proxies to the raw fixture on 28778, so
`testRealRustGatewayChunksAndFreshReconnects` exercises the real chunking path
end to end. When the run is over:

```sh
kill "$GATEWAY" "$FIXTURE"
rm -rf "$RIG"
```

The key in this rig is the throwaway fixture key, and the credentials file
lives only in the temp directory.

## Code map

- `apps/ios/Zeron/Companion/ConnectionProfile.swift`: profile, code parsing
  and validation, Keychain storage.
- `apps/ios/Zeron/Companion/DirectConnection.swift`: WebSocket framing, chunk
  reassembly, deadlines, heartbeats, and the never-replay failure path.
- `apps/ios/Zeron/Companion/CompanionModel.swift`: catalogs, transcript
  frames, sessions, and commands.
- `apps/ios/Zeron/Companion/CompanionView.swift`: navigation, pairing,
  new-session sheet, session view, and composer.
- `apps/ios/Zeron/Companion/CompanionHome.swift`: project list, session rows,
  search/filtering, project badges, and session actions.
- `apps/ios/Zeron/Companion/CompanionSession.swift`, `CompanionComposer.swift`,
  `CompanionTrays.swift`: session header, composer, and agent/queue trays.
- `apps/ios/Zeron/Companion/CompanionNewSession.swift`: new-session sheet.
- `apps/ios/Zeron/Companion/CompanionModel+*.swift`: state derivation, seen
  markers, config, queue, uploads, and project icons.
- `apps/ios/Zeron/Companion/CompanionTranscript.swift`, `CompanionSubagents.swift`,
  `CompanionImages.swift`: host message adapter, subagents, and images.
- `apps/ios/Zeron/Theme/ControlPlaneStyle.swift`, `Views/ControlPlaneViews.swift`:
  status, tool-family, and monogram palettes and the shared glyphs.
- `apps/ios/Zeron/Companion/CompanionWorkspace.swift`: files, search, and edit.
- `apps/ios/Zeron/Companion/CompanionChanges.swift`, `CompanionChangesDiff.swift`: live diff panel and parser.
- `apps/ios/Zeron/Companion/CompanionTerminal.swift`: terminal and project actions output.
- `apps/ios/Zeron/Companion/CompanionFolderBrowser.swift`, `CompanionProjects.swift`, `CompanionProjectList.swift`: host folder browser, add/rename/remove project, projects drawer.
- `apps/ios/Zeron/Companion/CompanionMentions.swift`: `@` file and `/` command suggestions.
- `apps/ios/Zeron/Companion/CompanionScanner.swift`: camera QR scanning.
- `scripts/pairing-qr.swift`: private local QR rendering on macOS.
- `apps/ios/Zeron/Theme/AppearanceSettings.swift`: catalog, appearance and
  accent state, custom-family import.
- `crates/rpc/src/bin/noches-connect.rs`: gateway `serve`, `pair`, `revoke`,
  `import`, and `probe`.
- `crates/rpc/src/remote/host.rs`: host-side pairing, revocation, and tailnet
  address discovery, shared by the CLI and the desktop page.
- `crates/ui/src/remote_access.rs`: the in-app gateway, its status, and the QR
  rendering; `crates/ui/src/settings/connections.rs` is the page around it.
