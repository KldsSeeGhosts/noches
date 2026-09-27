# Noches mobile: the control plane in your pocket

The phone follows [control-plane.md](control-plane.md). It answers the same
questions at a glance: what each session is doing, where it runs (project,
branch), which agent runs it, and whether it needs you. This doc covers only
the phone's own translation of those rules. Where the two docs conflict,
control-plane.md wins on color and meaning, and this doc wins on sizes.

## Rules

1. **Color encodes state or identity, never decoration.** Status hues come
   from `SessionState` in `Theme/ControlPlane.swift`, which mirrors
   `crates/ui/src/status_palette.rs` exactly:
   - Working is sky `#7dd3fc` (dark) and `#0284c7` (light).
   - Awaiting input is indigo `#a5b4fc` / `#4f46e5`.
   - Completed but unseen is emerald `#6ee7b7` / `#059669`.
   - Failed is the theme's `danger`.
   - Queued and Idle are neutral.

   Tool-family tints mirror `tool_palette.rs`:
   - Explore is teal `#7cc4bd` / `#2f7f78`.
   - Change is amber `#d9b26f` / `#946514`.
   - Delegate is orchid `#c9a0dc` / `#8a4a9e`.
   - Everything else is `textMuted`.

   Project monograms mirror `project_icon.rs`: an 8-tone palette picked by an
   FNV-1a hash of the space path (`"home"` when there is no project). Harness
   marks keep their brand tint. Do not use any other hard-coded hex values.
2. **No mascots.** Bot avatars are gone. Identity comes from the project badge
   and the harness mark.
3. **Metadata is monospace.** Branch, elapsed time, token counts, and model
   reasoning use Geist Mono at 12pt. Titles use Geist.
4. **Weight is hierarchy.** Titles are regular weight, or medium when the
   session needs you. Section and status labels are medium.
5. **Sizes are phone sizes.** The body is 17pt, titles on cards are 16pt,
   secondary text is 13 to 14pt, and metadata is 12pt mono. Nothing
   interactive is smaller than 44pt. Nothing on screen is smaller than 11pt.
6. **Motion is quiet.** Use `Motion` timings. Status changes cross-fade;
   trays slide and fade. There is no bouncing.

## Home

```
 ▦ noches                                   (…)
 🖥 Studio ⌄                         ● Connected
 ▢ All projects ⌄                     6 sessions

 ● Needs you                                   2
▌[N] noches               ◌ Awaiting input
   Review the authentication flow
   Allow the migration command?          (preview, needs-you only)
   ⑂ fix/session-auth                          ✳
 ● Running                                     1
 [N] noches                        ≋ Working 2m
   Polish the desktop sidebar
   ⑂ design/sidebar                            ✳
 Recent
 [W] website                                   3h
   Pricing page copy pass
   ⑂ main                                      ◎

 (≡)  ( ⌕ Search                    )  (✎)
```

- Sections are **Needs you** (awaiting input or failed, with an indigo dot),
  **Running** (working or queued, with a sky dot), and **Recent** (everything
  else, no dot). Recent appears as a titled section only when a section above
  it exists. A header is a 13pt medium `textMuted` label, a 6pt dot, and a mono
  12pt `textFaint` count on the right. There is 20pt of space above each
  header.
- Cards have three lines plus an optional preview. Text starts at the badge's
  left edge, as in T3.
  - Line 1: an 18pt project badge (favicon, or monogram with 5pt corners),
    then the project name at 13pt medium `textMuted`. The status slot sits on
    the right. It shows a 12pt glyph and a 13pt medium label in the state
    color. Working adds mono elapsed time (`2m`, `1h 4m`) from
    `Session.startedAt`. Settled rows show the mono relative time in
    `textFaint`.
  - Line 2: the title at 16pt, in `text` at 0.9 opacity. When the session
    needs you, it is medium weight at full opacity. At most two lines.
  - Preview: shown only when the session needs you. It is the last message
    preview at 14pt `textMuted`, at most two lines. This is the question the
    agent is asking.
  - Line 3: a branch glyph and the branch in mono 12pt `textFaint`,
    truncating. On the right is the harness mark at 14pt in its brand tint.
  - Needs-you cards have a 3pt rounded bar in the state color inside the list
    padding (x = 8).
  - Cards are separated by a 0.5pt hairline inset to the text start. Pressing
    a card shows the `elementHover` wash.
