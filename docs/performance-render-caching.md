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
