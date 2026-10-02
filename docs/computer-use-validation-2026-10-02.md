# Linux computer-use integration, 2026-10-02

## Current Linux installation, operation-scoped follow-up

The user approved a reversible Linux-only installation while full remote
certification remains pending. Noches Dev now selects
`0.1.78-dev.1.cua.20261002.2`, built from runtime commit `17bf29d0`.
Its base includes the exact `0b82bf6d` source used for the previously installed
`0.1.78-dev.1` release. The browser bundle and icon were checked unchanged.
The old package and all user data were retained. Running engine and UI
processes were not restarted and still execute `0.1.78-dev.1`.

Standalone Pi and the Linux launcher were installed from CUA
`585283f5ec8b3f89e398f32ba5849c3598a3190f`. The native driver bytes did not
change; the installer selected its content-addressed copy under
`~/.local/lib/cua-driver/sha256-c5a1730ee425d10079a91b18fcfa28d3f7cf18effc0b9f9164a19a65e347c59c/`.
Compatibility pins, services, the compositor plugin, permissions and stable
Noches were not changed. No work ran on the user's Mac Studio.

The Noches rerun caught an intermittent lock-release race. A child forked by
another thread could retain the open-file description until exec, extending
a lock after the parent's file closed. `OperationLease` now explicitly
unlocks before closing. A deterministic fork regression checks both the
desktop gate and window lock while the child still holds inherited descriptors.
Partial acquisition failures also release through that guard.

Verification for this rollout:

- 35 tests against the installed Pi extension passed; 2 host installer tests passed.
- 37 Noches computer-use tests passed in five consecutive runs. The opt-in
  installed-driver metadata smoke also passed separately.
- 17 managed Pi tests and 11 release tests passed. The optimized Noches build
  and installed version/hash checks passed.
- Pi's actual extension loader registered `cua` without errors. Two concurrent
  installed-adapter clients, one through the launcher, each listed 60 tools and
  read live window metadata without claiming input ownership.
- Driver `doctor` reached the accessibility bus and Wayland display. Its X11
  probe warned that no top-level X11 windows were returned.

An older Pi process still owned the legacy exclusive lock throughout the
metadata check. That process was preserved. Fresh live input testing was
deferred, not bypassed. Real kernel-lock and fixture-process tests cover
independent windows, conflicting operations and cancellation; these do not
replace native application input qualification.

The metadata smoke previously asserted that the entire desktop was idle after
its turn. That fails legitimately when an unrelated client owns the gate.
It now checks that its own bridge holds no operation lease during metadata
and after cleanup. The native executable and response checks remain intact.

Activation requires finishing existing jobs, running `/cua-disconnect` in
the old Pi session and restarting Pi. For Noches Dev, close the idle UI,
restart `noches-dev.service`, then reopen the UI. No running session was
terminated by this rollout.

Backups, installation hashes, read-only verification evidence and a checked
rollback helper are in
`~/.local/state/cua-operation-rollout-20261002-SOIRc8/`.
`node rollback.mjs` checks rollback safety; `node rollback.mjs --apply`
restores installation files without restarting sessions. It refuses if the
checked installation changed afterward. The host installer's original files
are retained in `~/.local/state/cua-driver/install/backup-X9aWI7/`.
Published Noches updates can replace this local build until the source lands
in the release branch. No full Mac/Windows certification or dev merge is
claimed by this local deployment.

## Earlier installation record

The initial Noches dev engine ran `0.1.62-dev.1.cua.20261002` from
`~/.local/share/noches-dev/app/0.1.62-dev.1.cua.20261002/zeron`.
`noches-dev.service` and its IPC listener passed the post-install check.
Production Noches, existing user data and the loaded Hyprland plugin were
not replaced. The previous dev package remains installed.

The native driver is
`~/.local/lib/cua-driver/local-20261002/cua-driver`, with SHA-256
`c5a1730ee425d10079a91b18fcfa28d3f7cf18effc0b9f9164a19a65e347c59c`.
It reports `0.28.2` and includes local patches. It is not stock `0.32.0`.
The shared CLI launcher, host settings, CUA systemd environment and installed
Pi extension were updated. Start a new Pi process after finishing an active
turn so it loads the final extension files.

`~/.config/cua-driver/host.env` selects that native executable and pins
`zen-browser-bin=1.22.3b-1,libreoffice-fresh=26.8.0-2.1`. Both match installed
packages. The stale Zen pin was `1.22.2b-1`; the obsolete
`CUA_HYPRLAND_OPEN_INPUT` setting did not select the installed driver's route.
The duplicate package pin in Hyprland's `env.lua` was removed.

## Upstream review

