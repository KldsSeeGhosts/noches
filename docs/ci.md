# CI and release validation

## One owner per event

Dev/feature desktop PRs run **UI tests** once, including reusable Cursor compatibility.
PRs targeting `main` run it once through the release caller instead, preserving
the branch's protected `validation / ...`, `compatibility / ...`, `prepare` and
`edge-validation` check names. The caller disables the nested Cursor job because
it owns the compatibility check directly. There is never a second copy.
After a merge to `dev` or `main`, the release workflow calls the full desktop
suite once alongside package builds, Cursor compatibility and edge validation;
the UI workflow has no independent push trigger.
The lightweight **CI policy** workflow checks release scripts, workflow contracts
and actionlint on every PR. Edge PRs retain the existing Preview networking tests.

Dev/feature mobile changes run **iOS tests** separately. Main PRs use a compatibility
job named `validation / ios-tests` when mobile files change; both jobs share the
same composite action. Post-merge desktop publishing does not depend
on simulator animation/keyboard tests. TestFlight explicitly requires the same
iOS test workflow to pass before its upload job can obtain release credentials.
No mobile tests have been disabled.

## Build the sync gate once

`scripts/ci/run-session-sync.py` builds the union of the five sync-gate packages
and required test targets in one locked Cargo invocation. It selects the
executables from Cargo JSON, refuses missing/ambiguous targets, and runs the
previous filters in each package's working directory. In particular, this does
not accidentally enable unrelated engine integration tests or real-login tests.
MCP doctests remain covered with the same package selection. GitHub job summaries
separate build time from test time so we can measure the improvement.

The October 5 baseline spent approximately 15 minutes compiling in a 17-minute
sync job, rebuilding proto/doc/sync under multiple feature sets. The new runner
is intended to remove those repeated builds; hosted wall-time improvement must
be measured after landing, not inferred from a warm local build.

Cache writers also save dependencies on same-repository PRs, including on test
failure. GitHub scopes those caches to the PR merge ref: later PR attempts can
warm-start, but integration branches cannot restore a PR's cache. Fork PRs remain
read-only cache consumers. Reader jobs sharing a cache do not compete to save it.

## Release safeguards and remaining waits

PRs still cancel superseded validation. New `dev` pushes also cancel obsolete
desktop **test/build jobs**, not an entire release workflow. Validation and package
build groups separate pushes from manual dispatches. Publisher concurrency remains serialized
and non-cancelling; stable releases and migration dispatches are not automatically
interrupted.

Publishing still requires both Linux architectures, the macOS package, complete
desktop validation (including Cursor), and edge validation. The branch-current
checks in `noches-release.py` remain intact. Neither green PR checks nor cache
hits bypass testing of the actual post-merge commit: a PR SHA and merge SHA are
different identities even when their source trees happen to match.

This removes duplicated work and mobile coupling; it does not make publishing
instant or suppress flaky desktop checks. Signing/notarization and hosted runner
queues remain. No repository branch-protection settings are changed; the existing
six required check names on `main` continue to be emitted by its PR caller.

## Local validation

In a Python virtual environment, install `scripts/ci/requirements.txt`, then run:

```sh
python3 scripts/test-noches-release.py
python3 -m unittest discover -s scripts/ci -p 'test_*.py'
actionlint -shellcheck= -pyflakes=
python3 scripts/ci/run-session-sync.py
```

The last command needs the pinned Rust toolchain on `PATH` and actually executes
the selected Rust tests. Linux/macOS GUI validation and mobile simulator tests
remain hosted checks, not claims made by these workflow-contract unit tests.
