# Wave-3 launch implementation handoff

Branch: `orch/w3-launch`. No push, PR or merge.
Authority: read-only T3 checkout `f391794a35c604d57e166a3ab48d56fc6e4e469a`,
the pinned MCP descriptors, R3 P5a/P5b/P5c and C22–C24, and R6 #8.

## Implemented

| Tools | Backend |
| --- | --- |
| `t3_project_list/read/create/update/delete` | Existing space identities, durable T3 settings, title-only managed repositories, exact README/icon scaffold and folder slug, soft `commitError`, force guard, execution cancellation and durable cleanup. Never deletes the repository. |
| `t3_project_clone` | Shell-free clone, destination refusal before Git, no registration/adoption/removal, credential-redacted URLs/errors. See provider lookup exception below. |
| `t3_environment_read/preferences_update` | Host identity, durable partial updates, T3 defaults, empty instructions clear, ECMAScript trimming, 4,000-codepoint read truncation. |
| `t3_attachment_prepare_upload/discard`, `t3_thread_send_attachments` | Private persistent signing key, signed expiry, byte and max-eight limits, pending-only deletion, canonical owned references, copied claims, uncertain-acceptance retention, provider attachment forwarding. |
| `t3_thread_launch` | Top-level root/default, existing checkout, local/origin new checkout, scratch, preparing runs, atomic accepted intent and stable workflow identities, fetch→checkout→bind→setup→release, no retry key or dirty-edit copy. |
| `t3_worktree_handoff/status/list` | Caller-only binding CAS, in-flight guard, continuation and hold committed before detach, exact worktree failure family, inventory of refs rather than detached checkouts. |
| R6 #8 | Saved `async:false` blocking setup, managed owner PTY, durable run/status/exit/timeout, cancel/retry/continue guards, nonblocking option, uncertain restart hold. |

The Git worktree ownership lock is internal preparation bookkeeping, not a
permanent user lock. Git records the operation's reason before checkout;
recovery refuses an unowned destination even if its path and branch match.
The lock is released after binding or a definite preparation failure.
Thread deletion cancels execution and cleans only owned setup terminals and
claimed files; it does not remove the worktree.

## Remaining parity / merge work

- `LaunchThreadIntake` and `host_intake.rs` now call canonical thread intake:
  auto steering, native-child refusal, attachments and sender provenance.
  Scheduled launches share MCP launch's workspace preparation with stable
  claim command/message identities and schedule actor/source.
  **TODO(merge-threads)** remains for driver-authorized cross-cwd native resume
  or bounded conversation transfer after handoff. Automatic native resume
  remains fenced by instance and cwd; no real-provider continuation is claimed.
- Provider/repository clone input currently derives public GitHub/GitLab/
  Bitbucket URLs and returns `repository:null`; it does **not** perform T3's
  configured provider lookup (including enterprise hosts, Forgejo and Azure).
  Raw `remoteUrl` cloning works independently. This is not full clone parity.
- Temporary worktree branches are stable journal-derived names; T3's
  background semantic rename is not implemented.
- Generic Git execution reuses Noches' existing runner without T3's exact
  per-operation deadlines/progress callbacks or submodule setup policy.
- Setup and launch progress have passive owner RPC reads, not a new replicated
  chat2 progress/watch projection. Canonical direct thread deletion should
  share the local deletion planner at merge rather than duplicate it.
- The explicit requested destination rule refuses even an existing empty
  folder; the inspected upstream clone service allows an empty directory.

## Evidence and verification

Source-informed tests mirror `project/handlers.test.ts`,
`WorktreeMcpService.test.ts`, `ThreadLaunchService.test.ts`,
`packages/shared/src/path.test.ts`, and C22–C24. They validate successful
payloads against pinned output schemas and exercise actual MCP framing.
These are Rust regression tests, **not** an executed cross-app T3 trace oracle.

Linux validation uses:

```sh
LINUX_TARGET=target-w3-launch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/w3-launch \
  test -p zeron-engine -p zeron-proto -p zeron-rpc -p zeron-mcp --lib
```

The helper's remote Cargo pipeline lacks `pipefail`; inspect test-result lines,
not only the helper exit code. Formatting is limited to touched Rust files.
Final Linux results: engine **475 passed, 0 failed, 3 ignored**; MCP **8 passed**;
proto **40 passed**; RPC **22 passed**, all with zero failures.
No CI clippy invocation was found in `.github/workflows`.
No GPUI changes; macOS/Windows native setup, live provider execution and
headed visual QA remain unverified.

Stable UI API: [launch-ui-api.md](launch-ui-api.md).
