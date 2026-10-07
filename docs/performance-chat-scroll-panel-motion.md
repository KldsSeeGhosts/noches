# Chat scrolling and panel motion

This follow-up targets transcript wheel input and the left/right panel
transitions, not engine throughput. The integration base is `dev` at
`6dc76ff4`; measurement-only commit `96d524ce` retains the original behavior
and can be built as the comparison baseline.

## Changes

- The outline previously cloned the entire rich transcript, regenerated prompt
  previews, and searched the row array separately for every prompt on every
  scroll/hover render. It now borrows the correct session's source and builds a
  linear row-id index once per content revision. A retained model holds only
  prompt/reply previews (160/200 characters) and row identities. Scroll, resize,
  typography and hover frames reuse it; replay, echoes and session changes
  invalidate it. Reply lookup no longer rescans suffixes, and preview
  normalization stops at the displayed prefix.
- Wheel/touch input now cancels the outline's 500-ms navigation task
  synchronously. Otherwise its next timer tick could move the viewport after
  the user's gesture. Deferred ListState reads and existing own-turn/runway
  ownership rules are unchanged.
- Panel motion default/reset is the authored 200 ms rather than 0 ms.
  **Saved `panelAnimationMs: 0` remains Off.** There is no safe way to distinguish
  a saved old default from a deliberate opt-out. Existing Off profiles should
  choose 200 ms or Reset under Settings → Appearance → Panel animations.
  Reduced motion remains authoritative. The design, split layout and Calm
  tool-detail behavior are unchanged.

## Native CPU evidence

Two clean, sequential comparison pairs on a Mac Studio / M5 Max, macOS 27.0.1:
1,800 exchanges, 14,400 virtual rows, mixed prose/code/tables/completed tools,
40 sidebar sessions, a 1320×880 logical window. Both binaries use 200-ms panel
timing. Each scroll phase dispatches 180 real GPUI list wheel events at a
nominal 16-ms interval; reversals change direction every 15 events.

The following ranges are the per-run results, not pooled percentiles:

| Native replay phase | Baseline draw median | Changed draw median | Baseline draw p95 | Changed draw p95 |
| --- | ---: | ---: | ---: | ---: |
| Warm downward scrolling | 13.83–13.99 ms | 2.96–3.42 ms | 14.87–14.91 ms | 4.35–5.61 ms |
| Direction reversals | 14.10–14.39 ms | 3.39–3.69 ms | 15.31–15.41 ms | 4.81–6.02 ms |

Warm/reversal phases each rebuilt the outline 180 times before, costing
1.91–1.98 seconds of attributed CPU work per phase. Afterwards they rebuilt it
zero times. Both versions retained user scroll ownership and the jump control.
The second pair's item/offset endpoints match exactly. The first pair ends at
row 7199/offset 0 before versus row 7198/offset 30 after, with estimated
bottom-distance values differing by 3 px; do not treat it as exact pixel
alignment. [Per-run evidence](performance-chat-scroll-panel-motion-results.json)
preserves frame counts, metrics, endpoints and raw-result hashes.

These are **CPU `Window::draw` and invalidation-to-draw measurements**, not
physical input-to-display latency, GPU execution, presentation timestamps or
FPS. The headed fixture uses the system allocator rather than the distributed
Mac application's mimalloc. Native frame callbacks can produce different frame
counts, so all-frame cold-pass medians are not used for the headline comparison.
Normal desktop workload was not isolated. The comparison script samples
compiler activity each second and rejects contaminated runs; earlier overlapping
repeat runs are excluded. Smaller 600-turn exploratory runs were noisier and
are not used to claim a universal speedup.

Immutable compared binaries, SHA-256:

- Baseline: `86860d492caa4034f6442c0e9c460c5e880246ad50e0fc419b9ff80a06850ebb`
- Changed: `ab4b289f0d9c3c619317356fe951510121181582619375079109e34441f0346d`

The changed comparison binary predates fixture-only changes that move
synchronous screenshot readbacks outside active transitions/input sustain
and verify each appearance in a separate process.
The production UI changes and measured event replay are the same.

## Reproduce

Build the measurement commit and this PR in separate checkouts, then copy each
executable outside its build target before comparing. Do not build during runs.

```sh
cargo build -p zeron-ui --release --locked \
  --example ui-response-fixture --features ui-response-fixture
node scripts/ui-response-compare.mjs BASELINE_BINARY CHANGED_BINARY RESULTS_DIR 1800
node scripts/ui-response-report.mjs RESULTS_DIR/baseline-1800-1/response.json \
  RESULTS_DIR/after-1800-1/response.json
```

`NOCHES_RESPONSE_RUNS=1` selects one comparison pair. Each result includes a
compiler-contention audit. Fixture profiles are temporary, no engine is
bootstrapped, and no real agent message or installed application data is used.

Run verification/readbacks separately from timings:

```sh
NOCHES_PERF_TRACE=1 NOCHES_RESPONSE_VERIFY=1 CHANGED_BINARY QA_RESULTS_DIR
NOCHES_PERF_TRACE=1 NOCHES_RESPONSE_VERIFY=1 NOCHES_RESPONSE_LIGHT=1 \
  CHANGED_BINARY QA_LIGHT_RESULTS_DIR
NOCHES_PERF_TRACE=1 NOCHES_RESPONSE_INTERACTIVE=1 CHANGED_BINARY INTERACTIVE_DIR
```

Additional knobs: `NOCHES_RESPONSE_TURNS`, `NOCHES_RESPONSE_PANEL_MS`,
`NOCHES_RESPONSE_REDUCED=1`, `NOCHES_RESPONSE_LIGHT=1`. Interactive mode leaves
the isolated window open without scripted input. Native verification exercises
rail interruption, streaming while reading history, interrupted panel reversal,
light/dark endpoint captures and reduced/instant snaps. Synchronous Metal
readbacks are outside measured phases.

## Validation and remaining QA

`cargo test -p zeron-ui --lib --locked`: 1,504 passed, two existing skips
(comparative subagent benchmark and installed Anthropic font geometry).
The new cross-platform cached-list test dispatches actual wheel input and
verifies immediate rail-task cancellation. Additional regressions cover
outline reuse/invalidation, fixed-session sources, echo acknowledgement,
bounded Unicode previews and preservation of explicit animation preferences.
Existing pane/workspace/split regressions pass.

Native replay passed motion/ownership assertions. In-process live dark-to-light
switching logged a nonfatal GPUI App borrow warning: existing
`appearance::sync_ns_appearance` calls AppKit's `setAppearance:` while the App is
borrowed, and GPUI's synchronous appearance callback tries to borrow it again.
This PR does not change that production path. Light/dark fresh-process
verification passed separately with no runtime errors. Settled transcript,
left-collapsed and right-open native endpoint readbacks were inspected in both
appearances; these are GPUI image readbacks, not OS screenshots or transition
recordings.

The connected Computer Use tool now works without changing macOS permissions.
A dark-appearance desktop smoke check used OS-level scroll actions in both
directions, Cmd+B for the left sidebar, and the visible right-sidebar toggle.
Updated OS screenshots confirm transcript movement and both panels' open/closed
endpoints. The stale terminal-only tool connection was stopped without accepting
its approval request. This is a smoke check, not an exact scroll-displacement,
continuous animation recording, physical trackpad momentum or input-to-display
measurement. Those checks and Mac split-view manual input remain unverified,
so the PR remains draft; this is not a claim that every reported stall has been
eliminated. Linux and Windows native behavior have not been verified locally.
