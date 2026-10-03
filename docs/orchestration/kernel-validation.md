# Kernel validation — 2026-10-03

Worktree: `/Volumes/DevDrive/AiStack/noches-wt/orch-kernel`, branch `orch/kernel`.
All builds/tests ran on Linux using the program helper and `target-orch`.
The helper pipes cargo output through `tail`; its shell exit code alone is
not a success gate. The reported cargo/test result lines were inspected.

## Commands

```sh
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel check -p zeron-engine --locked
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel test -p zeron-engine --locked
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel test -p zeron-engine --locked --lib orchestration
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel clippy -p zeron-engine --locked --all-targets -- -D warnings
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel clippy -p zeron-engine --locked --no-deps --lib --message-format short -- -D warnings
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel clippy -p zeron-engine --locked --no-deps --all-targets --message-format short
```

Check passed. Full engine tests passed, including the existing integration
suites (tests requiring external/live services retain their existing ignores).
Focused kernel tests passed: **33 passed, 0 failed**. Engine-only non-denying clippy completed; no
warning was reported in the new orchestration module. Strict clippy did not
pass due to the existing source warnings below, which were not changed.
The engine test build also reports the existing `unused doc comment` on the
`thread_local!` instrumentation in `sessions.rs:1299`.

## Strict dependency failure (verbatim)

```text
error: this `if` statement can be collapsed
   --> crates/update/src/lib.rs:560:5
    |
560 | /     if let Some(expected) = Some(expected) {
561 | |         if !actual.eq_ignore_ascii_case(expected.trim()) {
562 | |             tokio::fs::remove_file(&partial).await.ok();
563 | |             bail!("checksum mismatch for {file}: expected {expected}, got {actual}");
564 | |         }
565 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.98.0/index.html#collapsible_if
    = note: `-D clippy::collapsible-if` implied by `-D warnings`
    = help: to override `-D warnings` add `#[allow(clippy::collapsible_if)]`
help: collapse nested if block
    |
560 ~     if let Some(expected) = Some(expected)
561 ~         && !actual.eq_ignore_ascii_case(expected.trim()) {
562 |             tokio::fs::remove_file(&partial).await.ok();
563 |             bail!("checksum mismatch for {file}: expected {expected}, got {actual}");
564 ~         }
    |

error: this `if` statement can be collapsed
   --> crates/update/src/lib.rs:620:5
    |
620 | /     if let Some(expected) = expected_sha256 {
621 | |         if !record.sha256.eq_ignore_ascii_case(expected.trim()) {
622 | |             return false;
623 | |         }
624 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.98.0/index.html#collapsible_if
help: collapse nested if block
    |
620 ~     if let Some(expected) = expected_sha256
621 ~         && !record.sha256.eq_ignore_ascii_case(expected.trim()) {
622 |             return false;
623 ~         }
    |

error: could not compile `zeron-update` (lib) due to 2 previous errors
warning: build failed, waiting for other jobs to finish...
```

## Strict engine-only failures (verbatim short-format diagnostics)

```text
crates/engine/src/agent_accounts.rs:3778:9: error: this `if` statement can be collapsed
crates/engine/src/diff_sync.rs:14:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:15:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:16:5: error: doc list item without indentation
crates/engine/src/doc_host.rs:3656:5: error: this function has too many arguments (8/7)
crates/engine/src/repos.rs:1777:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { path_a.split('/').count().cmp(&path_b.split('/').count()) } else { std::cmp::Ordering::Equal }`
crates/engine/src/repos.rs:1782:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { dir_a.cmp(dir_b) } else { dir_b.cmp(dir_a) }`
crates/engine/src/run_journal.rs:114:13: error: this `if` statement can be collapsed
crates/engine/src/sessions.rs:1656:9: error: this `if` statement can be collapsed
crates/engine/src/terminals.rs:482:24: error: this expression creates a reference which is immediately dereferenced by the compiler: help: change this to: `bytes`
crates/engine/src/workspace_files.rs:1980:10: error: using `chunks_exact` with a constant chunk size: help: consider using `as_chunks` instead: `as_chunks::<2>().0.iter()`
error: could not compile `zeron-engine` (lib) due to 11 previous errors
```

No unrelated lint fix, suppression, harness invocation, main-checkout change,
remote push, PR, or merge was performed.