- Swipe trailing archives or restores. The context menu offers Rename,
  Archive, and **Mark as read** (shown only for completed-unseen sessions).
- The bottom bar (filter, search capsule, compose) stays. Filter scopes are
  All, Needs you, Running, and Archived. Compose preselects the filtered
  project.
- Empty states are one 17pt medium line and one 14pt `textMuted` hint.

## Session

- **Header:** the back button, then a centered two-line title. Line 1 is the
  harness mark (12pt, brand tint) and the title at 15pt semibold. Line 2 is a
  12pt project badge and the mono 12pt `textFaint` context `{project}:{branch}`.
  The trailing glass group holds Changes, Files, and More.
- **Transcript:** this is the shared native transcript.
  - Tool icons tint by family.
  - A failed tool turns its icon `danger` and adds a mono `failed` tag.
  - Reasoning shows as a collapsed neutral "Thinking" tool row.
  - Images render inline with 14pt corners, and tap opens them full screen.
  - Seen: opening a session, and new messages arriving while it is visible
    and settled, send `Mutate {op: "markChatSeen"}`. This is how emerald
    clears everywhere.
- **Activity line:** while working or awaiting input, a single 13pt row sits
  above the tray stack. Working shows a sky equalizer glyph, "Working", and
  mono elapsed time. Awaiting input shows an indigo glyph and "Awaiting
  input". There is no other status text near the composer.
- **Tray stack**, from top to bottom: Agents, Queue, then the composer pill.
  Trays use `surfaceRaised` with 16pt top corners and a `border` hairline, and
  tuck 14pt behind the element below them.
  - **Agents tray:** "Agents" at 12pt medium `textFaint` and a mono
    `{done}/{total}` count, then horizontally scrolling 32pt pills. A pill has a
    `wash` fill, a status glyph, a 13pt `textMuted` title, and mono elapsed
    time. Tapping a pill pushes the subagent's transcript, which is read-only.
    The tray shows the latest turn's agents plus any agent still running.
  - **Queue tray:** "Queued · N" with a collapse chevron. Rows are 14pt
    `textMuted` and at most two lines. Each row has a menu with Send now (or
    Steer when the harness supports it), Edit, and Remove. Swiping a row
    removes it.
- **Composer pill:** Liquid Glass with 26pt corners.
  - Row 1 is the editor. The placeholder is "Message {Harness}…" when the
    session is idle and "Queue a follow-up…" when it is running.
  - Row 2 holds, from left to right:
    - the attach button (a 32pt `+`, for photos);
    - the model chip: the harness mark in its tint, the model name without the
      `provider/` prefix in `textMuted`, and reasoning in `textFaint`. Tapping
      it opens a menu of the catalog's models and reasoning levels, which saves
      through `Mutate setChatConfig`;
    - a spacer;
    - the context ring, 18pt with a 2pt stroke. Its fill is `textMuted`,
      turning `warning` at 75% and `danger` at 90%. Tapping it opens the
      context card: "Context window", the percent in mono, a 4pt bar, then
      `used / window`, `left`, `Auto-compacts at ~N%`, and session totals;
    - the 36pt send button. It morphs to Stop when the agent is running and
      the draft is empty. Stop uses a neutral fill, not red.
- Attachments show as 56pt rounded thumbnails above the editor inside the pill,
  each with an x button.

## New session

A sheet with, in order:

- the project (a list with badges, "No project · home folder" first);
- the agent (a horizontal row of brand-mark cards);
- the model and reasoning (a menu and chips);
- a first-message editor.

The primary button is "Start session". It creates the chat and, when the
message is not empty, sends it in the same step, then pushes the session.
