# Noches design direction: control plane, in color

Noches supervises coding agents running on several machines. The chrome must
answer at a glance: **what** is each session doing, **where** (project,
branch, device), **which agent**, and **does it need me**. The sidebar
uses BB's compact, title-first thread hierarchy; the color language follows Cursor and T3:
color is plentiful but always *means* something.

## Rules

1. **Color encodes state and identity, never decoration.**
   - State: `crate::status_palette::SessionState` is the only source for
     status hues. Working is sky, Awaiting input is indigo, Completed (unseen)
     is emerald, and Failed is the theme's danger color. Queued and idle
     sessions are neutral.
   - Identity: harness marks keep their brand tint (Claude orange; the other
     marks are monochrome by design). Projects show their repository favicon
     or a colored monogram (`Shell::render_project_icon`).
   - Change: PR badges keep their state colors. Diff stats in the Changes pane
     use `theme.diff_add` and `theme.diff_del`; per-card diff counts in the
     sidebar are deferred to a later phase (see the context line).
   - Surfaces, text, and hairlines stay neutral theme tokens.
2. **One surface, hairline splits.** Panes are flush on the same shell
   backdrop as a lone session and are separated by 1px `theme.border`
   hairlines. Do not add opaque pane fills that darken split sessions.
3. **Metadata is monospace.** Branch, device, elapsed time, and the model use
   `theme.font_mono` at 11px. Titles use the UI face.
4. **Weight is hierarchy.** Titles use NORMAL weight, or MEDIUM when the
   session needs you. Project names and status labels use MEDIUM at 11.5 to
   12px.
5. **No mascots.** Buddy avatars are gone. Identity comes from the project
   badge and the harness mark.

Status colors are explicit hues with light and dark variants (like the
monogram palette), so every theme reads a state the same way. Do not hardcode
any other hex values in UI code.

## Sidebar (compact control-plane threads)

The thread, not the repository, is the primary navigation target. BB's
compact thread hierarchy replaces the earlier three-line T3 card: title
first, one context line below, status at the trailing edge. No blank
metadata row, repeated status words, or archive text pill.

```
 ●●●  ▯  ← →                           +
 [dir] All projects ⌄         [⌕] [≡]
 Needs you
▌✳ Fix auth token refresh              ◌
 [N] noches  ⑂ feat/auth-refresh      #42
 Running
 ✳ Port sidebar sections           ▥ 2m
 [N] noches  ⑂ design/control-plane
 Recent
 ◎ Pricing page copy pass            3h
 [W] website  ⑂ main
```

### Thread (two lines, 56px)

- Geometry: 8px top padding, 20px title line, 4px gap, 16px context
  line, 8px bottom padding. FLIP and keyboard projection share the rendered
  list. Child-agent disclosures add their measured height to the base row.
- Title line: 13px harness mark in its brand tint, 6px gap, title at 13px
  NORMAL (MEDIUM when the session needs you). The trailing 56px slot keeps
  title truncation stable across hover and status changes. While jump hints
  are held, wider custom shortcut labels can grow that slot without overlapping
  the title.
- Live status uses a 12px glyph from the shared state palette, with the full
  state in its tooltip and accessibility label. Working keeps the animated
  equalizer and mono 11px elapsed time. Idle rows show mono relative time.
  State changes fade quickly; reduced motion stays static.
- Hover reveals a quiet 20px Archive icon button. Only the clock yields:
  live status remains visible. The action stops propagation and never starts
  a sidebar drag or selects the thread. Jump hints take precedence over the
  action while the shortcut modifier is held.
- Context line: 14px project favicon/colored monogram, project name at
  11.5px `text_muted`, branch glyph + truncated branch in mono 11px
  `text_faint`, PR badge, and remote device (only when not local). Branchless
  sessions still have a useful project line, not a reserved empty third row.
- Needs-you rows keep the 2px gutter bar in the state hue (indigo awaiting,
  danger failed). Selection is a neutral wash, not a ring or a new accent.
- Running child agents stay inside the selected/open row's wash and radius.
  Their disclosure and the list's reorder glides remain animated. Initial
  list entry is a restrained 180ms settle with a capped 15ms stagger.
- Deferred: per-row `+a -d` counts need a bounded summary-only engine stream
  keyed by checkout identity. Do not subscribe every row to full patches.

### State sections

Unchanged from v1: **Needs you** (awaiting input or failed), **Running**
(working or queued), then the rest in the chosen organization. **Recent** is
shown only for `InOneList` and only when a section above exists. Headers are
30px tall with the label bottom-aligned. Each non-empty header carries a
6px dot in its state color (indigo for Needs you, sky for Running) before the
label. `sidebar_visible_order` must match the rendered order.

## Panes

