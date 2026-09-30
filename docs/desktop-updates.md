# Installing and updating Noches

The repository is [KldsSeeGhosts/noches](https://github.com/KldsSeeGhosts/noches). `dev` publishes Noches Dev, and `main` publishes Noches. Both applications have Settings → Updates and a Check for Updates application-menu action. Download the update while working, then select Install and restart after finishing active runs, closing terminals, and saving files.

## First installation

Local source builds have no distributed update channel; install a versioned package. Existing distributed 0.3.x dev/stable clients migrate through the one-time bridge described below.

Download a versioned installer from [GitHub Releases](https://github.com/KldsSeeGhosts/noches/releases). The permanent feeds are `noches-epoch1-dev` and `noches-epoch1-stable`; the older `noches-dev` and `noches-stable` feeds are pinned migration entry points. Choose a numbered version for an installer.

On macOS, open the DMG and drag Noches or Noches Dev to Applications. Eject the DMG and launch the installed app. These builds use ad-hoc signing by default and do not require a paid Apple Developer account to build or publish. macOS may require approval in System Settings → Privacy & Security before the first launch. Permission prompts can recur after ad-hoc-signed updates. Developer ID signing and notarization remain optional improvements when a paid account is available.

On Linux, extract the matching x86_64 or aarch64 tarball and run `bash install.sh` inside it. Python 3 is required by the installer. The app installs under your home directory, with no root privileges and no automatic daemon installation. Launch it from the desktop application menu or `~/.local/bin/noches` / `~/.local/bin/noches-dev`.

Linux release builds use Ubuntu 24.04. Other distributions need compatible glibc and graphics libraries. Packages include Chromium and require its system libraries and sandbox support; see [Linux browser dependencies](reference/linux-browser.md). macOS CI currently produces Apple silicon builds. Intel Mac release artifacts are not included.

## Channel separation and existing data

| | Noches | Noches Dev |
| --- | --- | --- |
| Source branch | `main` | `dev` |
| Channel | `stable` | `dev` |
| Data | `~/.noches` | `~/.noches-dev` |
| Linux application root | `~/.local/share/noches/app` | `~/.local/share/noches-dev/app` |
| IPC port | 27655 | 27656 |
| Optional Linux service | `noches.service` | `noches-dev.service` |
| macOS bundle ID | `io.github.kldsseeghosts.noches` | `io.github.kldsseeghosts.noches.dev` |

Ordinary source builds remain local builds, keep the existing `~/.zeron` data and IPC port, and do not install published updates. Your existing sessions are not deleted or automatically moved. The two new installed channels start with separate data directories.

To bring existing local data into one channel, quit all applications and stop the old daemon first. Make a backup of `~/.zeron`, then copy it into the chosen channel's data directory only if that destination is empty. Do not run two processes against the same data directory. Do not copy a synced device identity into multiple channels; reconnect or pair the other channel independently. Provider CLI credentials stay in the providers' own locations.

`NOCHES_DATA_DIR` overrides the data location. `ZERON_DATA_DIR` remains a compatibility fallback for existing scripts. Internal Rust crate and executable names still use `zeron`; public installer, launcher, menu, and bundle names use Noches.

## One source tree

macOS and Linux packages are built from the same commit. The Linux Wayland input repair lives in `vendor/gpui_linux` and is Cargo-patched onto the pinned `zeronsh/zui` revision. A push to `dev` or `main` publishes that tree; a Linux machine does not need a local `gpui_linux` overlay, and a MacBook push contains the same Linux crate. See `vendor/gpui_linux/NOCHES-PATCH.md`.

Cua driver patches live in `scripts/cua/` on the same branch. They apply to a separate Cua checkout. The installed app starts the patched driver for Pi through `noches_cua` and keeps Cua seats off the physical Wayland seat. Updating the app does not build the driver or load the Hyprland plugin. See `docs/computer-use.md`.

## Release cadence

1. Merge a feature or fix into `dev`. The Noches releases workflow tests and packages Linux on both architectures and macOS on Apple silicon. It publishes a numbered prerelease, then advances the dev feed only when the complete platform set is available.
2. Promote through a `dev` → `main` PR. The same workflow publishes a stable build and advances only the stable feed.
3. In the installed app, open Settings → Updates. Checks also run after startup and every six hours, independently of account sign-in or remote-host connections.

The product version is plain SemVer in `[workspace.package].version` in `Cargo.toml` (currently `0.1.0`). Bump it deliberately: `1.0.0` for a major release, `0.2.0` for a feature/minor release, `0.1.1` for a fix/patch (increment the corresponding component from the current version). It appears in the About panel, Updates page, macOS marketing version, GitHub release title (`Noches Dev 0.1.0`), and new release manifests. Multiple builds can share the same product version.

The **separate build identity** stays CI-owned: stable `0.1.<workflow-run-number>` and development `0.1.<workflow-run-number>-dev.<attempt>`. Unique tags, artifact filenames, installer directories, updater ordering, and macOS build numbers use this identity, not the product version. The Updates page and release title show it next to the product version for troubleshooting. Older manifests without `display_version` fall back to the build identity. Do not compare product versions to decide if an update is available. A completed release cannot be replaced with different artifacts. If a draft version release was interrupted, inspect and resolve it manually before rerunning; do not clobber it blindly. Start a new workflow run for a new stable build.

Each channel uses a GitHub release containing `manifest.json`:

- `https://github.com/KldsSeeGhosts/noches/releases/download/noches-epoch1-dev/manifest.json`
- `https://github.com/KldsSeeGhosts/noches/releases/download/noches-epoch1-stable/manifest.json`

Manifests carry `epoch`. Epoch-1 `0.1.x` builds compare `(epoch, SemVer)` and read only the new permanent feed. Older distributed `0.3.x` builds **ignore epoch**, read only the legacy feed, and compare SemVer alone; putting `0.1.x` on their feed would strand them. Ordinary CI publishes only the epoch-1 feed and never changes a legacy feed. Publication refuses to move the new feed backwards or replace an unexpected release.

### One-time 0.3.x migration (per channel)

1. Merge this code into `dev`. Let the ordinary dev push finish publishing a complete epoch-1 `0.1.x` version and `noches-epoch1-dev`. Verify its manifest and downloadable platform assets. Do not publish a bridge before this destination exists. For stable **before the main merge**, manually dispatch the release workflow on `dev` with `migration=seed`, `channel=stable`, `confirm=seed-stable`. This builds a stable 0.1.x binary from the fresh dev tree and seeds only `noches-epoch1-stable`. If main has already merged and published a normal stable release, a seed is unnecessary. A seed does not touch the legacy feed.
2. For each channel, verify the legacy feed currently points to the expected published 0.3.x release and the epoch-1 destination exists. Manually dispatch on the **dev branch** with `migration=bridge`, the desired `channel`, and `confirm=bridge-dev` or `confirm=bridge-stable`. Keep the branch tip unchanged until publication completes. Do this once per channel, after its destination feed exists. `bridge-stable` may run before merging dev to main. The workflow builds all platforms from that current dev commit with compiled epoch **0**, version `0.4.<run-number>` (dev adds `-dev.<attempt>`), and a compiled feed URL pointing only to `noches-epoch1-{channel}`. It publishes immutable version assets, then writes their manifest to the old `noches-{channel}` feed with epoch **1**. Old clients see a higher SemVer and take the bridge; already-installed epoch-1 clients still reading the old URL also see a higher epoch/version and take it. The bridge then sees the epoch-1 0.1.x feed as newer than its own epoch-0 version.
3. Inspect both feeds and test one real 0.3.x installation on each supported platform, including its second hop and data preservation. After this, merge dev to main to begin normal stable publication if not already done. Never use ordinary releases to update the legacy feed. Keep legacy tags and manifests available indefinitely; never reset them to 0.1.x or rerun a bridge with a new version. An interrupted legacy bridge upload can leave its feed without `manifest.json`; stop and repair it manually after verifying the immutable version release, rather than blindly rerunning. For an interrupted epoch-1 feed upload, a normal publication from the current branch tip can restore the missing manifest from its complete immutable release. The bridge only accepts a complete published legacy 0.3.x feed and a complete published epoch-1 0.1.x feed; existing version tags/drafts are not overwritten.

Manifests point to assets under immutable version tags. Linux and Apple silicon macOS packages build on GitHub-hosted runners. Builds can overlap when new commits arrive; only publication is serialized per branch, so an older build cannot hold up new builds while waiting for a runner. Publication rejects stale branch builds and older feed versions, and uploads all artifacts before advancing the feed. A workflow interrupted during replacement of an epoch-1 channel manifest may temporarily make checks fail; rerun the current branch-tip normal workflow to repair a missing manifest after its complete version release is verified. Legacy bridge feeds require manual repair. The installed app remains unchanged when a check fails.

The updater requires the Noches product ID, matching channel, valid newer version, HTTPS asset URL, exact size, and SHA-256 digest. There is no legacy upstream-feed fallback. HTTPS and the GitHub repository's access controls authenticate publication; manifests do not have a separate cryptographic signature. macOS verifies the staged bundle's code signature and bundle identifier, including ad-hoc signatures. This does not give an ad-hoc build a Developer ID identity or notarization ticket.

All release jobs use the normal GitHub Actions token. Apple credentials are optional. The previous Cloudflare release publishing path is removed, and the inherited deployment workflow is gated to the upstream owner so this fork cannot deploy into Zeron's infrastructure.

## Local packages

For a local build that can join the development update channel after installation:

```bash
NOCHES_CHANNEL=dev NOCHES_VERSION=0.1.0-dev.0 bash scripts/package-macos.sh
# Or on Linux:
NOCHES_CHANNEL=dev NOCHES_VERSION=0.1.0-dev.0 bash scripts/package-linux.sh
```

Install the resulting package from `target/package`. Future published dev versions will sort newer than this bootstrap version. `NOCHES_REPOSITORY` and `NOCHES_COMMIT` are compiled into packaged builds. `NOCHES_RELEASES_URL` is a runtime feed override for controlled testing; it must be an HTTPS base URL containing a valid channel manifest.

## Restart and recovery

The update screen keeps the downloaded version until restart. Close additional Noches windows before installing so their editors and local work can remain safe. Local engine checks refuse a restart while runs or terminals are active. A failed download or checksum leaves the installation intact. Save-file handling runs before replacement.

Linux retains an `app/previous` symlink. Quit the app and stop its optional service before manually restoring that version. macOS keeps a hidden `.Noches.app.old-<pid>` or `.Noches Dev.app.old-<pid>` bundle next to the installation. A failed bundle rename attempts to restore the old bundle. Old versions are retained for manual recovery; there is no automatic crash-detection rollback or old-version cleanup yet. Restoring an executable does not reverse a database migration.

Updating from a remote-connected window updates the local desktop only. If a Linux background service is running, return to a local window so its active work can be checked before restarting the service. Remote hosts are never implicitly upgraded.

## Mobile

This workflow distributes desktop applications only. iOS continues to build locally in Xcode. TestFlight and App Store distribution require a paid Apple Developer membership, so neither is required for desktop releases. Existing iOS identifiers and provisioning are unchanged by the desktop updater.
