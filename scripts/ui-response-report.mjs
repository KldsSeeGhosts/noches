#!/usr/bin/env node
// Summarize headed fixture output without mislabeling CPU draw as presentation.
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const files = process.argv.slice(2);
if (!files.length) throw new Error("Usage: node scripts/ui-response-report.mjs RESPONSE_JSON [...]");
const percentile = (values, fraction) => {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  return sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))] : null;
};
const stats = (values) => ({
  p50_us: percentile(values, .5),
  p95_us: percentile(values, .95),
  max_us: percentile(values, 1),
});
for (const file of files) {
  const run = JSON.parse(readFileSync(file, "utf8"));
  let previous = run.phases[0].samples[0].panels;
  const phases = run.phases.map((phase) => {
    const panel = phase.phase.startsWith("left") || phase.phase.startsWith("right");
    const side = phase.phase.startsWith("right") ? "right" : "left";
    const target = phase.samples.at(-1).panels[`${side}_target`];
    const moving = panel ? phase.samples.filter(s =>
      s.panels[side] > Math.min(previous[side], target) &&
      s.panels[side] < Math.max(previous[side], target)
    ).length : undefined;
    if (panel) {
      const last = phase.samples.at(-1).panels;
      assert.equal(last[side], last[`${side}_target`], `${phase.phase} failed to settle`);
    } else {
      assert(phase.samples.every(s => s.position.distance <= 70 || !s.position.pinned),
        `${phase.phase}: user lost scroll ownership`);
      assert(phase.samples.every(s => s.position.distance <= 320 || s.position.jump),
        `${phase.phase}: jump control vanished away from bottom`);
    }
    previous = phase.samples.at(-1).panels;
    return {
      phase: phase.phase,
      frames: phase.frames.length,
      cpu_draw: stats(phase.frames.map(f => f.draw_us)),
      dirty_to_draw: stats(phase.frames.map(f => f.dirty_to_draw_us)),
      input_dispatch: stats(phase.samples.map(s => s.input_us)),
      rail_builds: phase.after.rail_builds - phase.before.rail_builds,
      rail_build_us: phase.after.rail_build_us - phase.before.rail_build_us,
      row_renders: phase.after.row_renders - phase.before.row_renders,
      intermediate_panel_samples: moving,
    };
  });
  console.log(JSON.stringify({file, turns: run.turns, panel_ms: run.panel_ms,
    scope: run.timing_scope, phases}, null, 2));
}