Shared shell surface, hairline dividers, and focus cues are unchanged from v1. The
pane header is a 36px row, `pl` 10px / `pr` 6px (the trailing inset pairs
with the titlebar's `TITLEBAR_ACTION_EDGE_INSET`), content left to right:

- The harness mark at 14px in its brand tint inside a fixed 16px column.
  A bound chat shows its configured harness; an unbound new-session pane
  shows the harness picked in its composer. When no harness resolves the
  column stays empty - never a placeholder glyph.
- The title at 13px MEDIUM (`text` when focused, `text_muted` when not).
- A 14px project badge, then the context in mono 11px `text_faint`:
  `{project}:{branch}`, plus ` · {device}` for a remote device.
- A status label only when it adds information the transcript does not
  already show: icon and label in the state color at 11.5px MEDIUM. Awaiting
  input and Failed show on every pane; unfocused panes in a split show all
  states; Working hides on the focused pane because the transcript's live
  activity line already carries it.
- The project action control: a quiet 24px ghost segment. With no action
  configured it is a play glyph at 14px with an "Add action" tooltip; with
  one configured it is `[icon] {name}` at 12px `text_muted`. Hover is the
  single `wash(0.11)` blend - no pills, no plus signs.
- Changes toggle and close as 24px icon buttons on a 2px in-group rhythm
  (14px glyphs, `text_muted`, tooltips + aria labels).

While the sidebar is collapsed (or mid-collapse) the pane at the window's
top-left adds `pane_header_leading_inset` of left padding so its mark and
title start `TITLEBAR_IDENTITY_GAP` past the titlebar cluster (traffic
lights, sidebar toggle, nav, and the "+" slot). Only that one pane (or
top-left tab strip) insets; the value rides the sidebar and titlebar
tweens, so it animates rather than jumping.

## Composer

The placeholder is "Message {Harness}…". The model chip keeps the harness
mark's brand tint, shows the model name without the `provider/` prefix at
NORMAL weight in `text_muted`, and shows reasoning in `text_faint`.

Only the context ring (16px, 1.8px stroke) sits left of the send button
whenever the harness reports a window. Provider/account limits and account
switching live in Settings > Accounts (the sidebar cog); rendering a composer
never polls account usage. Its fill is `text_muted`, turning
`warning` at 75% and `danger` at 90%. Hovering opens the context card:
"Context window" with the percent in mono, a 4px usage bar, then mono
11px lines - `used / window tokens`, `left`, `Auto-compacts at ~N%`
(only when the harness reports the threshold), and session totals
(`in · out · cache`) when available. Pi reports through the Noches Pi
extension (`crates/harness/src/pi/noches-context-usage.ts`), which writes
snapshots the harness polls; `tokens: null` after compaction reads as
"Waiting for context usage", never 0%.

## Tool rows

Transcript tool calls have two looks, chosen in Settings > Appearance >
"Transcript tool rows". **Calm** (T3's work log) is the default; **Tree** is
the earlier BoardUI task tree, kept as-is behind the setting. Both keep the
inline diffs, the group-title shimmer, and the reserved `failed` tag. Subagent spawn
chips are cards in either look.

### Calm (default)

A flat log, no rail, no ribbons, no per-family hue.

- Rows are 24px: a 16px glyph in a 24px cell (`icon_muted`), 6px gap, the
  label at 14px in `text_muted` (verb and detail as one run, files by name),
  then a 16px box holding a 12px chevron (`icon_muted` at 70%). The chevron is
  always present so labels never shift; it turns 90 degrees over 200ms
  (`EASE_TAILWIND`) and an unexpandable row leaves its box empty.
- Interactive rows take an 8px-radius hover plate (`row_hover_fill`, the
  accent surface at 20%) that fades in over `HOVER_FADE`. Rows have 2px side
  padding.
- The group header is the first row of the log: a glyph chosen by what the
  group did (uniform groups wear their tools' glyph, mixed groups the generic
  tool mark), the summary label, and the chevron. Groups sit 8px from the
  text around them (Tree: 12px).
- Open details indent 28px and sit 4px below their row. Output, thought, and
  stat lines are a quiet plate (`detail_panel_fill`, 8px radius, 12/8px
  padding) in the code face at the code size on a 1.625 line, `text_muted`,
  with the 24-line cap and counted tail. Diffs keep the changes pane body,
  clipped to the same radius. Heights are analytic (`calm_detail_height`).
- Thoughts read `Thinking` while streaming and `Thought` once settled, then
  the first line of the reasoning as a preview.
- Nothing animates but the chevron and the live shimmer: rows appear, folds
  and details open instantly.
- Failure keeps the reserved tag: the glyph softens to `theme.danger` at 60%
  and a trailing mono 11px `failed` tag in danger at 0.9 follows the label.
- Live shimmer: T3's `live-tool-shine` on the title of the active group (both
  looks) and on each still-running row (Calm only), a 72px absolute crest
  every 2.2s, `text_muted` to `text`.

### Tree

Transcript tool rows tint the 14px icon by the identity of the action -
`crate::tool_palette::ToolFamily`, the same (dark, light) hue-pair pattern
as `status_palette`. Verbs stay `text_muted` (MEDIUM on card chips),
details stay `text_faint`, and connectors/badges stay neutral.

- Explore (read, search, glob/list, fetch, web search): muted teal
  `0x7cc4bd` dark / `0x2f7f78` light.
- Change (edit, write, patch): muted amber `0xd9b26f` / `0x946514`.
- Delegate (subagent spawn, "Wait for agents"): muted orchid
  `0xc9a0dc` / `0x8a4a9e`.
- Run, Plan/Todo, MCP/Unknown, Thinking: neutral `text_muted` - commands
  are the bulk of the column and keeping them quiet is what makes the
  tinted families legible.

These hues are deliberately desaturated and in different sectors than the
session status hues (sky/indigo/emerald); a tinted tool icon must never be
readable as session state.

Failure is reserved: a failed row keeps its verb/detail colors, the icon
turns `theme.danger`, and a trailing mono 11px `failed` tag in danger at
0.9 opacity follows the detail. The tree connector stays neutral. Running
rows keep the existing live treatment, neutral.

## Subagents

Three surfaces read one selector, `subagents_for(state, chat)`, which
distills a chat's spawn tool parts into `SubagentSummary` rows: id, title
(from the spawn's `description`, else "Agent"), agent type, model, a
four-phase status (Running / Started / Done / Failed), start and finish
instants, and a one-line result tail. `Started` is the honest state for
background spawns - a bare tool result never means Done there - and for
doc-less harnesses like Pi where the call's lifecycle is all we have.
Ordering: running first (oldest first), then finished (newest first).
Status hues come from `SessionState` only: sky equalizer Running,
emerald check Done (neutral once the thread has been opened), danger
triangle Failed, neutral dot Started.

- **Agents tray**: a queue-tray surface stacked above the composer pill
  (same `QUEUE_SIDE_INSET`, rounded top `PANEL_RADIUS`, tucked
  `QUEUE_COMPOSER_OVERLAP` behind the pill, `popover::surface_bg` +
  `theme.border`, frost-aware shadow). Rendered by the composer itself
  on every route, so the transcript's measured bottom clearance already
  covers it - no floating row. When the queue tray is also up, the
  agents tray sits on top of it and tucks behind the queue tray's top
  edge (one continuous stack: agents, queue, pill). The content row is
  32px: 12px left pad, `Agents` 11.5px MEDIUM `text_faint` + mono
  `{done}/{total}`, then 24px pills (no border inside the tray -
  `wash(0.06)` fill, `wash(0.10)` hover): 12px status glyph, truncated
  title at 12px `text_muted`, mono 11px elapsed. A `+N` pill and the
  trailing 24px chevron toggle the Agents tab; 6px right pad. The tray
  shows the latest turn's agents plus anything still running, and
  appears/disappears with `motion::fade_quick`.
- **Agents panel**: `RightSurface::Agents`, one "Agents" tab with the
  `BOT` icon. `Active` (with the sidebar's 6px sky section dot) and
  `Done · N` sections (30px headers like the sidebar's), 44px rows: line 1
  is the 12px status glyph, 8px, 13px `text` title truncating, and the
  mono 11px elapsed right-aligned, all centered on the 18px line; line 2 (16px, starting at the title's x) holds
  the one-line result in 12px `text_muted` truncating with `agent_type
  · model` in mono 11px `text_faint` right-aligned (either part may be
  absent; no summary and no meta collapses the row to 32px). Rows open
  the child thread. There is no bulk stop - the engine exposes no
  subagent-interrupt call.
- **Sidebar children**: under selected or pane-open cards with running
  agents only, up to three 22px rows rendered as extra lines INSIDE the
  card after the context line (sharing the card's wash and radius; no tree stubs,
  no hairlines). Each row: 12px status glyph at the card's text-start x,
  6px gap, 12px `text_muted` title truncating, mono 11px `text_faint`
  elapsed flush to the card's right edge. A `+N more` row (no glyph,
  indented to the title start) selects the chat and opens the Agents tab.
  2px between the context line and the first child, 4px bottom pad. Child clicks
  stop propagation and open the agent thread. Finished agents never
  nest; card height animates via the disclosure tween.

Opening an agent opens its thread, not a dead end: real sub docs go
through `add_subagent_surface`; doc-less results (Pi) render a frozen
single-entry snapshot titled from the spawn, so the row never opens an
empty transcript.
