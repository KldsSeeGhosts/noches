# Wave 3 integration handoff

Branch: `orch/w3-integrate`. Local commits only; no push or PR.

## Merge order and resolution decisions

1. Bound scheduler → canonical `ThreadService` intake (`3275a53b`).
2. `orch/w3-launch` (`dd7c16e4`).
3. `orch/w3-provider-instances` (`fdfa8ce7`).
4. Scheduled launch and attachment intake integration (`94b85591`).
5. `orch/w3-transfer` (`746124d8`).
6. `orch/w3-queue` (`28108bec`).
7. `orch/w3-pr-watch` (`6a80a3ae`).
8. Queue title instance reconciliation (`d05e916f`) and PR-link authority
   integration (`9e2ee87f`).

Conflicts retain every toolkit field/setter, operation variant/planner branch,
RPC constant and domain module. Worktree capability checks run before launch
dispatch; PR tools use their own capability/error family. Every domain
migration is retained; idempotent domain DDL is reconciled when reopening a
database made by an independently numbered slice.

Provider fields use the existing legacy defaults once, without duplicate
initializers. Git-message generation resolves the selected instance. Queue
title generation also forwards the actual instance. Runner composition retains
launch attachments, transfer history preparation/checkpoints, and queue's
sole-owner Loro drainer. Queue's strict promotion effects are not replaced
with normal send (which would duplicate messages or permit late restart).
`thread_lifecycle.rs`/`ChatLifecycle`, `AppState.thread_lifecycles`, `chat_lifecycle`,
`ORGANIZE_THREAD` and `ACKNOWLEDGE_THREAD_WOKE` retain the queue slice's names.

## Closed seams

- Bound schedules revalidate task/claim authority, use mode `auto` and the
  thread's current checkout, and enqueue through ordinary stable command and
  message receipts with schedule ID and actor/source.
- Unbound schedules use launch's shared preparation pipeline with the saved
  root/existing/new-worktree strategy. Each claim creates a new top-level
  thread; acceptance-loss replay preserves the original run and message.
  Acceptance is not turn completion; no scheduler directly starts a harness.
- Launch attachments use canonical thread intake, retaining native-child
  refusal ordering and uncertain-acceptance attachment retention.
- Git-actions installs PR-watch's host authority with source `Created`.
  Successful linking reports `prLinked:true`; failure preserves the PR URL
  with `uncertain` status and `prLinked:false`.
- Queue link/unlink invokes `PullRequestLinks::update_metadata` inside its
  ordinary receipt transaction, matching T3's legacy metadata reducer.
  Old/new scalar PR keys are replaced, unrelated links remain, and re-linking
  preserves the watch but resets snapshot/stack. Rollback and acceptance-loss
  replay cover both link representations; replay cannot resurrect an unlink.

## Remaining boundaries

Native cross-checkout handoff still needs driver-authorized resume or bounded
history transfer; instance/cwd fences are preserved. Canonical direct thread
deletion, sidebar-chat admission, strict existing-message delivery/selection
transition, and observational pinned-run history remain documented seams.
PR settlement still needs project/environment settings and integration of
queue's Auto marker/provider detach inside the existing sequence fence.
Launch's documented provider-clone/setup/replicated-progress gaps remain.
Live providers, Windows, and headed visual QA were not exercised here.

## Validation

All final suites use the local Mac `linux-test.sh` wrapper, one at a time,
with `LINUX_TARGET=target-w3-integrate`. No SSH to kidsseeghosts.

An earlier pre-final-merge orchestration run reported:

```text
174 passed; 2 failed; 1 ignored; 362 filtered out; finished in 2.69s

---- orchestration::tests::lane_retry_blocks_followers_but_not_titles_or_other_threads stdout ----
thread 'orchestration::tests::lane_retry_blocks_followers_but_not_titles_or_other_threads' (4944174) panicked at crates/engine/src/orchestration/tests.rs:640:5:
assertion failed: worker.step(NOW + 100).await.unwrap()
```

The other failure was a macOS `/var` versus `/private/var` checkout assertion;
canonical comparisons now handle those aliases.

The affected timing test was rerun with
`test -p zeron-engine --lib lane_retry_blocks_followers_but_not_titles_or_other_threads -- --test-threads=1`:

```text
running 1 test
test orchestration::tests::lane_retry_blocks_followers_but_not_titles_or_other_threads ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 625 filtered out; finished in 0.00s
```

The final merged engine `test -p zeron-engine --lib` run reported:

```text
test result: ok. 624 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 98.72s
```

After adding the additional precommit rollback assertion, its focused serial
regression (`metadata_pr_authority_is_atomic_replayable_and_preserves_sibling_and_watch`)
reported:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 625 filtered out; finished in 0.04s
```

| Suite | Final result |
| --- | --- |
| `test -p zeron-proto` | 46 unit + 14 oracle passed; zero failures |
| `test -p zeron-rpc` | 22 unit + 37 integration passed; zero failures; 2 live tests ignored |
| `test -p zeron-mcp` | 8 unit + 1 stdio integration passed; zero failures |
| `test -p zeron-doc` | 118 unit + 1 attachment integration passed; zero failures |
| `test -p zeron-harness` | Complete unit/integration/doc suite passed; zero failures; live/install fixtures remain ignored |
| `test -p zeron-ui --lib` | 1465 passed; zero failures; 1 native-font test ignored |
| Engine production boundaries (serial) | 5 passed; zero failures |

The additional production boundary command was:

```sh
test -p zeron-engine --test scheduler_bootstrap --test orchestration_bootstrap \
  --test orchestration_mcp --test queue_lifecycle_rpc -- --test-threads=1
```

It exercises real host assembly, scheduler Settings CRUD/manual/restart,
parent queue/steering over HTTP, injected MCP/live catalog, and desktop
lifecycle organization/wake acknowledgement. No merged queue/shutdown race
occurred in the final engine or boundary runs.

The wrapper returns only the last 120 output lines for a command, so its
harness output does not contain the early unit-suite aggregate. Its exit
status was zero, and every retained integration/doc result was `ok`.
The engine also reports an existing `unused_doc_comments` warning for
test-only instrumentation in `sessions.rs`; no new warnings were introduced.
