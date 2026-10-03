# Session MCP

`noches mcp` is an opt-in stdio server over the running engine's loopback IPC.
`ZERON_IPC_PORT` selects the IPC port, default 27654. Configure this command
explicitly in your MCP client. It is not injected into agents or loaded from
repository configuration. `noches browser-mcp` remains a separate,
conversation-scoped browser server.

This ports the standalone-session behavior of upstream #706. Noches has no
upstream side-chat parent schema, so creation accepts only `kind: "chat"`.
Omitting kind also creates a standalone session. Nonempty `parent`, `kind:
"side"` and unknown kinds fail before writes. No document fields were added.
Noches' existing Sessions list displays the new rows without UI changes.

## Discovery and creation

Discover hosts with `list_devices`, then `list_projects {device}`,
`list_harnesses {device}` and `list_models {device, harness}`. Without a device,
catalogs come from the local engine; projects list all known projects.

```json
{
  "kind": "chat",
  "device": "<listed-device-id>",
  "project": "<listed-project-id>",
  "harness": "codex",
  "model": "<model-id-from-that-host>",
  "title": "Implement feature",
  "prompt": "Implement the feature",
  "wait": false
}
```

Pass that object to `create_chat`. Project alone determines its host. Device
alone creates a projectless session on that host, using its home directory.
Neither argument uses the local engine. With both arguments, the project must
belong to the selected device. Names and paths resolve within that device;
ambiguous matches fail with candidate ids. Explicit ids take precedence.

Creation requires an installed, enabled, non-mock harness from the selected
host's catalog. The default is Claude Code when available, otherwise the first
available harness. An explicit model must be in that host's model catalog.
Catalog errors stop creation; there is no local fallback.

`create_chats {"requests": [...]}` accepts 1 to 32 requests. Results preserve
order and include per-request errors. Successful requests are not rolled back.
For independent sessions, launch with `wait: false`, then call `wait_for_turn`.

## Authorization and compatibility

The server uses existing engine RPCs, not a new edge connection. Devices and
projects must be visible in the current engine profile. A project id cannot
override the selected device. A remote catalog query must succeed before any
registry write; the existing owner-authenticated device relay still decides
whether this account may reach that host. Registry visibility is not a grant
to access another user's device. No relay or registry authorization was changed.

`cwd`, attachments, automatic approval, and sandbox disabling are not accepted.
Creation allows `read-only` or `workspace-write`, defaulting to the latter.
Run requests always carry `autoApprove: false`. Pending questions and approvals
use the existing `respond_to_input` durable command and harness approval route.
This is not an unattended approval service.

Remote operations require the caller engine to advertise the additive
`mcp-session-routing-v1` capability. Older callers fail closed instead of silently
using a local catalog or transcript. Existing remote host methods and document
fields are unchanged; unsupported hosts return their normal RPC error or fail
catalog validation. Old hosts and the Swift companion need no schema update.
Transport errors are returned without retrying writes that may already have
executed.

## Conversation tools

`read_chat {chat}` returns raw transcript entries from the session's host.
Chats accept an id, unique id prefix or exact title. `send_message {chat, text,
wait?, timeout_secs?}` starts an idle configured session. It rejects working or
awaiting-input sessions rather than overriding Noches' pane-specific steering
and queue policy. `interrupt_chat {chat}` interrupts through the host command
queue. `respond_to_input {chat, request_id, answers}` answers pending input.
Optional `ZERON_CHAT_ID` attributes sends and prevents self-messaging; it does
not grant authority.

Sends remember existing transcript message ids on this MCP connection.
`wait_for_turn {chat, timeout_secs?}` uses that baseline even when called after
`wait: false` and before the host publishes a session row. A remote clock behind
the caller does not hide new replies. Completion may precede transcript sync;
the wait allows the new complete assistant entry to arrive within the same
timeout. A previous reply never substitutes for a missing new reply.
Timeouts are 1 to 600 seconds, default 120. A separate connection without a send
baseline reports current posture. At most 256 uncollected sends are retained.

## Isolated smoke test

`cargo run -p zeron-engine --example mcp_standalone_smoke` serves a temporary
engine profile with `smoke-device`, `smoke-project` and a scripted installed
Codex harness offering `smoke-1`. A first prompt and subsequent send return
`pong`. It does not touch a running app or consume provider quota.

`cargo test -p zeron-mcp` checks validation, host catalogs, old callers, batch
errors, delayed replies, missing sessions and clock skew.
`cargo test -p zeron-engine --test device_routing
mcp_standalone_session_executes_on_the_selected_device` checks two-engine
creation, execution, transcript reads, a second turn and interrupt delivery.
