# Upstream review: Zeron v0.2.85 - v0.2.92

Follows `docs/upstream-sync-2026-09-22.md` (reviewed through v0.2.84). This pass reviews upstream through [v0.2.92](https://github.com/zeronsh/zeron/releases/tag/v0.2.92) and also picks up harness and catalog fixes the previous pass deferred. Every port keeps its upstream provenance in a `(cherry picked from commit ...)` trailer.

## Ported

**Chat sync resource safety**

- [ed3df8fb](https://github.com/zeronsh/zeron/commit/ed3df8fb) (#544): bounded shared connection and request budgets, explicit document vs sync lifetimes, durable wakeups without opening every chat. Noches adds a bounded retry for host nudges the edge rejects (the hosted edge answers 503 when its nudge queue is full).
- [e7ddbbe7](https://github.com/zeronsh/zeron/commit/e7ddbbe7) (#550): focus-aware connection rotation under load. Wired to Noches' pane and subagent watches. The `crates/mcp` hunks are dropped (Noches has no MCP crate).
- [45d34678](https://github.com/zeronsh/zeron/commit/45d34678) (#552): no transient connection notices for dormant chats. Storage failures use the failed status hue.

**Harness hardening**

- ACP/Pi: [1251321c](https://github.com/zeronsh/zeron/commit/1251321c), [29018a3e](https://github.com/zeronsh/zeron/commit/29018a3e), [549cb3ba](https://github.com/zeronsh/zeron/commit/549cb3ba), [55b09da6](https://github.com/zeronsh/zeron/commit/55b09da6), [ceb8d892](https://github.com/zeronsh/zeron/commit/ceb8d892), [248fe26f](https://github.com/zeronsh/zeron/commit/248fe26f). Pi context usage is flushed before `Done` and skipped on cancel.
- OpenCode: [08376d23](https://github.com/zeronsh/zeron/commit/08376d23), [db50cd37](https://github.com/zeronsh/zeron/commit/db50cd37), [fb01b951](https://github.com/zeronsh/zeron/commit/fb01b951) (2.0.4+ command body key, HTTP rejection settles the turn). An idle status no longer settles a turn whose prompt POST is still pending. An unknown 2.x version is treated as 2.0.4+.

**Model catalogs**

- [8e3ab3d0](https://github.com/zeronsh/zeron/commit/8e3ab3d0), [212a6ce8](https://github.com/zeronsh/zeron/commit/212a6ce8) (without the unused `force` flag), [ad95708f](https://github.com/zeronsh/zeron/commit/ad95708f). The picker's `ListModels` deadline now derives from the widest harness discovery budget.

**Agent accounts**

- [6eb5917d](https://github.com/zeronsh/zeron/commit/6eb5917d) (#546), [bbf590c8](https://github.com/zeronsh/zeron/commit/bbf590c8) (#542), [79b8d84c](https://github.com/zeronsh/zeron/commit/79b8d84c) (#547), landed as two combined commits because upstream's final code interleaves them. Includes the remote sign-in callback tunnels from [b782d043](https://github.com/zeronsh/zeron/commit/b782d043) (only that plumbing; the settings redesign is not taken). UI lives in Noches' own Settings -> Accounts; the plan-usage ring sits beside the Noches context ring and follows the pane's own chat in split views. Claude switches validate `~/.claude.json` before writing credentials.

**Small fixes and features**

- [d0b0e89d](https://github.com/zeronsh/zeron/commit/d0b0e89d) system proxy. Noches centralizes the exemption in `crate::http_error::is_direct_destination` (loopback, private, link-local, tailnet, `*.ts.net`, `*.local`) and applies it to every client that carries credentials to a configurable host.
- [e3f6f594](https://github.com/zeronsh/zeron/commit/e3f6f594), [88a07863](https://github.com/zeronsh/zeron/commit/88a07863), [aff9a498](https://github.com/zeronsh/zeron/commit/aff9a498) Windows fixes; [87ef6c8f](https://github.com/zeronsh/zeron/commit/87ef6c8f) only partially: tab drops work, but drag initiation stays gated on Windows until the pinned zui line carries its drag-threshold fix.
- [5f60293c](https://github.com/zeronsh/zeron/commit/5f60293c), [c2230f07](https://github.com/zeronsh/zeron/commit/c2230f07), [ae22a598](https://github.com/zeronsh/zeron/commit/ae22a598), [1fdcfe19](https://github.com/zeronsh/zeron/commit/1fdcfe19), [fb7adf74](https://github.com/zeronsh/zeron/commit/fb7adf74) (exposed a projectless `~` cwd bug in `OpenTerminal`, fixed), [cfe91887](https://github.com/zeronsh/zeron/commit/cfe91887), [d1010657](https://github.com/zeronsh/zeron/commit/d1010657), [67524512](https://github.com/zeronsh/zeron/commit/67524512).
- [d268830b](https://github.com/zeronsh/zeron/commit/d268830b) Settings reopens on the last section; [23e258ff](https://github.com/zeronsh/zeron/commit/23e258ff) terminals from the new-chat canvas (keyed per device for projectless canvases); [03b67beb](https://github.com/zeronsh/zeron/commit/03b67beb) projectless file explorer.
- [a456eb09](https://github.com/zeronsh/zeron/commit/a456eb09) safe working tree discard. Noches pins the confirmation to every untracked file (`--untracked-files=all`) with raw-byte digests, and refuses when any untracked file is unreadable or the snapshot is truncated.

## Not ported

- [8ee7a622](https://github.com/zeronsh/zeron/commit/8ee7a622), [9abe0167](https://github.com/zeronsh/zeron/commit/9abe0167): already in Noches.
- [adb63378](https://github.com/zeronsh/zeron/commit/adb63378) OpenCode auto-approve: upstream's stopgap for missing approval UI. Noches routes OpenCode permissions to its composer approval panel.
- [d8ea030f](https://github.com/zeronsh/zeron/commit/d8ea030f), [b3ec6b7e](https://github.com/zeronsh/zeron/commit/b3ec6b7e): depend on upstream's persisted model catalog, which Noches does not have.
- [94304283](https://github.com/zeronsh/zeron/commit/94304283): Noches has its own update channels; source builds do not check for updates.
- [b782d043](https://github.com/zeronsh/zeron/commit/b782d043) settings redesign and [731697b6](https://github.com/zeronsh/zeron/commit/731697b6) side chats / MCP injection: overlap Noches' own settings and subagent surfaces. MCP injection may be worth revisiting on its own.
- iOS test churn ([096f361f](https://github.com/zeronsh/zeron/commit/096f361f)), landing page, docs, version bumps.

## Known flakes

These also fail intermittently on `dev` and are not caused by this pass: `zeron-preview` `peer::tests::actual_webrtc_pair_streams_in_both_directions` and `tests/leak.rs` under load, and `zeron-engine` `git_history_search_is_fuzzy_complete_and_accepts_sha_prefixes`.
