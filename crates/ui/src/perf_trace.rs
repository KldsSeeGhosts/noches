//! Small, opt-in attribution counters. Draw timings are CPU scene construction,
//! not GPU execution or presentation latency. Nothing is scheduled by default.

use std::{cell::RefCell, collections::HashMap, sync::OnceLock, time::Duration};

use gpui::{App, EntityId};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Counters {
    pub transcript_renders: u64,
    pub transcript_cache_hits: u64,
    pub transcript_cache_misses: u64,
    pub subagent_scans: u64,
    pub subagent_cache_hits: u64,
    pub highlight_hash_bytes: u64,
}

thread_local! {
    static COUNTERS: RefCell<Counters> = RefCell::default();
    static RENDERS: RefCell<HashMap<EntityId, u64>> = RefCell::default();
}

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("NOCHES_PERF_TRACE").is_ok_and(|v| v == "1"))
}

fn counting() -> bool {
    cfg!(test) || enabled()
}

pub(crate) fn transcript_render(id: EntityId) {
    if counting() {
        COUNTERS.with(|c| c.borrow_mut().transcript_renders += 1);
        RENDERS.with(|r| *r.borrow_mut().entry(id).or_default() += 1);
    }
}

pub(crate) fn render_count(id: EntityId) -> u64 {
    RENDERS.with(|r| r.borrow().get(&id).copied().unwrap_or(0))
}

pub(crate) fn scene_reuse(id: EntityId, before: u64) {
    if counting() {
        let hit = render_count(id) == before;
        COUNTERS.with(|c| {
            let mut c = c.borrow_mut();
            if hit {
                c.transcript_cache_hits += 1;
            } else {
                c.transcript_cache_misses += 1;
            }
        });
    }
}

pub(crate) fn subagent_scan() {
    if counting() {
        COUNTERS.with(|c| c.borrow_mut().subagent_scans += 1);
    }
}

pub(crate) fn subagent_cache_hit() {
    if counting() {
        COUNTERS.with(|c| c.borrow_mut().subagent_cache_hits += 1);
    }
}

pub(crate) fn highlight_hash(bytes: usize) {
    if counting() {
        COUNTERS.with(|c| c.borrow_mut().highlight_hash_bytes += bytes as u64);
    }
}

pub(crate) fn snapshot() -> Counters {
    COUNTERS.with(|c| *c.borrow())
}

pub(crate) fn init(cx: &mut App) {
    if !enabled() {
        return;
    }
    gpui::set_frame_trace_enabled(true);
    let mut frames = gpui::FrameTimingCollector::new();
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let counters = snapshot();
            for frame in frames.collect_unseen() {
                tracing::warn!(
                    window = ?frame.window_id,
                    draw_us = frame.draw_duration().as_micros() as u64,
                    dirty_to_draw_us = frame.dirty_to_draw_duration().map(|d| d.as_micros() as u64),
                    invalidations = frame.invalidations,
                    ?counters,
                    "noches perf frame (CPU draw, not presentation)"
                );
            }
        }
    })
    .detach();
}