[Cua Driver 0.32.0](https://github.com/trycua/cua/releases/tag/cua-driver-rs-v0.32.0)
shipped October 1. Its component-tagged assets and the canonical installer's
baked version were checked, rather than the monorepo's Latest badge.
The release includes newer Hyprland geometry, browser/keyboard cursor and
snapshot work. Installing it wholesale would discard this host's local
Zen and Noches repairs.

The isolated checkout is `~/AiStack/cua-background`, branch
`fix/background-integration-20261002`, based on local repair commit
`983e7e817`. Upstream cursor hotspot fix
`b0ebd915daf85beeb8e3301a2ae83df8d4499752` was cherry-picked with attribution
as `83c472618`, retaining local cursor visibility/tracking logic.
Pi and Linux host integration changes follow that backport on the
[same branch](https://github.com/KldsSeeGhosts/cua/tree/fix/background-integration-20261002).
The original dirty `~/AiStack/cua` checkout was left alone.

## Verification

| Check | Result |
| --- | --- |
| Cursor-overlay unit tests | 48 passed |
| Linux platform unit tests | 528 passed, 5 ignored |
| Noches computer-use tests | 33 passed, 1 opt-in test ignored |
| Installed-driver metadata smoke | 1 passed separately |
| Noches Pi adapter | 17 passed |
| Standalone Pi transport/results | 26 passed |
| Host installer, including staged native install and backups | 2 passed |
| Noches release build and deployed engine/IPC | Passed |
| Actual Pi resource loader | `cua` registered with no loading errors |
| Real MCP ownership | Second client refused; reacquisition after disconnect passed |
| GTK background fixture | Text and button state verified; physical focus/cursor unchanged during input |

The GTK before/after images and Linux test log are retained under
`~/.local/state/cua-background-20261002/`. The fixture was closed and its
absence verified. The final extra live-client check met an active user Pi
session and correctly returned busy. That session was not interrupted.
The final explicit-private-runtime and catalog-race changes passed unit
tests; another desktop smoke was deferred while the user owned the lease.

`git diff --check` and launcher/build-script syntax checks passed. Strict
Clippy is not clean: Rust 1.98 reports existing warnings in unrelated
`zeron-harness` code. Those files were not changed. Builds used the installed
rustup stable toolchain because the distro Rust compiler had an LLVM symbol
error and the CUA-pinned toolchain download failed.

## Remaining coverage limits

### Follow-up ownership correction

The initial installed integration above held ownership for an entire agent
turn. User feedback showed that a CUA call followed by coding could block
unrelated agents. The follow-up source changes replace that reservation with
per-operation ownership in Noches and standalone Pi. Reviewed explicit
background actions coordinate by window; global or unreviewed operations
remain exclusive. Help and observations do not reserve input. Connections and
snapshot tokens survive successful calls. Split held-button gestures in Pi
must use atomic `drag` instead.

The updated Noches suite passed 36 tests with one opt-in test ignored. New
tests cover two concurrent bridges, same-window refusal, independent windows,
inspection during input, and release after cancellation. Pi includes a real
private-process test that ignores SIGTERM and proves ownership is retained
until forced exit. These are coordination tests, not a new GUI/app qualification.
The earlier installation predates this correction. The current rollout above
installs it for future launches; updating files does not update already
running Pi processes or the Noches service.

This is not blanket macOS parity. The installed Hyprland input-v3 plugin
matches the current compositor ABI, but arbitrary occluded Wayland surfaces
still have no portable raw-input route. The GTK test proves AT-SPI delivery.
It does not prove native Zen raw input on the newly pinned version, every
LibreOffice operation, Electron canvases, Unicode/IME, or arbitrary modified
gestures. Those require separate app-specific native tests. No foreground
fallback or unrestricted approval mode was enabled to hide a refusal.

The shared lease is advisory. Updated Noches engines, Pi and the launcher
participate; direct native/daemon clients and older engines can bypass it.
Future published Noches updates can replace this locally built binary, so
retain the source changes when rebasing or rebuilding.

## Earlier installation rollback record

Backups are in `~/.local/state/cua-background-20261002/`:
`launcher.before`, `service.before`, `pi-extension.before/`,
`noches-current.before` and `noches-previous.before`.
The prior native build remains at `~/.local/lib/cua-driver/local-20260920/`.

To roll back Noches, stop the idle dev service, point
`~/.local/share/noches-dev/app/current` back to the retained
`0.1.62-dev.1` directory and restart `noches-dev.service`. Its data directory
was not moved. To roll back the driver, select the retained native executable
in `host.env`, restore the launcher/service backups as needed, reload the
user systemd manager and restart the idle CUA service. Restore the Pi files
from their backup and start a new Pi process. Do not delete or replace an
ownership lock while a client holds it. Package downgrades and compositor
plugin changes were not performed and are not part of this rollback.
