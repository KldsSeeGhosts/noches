# Render attribution

Set `NOCHES_PERF_TRACE=1` before launching the headed app (including
`scripts/dev-demo.sh`). It enables GPUI's bounded frame collector and logs
each completed frame once, sampled by a one-second collector. With the variable
unset, no collector or timer runs.

`draw_us` measures CPU scene construction only, **not** GPU time, native
presentation, or missed vsyncs. Counters are cumulative on the UI thread:
transcript renders, transcript scene hits/misses, full subagent selector scans
and shared-cache hits, and source bytes hashed for highlighting.
Scene hits compare an entity's render count before layout and after paint;
they count actual reuse, not just attempts to mount a cached view.

For a warm comparison, open and close the model picker over unchanged content
50 times in solo and split layouts. The settled transcript should stop rendering
on picker-only frames; subagent scans and highlight hashed bytes should stay
constant after warmup. Real content/status/selection/geometry updates must still
repaint. Active route fades intentionally do not reuse transcript scenes.

Use a release build and identical isolated seeded data for timing comparisons.
These logs establish work avoided; they do not establish smooth 120 Hz native
presentation. Light/dark intermediate-fade pixels and native Metal/presentation
timings remain separate QA.

## Headless work-avoidance evidence

Linux UI tests compare the uncached and cached paths using the same data.
Counters are enabled in test builds without scheduling a trace collector.

| Controlled workload | Uncached work | Warm cached work |
| --- | ---: | ---: |
| 50 parent invalidations, solo transcript | 100 renders | 0 renders, 100 scene hits |
| Same, two transcripts | 200 renders | 0 renders, 200 scene hits |
| Same, four transcripts | 400 renders | 0 renders, 400 scene hits |
| 50 open/close-equivalent cycles, three subagent consumers, 21 MB-class history | 300 full-history scans | 0 scans, 300 shared hits |
| 50 highlight requests, 100 / 1,000 / 10,000 lines | 74,950 / 749,950 / 7,499,950 bytes hashed | 0 bytes hashed |

The test platform draws automatically on notifications as well as on the
explicit draw call, hence 100 drawn frames for 50 parent invalidations.
Subagent results cover both zero-agent and live/finished-agent histories.
The highlight baseline is the former per-request key construction; ready and
pending slots both short-circuit now. These are attribution counts, not native
picker interactions, frame-time improvements, or presentation measurements.

Validation on 2026-10-03:

- `linux-test.sh <worktree> test -p zeron-ui --lib --locked --config profile.test.package.zeron-ui.opt-level=1 -- --nocapture`:
  1,437 passed, zero failed. The UI-specific profile avoids stale UI artifacts
  from another worktree in the shared target directory.
- Mac: `CARGO_TARGET_DIR=/Volumes/DevDrive/AiStack/noches-wt/target-shared CARGO_BUILD_JOBS=6 cargo check -p zeron-ui`:
  passed, with existing Objective-C macro configuration and unused-mut warnings.
- `cargo fmt -p zeron-ui` run; unrelated pre-existing formatting drift was
  preserved rather than included in this workstream.

The supplied `linux-test.sh` pipes Cargo output through `tail` without
`pipefail`, so its exit status alone does not establish success. The results
above were checked against Cargo's explicit test-result lines.

Regression coverage includes unchanged scene hits, status and stale-session
updates without row changes, focus/selection dispatch after reuse, intermediate
inherited fades, fade-band invalidation, child-document cache dependencies,
finish/restart semantics, and immutable highlight source/language/query keys.
Native light/dark intermediate-fade pixel QA, GPU/presentation timing, and
Windows verification remain outstanding. No native fixture suite, fence-line
virtualization, or incremental spawn index was added.
