//! Self-ticking elapsed label (T3's `AgentElapsed`).
//!
//! The label is its own view: a live clock wakes only itself, at the instant
//! its text would change, and never invalidates the surface that hosts it. A
//! settled clock holds no task. Hosts describe the clock with a [`ClockSpec`]
//! and key it with a stable id (one per surface, not per frame).

use chrono::{DateTime, TimeDelta, Utc};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, Hsla, IntoElement, Render, RenderOnce, SharedString, Task,
    Window, div, px,
};

/// What a clock shows: elapsed from `started`, running to now while `active`,
/// frozen at `finished` once settled. A settled clock with no observed finish
/// shows nothing rather than a guessed duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClockSpec {
    pub started: Option<DateTime<Utc>>,
    pub finished: Option<DateTime<Utc>>,
    pub active: bool,
}

impl ClockSpec {
    /// `45s` / `2m` / `1h 4m`, or `None` while nothing can be said honestly.
    pub fn label(&self, now: DateTime<Utc>) -> Option<String> {
        let started = self.started?;
        let end = if self.active { now } else { self.finished? };
        Some(crate::shell::format_working_elapsed(
            end.signed_duration_since(started).num_seconds(),
        ))
    }

    /// The next instant the label text changes; `None` when it never will
    /// again (settled, or no start).
    pub fn next_change(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.active {
            return None;
        }
        Some(crate::state::next_elapsed_label_change(self.started?, now))
    }
}

/// Mono 11px elapsed text in `color`, ticking on its own. `key` must be unique
/// per surface and stable across frames.
pub fn elapsed_label(key: impl Into<SharedString>, spec: ClockSpec, color: Hsla) -> AnyElement {
    ElapsedLabel {
        key: key.into(),
        spec,
        color,
    }
    .into_any_element()
}

#[derive(IntoElement)]
struct ElapsedLabel {
    key: SharedString,
    spec: ClockSpec,
    color: Hsla,
}

impl RenderOnce for ElapsedLabel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { key, spec, color } = self;
        let view = window.with_global_id(key.into(), |id, window| {
            window.with_element_state(id, |previous: Option<Entity<ClockView>>, _| {
                let view = previous.unwrap_or_else(|| cx.new(|cx| ClockView::new(spec, color, cx)));
                view.update(cx, |view, cx| view.set(spec, color, cx));
                (view.clone(), view)
            })
        });
        view
    }
}

struct ClockView {
    spec: ClockSpec,
    color: Hsla,
    _tick: Option<Task<()>>,
}

impl ClockView {
    fn new(spec: ClockSpec, color: Hsla, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            spec,
            color,
            _tick: None,
        };
        view.arm(cx);
        view
    }

    fn set(&mut self, spec: ClockSpec, color: Hsla, cx: &mut Context<Self>) {
        if self.spec == spec && self.color == color {
            return;
        }
        let respin = self.spec != spec;
        self.spec = spec;
        self.color = color;
        if respin {
            self.arm(cx);
        }
        cx.notify();
    }

    /// One sleeping task per live clock; replaced when the spec changes and
    /// dropped (cancelled) once it settles.
    fn arm(&mut self, cx: &mut Context<Self>) {
        self._tick = self.spec.active.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    let Ok(next) = this.update(cx, |view, _| view.spec.next_change(Utc::now()))
                    else {
                        break;
                    };
                    let Some(next) = next else { break };
                    // Round up to whole ms so a truncated wait cannot wake
                    // just before the deadline and burn a frame re-arming.
                    let wait = (next - Utc::now()).max(TimeDelta::milliseconds(16));
                    cx.background_executor()
                        .timer(wait.to_std().unwrap_or_default())
                        .await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
    }
}

impl Render for ClockView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = crate::theme::Theme::of(cx);
        div()
            .flex_none()
            .font_family(theme.font_mono.clone())
            .text_size(px(11.0))
            .text_color(self.color)
            .child(SharedString::from(
                self.spec.label(Utc::now()).unwrap_or_default(),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_000 + secs)
    }

    #[test]
    fn live_clock_runs_to_now_and_wakes_on_the_next_label_change() {
        let spec = ClockSpec {
            started: Some(t(0)),
            finished: None,
            active: true,
        };
        assert_eq!(spec.label(t(45)).as_deref(), Some("45s"));
        assert_eq!(spec.label(t(125)).as_deref(), Some("2m"));
        // Seconds tick each second; past a minute only the minute rolls.
        assert_eq!(spec.next_change(t(45)), Some(t(46)));
        assert_eq!(spec.next_change(t(125)), Some(t(180)));
    }

    #[test]
    fn settled_clock_freezes_at_finish_and_never_wakes() {
        let spec = ClockSpec {
            started: Some(t(0)),
            finished: Some(t(75)),
            active: false,
        };
        assert_eq!(spec.label(t(10_000)).as_deref(), Some("1m"));
        assert_eq!(spec.next_change(t(80)), None);
    }

    #[test]
    fn settled_without_an_observed_finish_says_nothing() {
        let spec = ClockSpec {
            started: Some(t(0)),
            finished: None,
            active: false,
        };
        assert_eq!(spec.label(t(10)), None);
        let unstarted = ClockSpec {
            active: true,
            ..ClockSpec::default()
        };
        assert_eq!(
            (unstarted.label(t(10)), unstarted.next_change(t(10))),
            (None, None)
        );
    }
}
