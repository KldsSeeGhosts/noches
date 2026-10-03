# Todo panel

Behavioral port of upstream `1f7b74a7fc97b0ca1d0379e77141075d73db8adf`,
PR #707. The checklist is a tray above the pane's composer, agents tray and
message queue. It follows that composer's `ChatTarget`, not global selection.
Each composer owns its cache, scroll handle and per-chat presentation choices,
including when two panes show the same chat.
Explicit choices survive Noches' project-switch pane parking; cached host
checklists and scroll handles are not restored into a rebuilt composer.

## Data and compatibility

`TodoItem` keeps `{ text, done }` and adds an optional, defaulted `status`.
Constructors write it only for `inProgress`. Old pending/completed items
serialize unchanged. Unknown or malformed statuses decode as absent.
Completion always comes from `done`, so old readers and newer readers agree.

Claude TodoWrite, OpenCode todowrite, ACP plan updates and ACP raw-input todo
tools preserve in-progress status. Cursor updateTodos preserves both legacy
completion booleans and status strings. Codex `turn/plan/updated` replaces the
live checklist; legacy todoList items still use their completion booleans.
Child Codex plan notifications do not change the parent's checklist.

ACP and Codex plans reuse `LIVE_PLAN_TOOL_ID`. Noches already exempts this id
from stale tool-echo filtering. The document writer refreshes the part in place
within a segment; the newest Todo part wins across segments. An empty write
clears the panel. The edge render-only policy retains the optional status.

The Swift companion needs no production change. `HostPart.call` is generic
`JSONValue`, and the cloud adapter also retains generic item payloads. Both
render Todo as `N/M done`, counting only `done`. In-progress items remain
unfinished there; this port adds no mobile checklist panel. A companion
regression test covers both paths and an unknown future status.

## Presentation

The header reports Todo, completed/total and the current item when collapsed.
Unfinished lists expand by default; finished lists compact. Completion plus an
idle turn resets an explicit expansion choice once. Manual reopening sticks.
Lists over six items fold to three around the current item, with earlier/later
controls that reveal items without reordering them.

Dismiss is available while idle, even for interrupted or cancelled lists.
It holds for the same list until the checklist changes. Completed glyphs use
the shared status palette; Todo activity remains neutral. An in-progress item
spins only during a live turn. Controls have labels, tab stops and focus states.
Updates do not request focus; animations follow reduced-motion settings.

The tray uses Noches' existing frost and queue helpers. It is narrower when
an agents or queue tray follows it. The composer owns its measured height,
so transcript clearance includes the checklist. Transcript tool details show
`[x]` completed, `[~]` in progress and `[ ]` pending.

For live mock checks, use `ZERON_HARNESS=mock ZERON_MOCK_TODO=1` with
`ZERON_MOCK_DELAY_MS=900`. The fixture walks an eight-item list to completion
using the live plan id. Existing agents, thinking and tool-palette fixtures
remain available.

Upstream's client demo crate does not exist in Noches. Its demo transcript
change is omitted. CONTRIBUTORS.md and upstream screenshots are also omitted;
those screenshots do not validate Noches' native UI.
