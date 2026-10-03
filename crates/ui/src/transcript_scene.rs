//! Definite-size scene boundary for stationary transcripts. Inherited paint
//! scopes are not part of GPUI's key: route motion must stay uncached.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Element, Entity, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, StyleRefinement, Window,
};

use crate::{perf_trace, transcript::Transcript};

pub(crate) fn reusable(
    route_active: bool,
    departing: bool,
    geometry_ready: bool,
    opacity: f32,
) -> bool {
    !route_active && !departing && geometry_ready && opacity == 1.0
}

pub(crate) fn scene(transcript: Entity<Transcript>, reuse: bool) -> AnyElement {
    TranscriptScene { transcript, reuse }.into_any_element()
}

thread_local! {
    static REUSE_ALLOWED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

fn with_reuse_scope<R>(allow: bool, f: impl FnOnce() -> R) -> R {
    REUSE_ALLOWED.with(|scope| {
        struct Restore<'a>(&'a std::cell::Cell<bool>, bool);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let _restore = Restore(scope, scope.replace(scope.get() && allow));
        f()
    })
}

/// An ancestor opacity animation must opt out all transcript descendants,
/// including split panes. Scope lives only through this element's traversal.
pub(crate) fn scope(allow: bool, child: impl IntoElement) -> impl IntoElement {
    SceneScope {
        allow,
        child: child.into_any_element(),
    }
}

struct TranscriptScene {
    transcript: Entity<Transcript>,
    reuse: bool,
}

impl IntoElement for TranscriptScene {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TranscriptScene {
    type RequestLayoutState = (AnyElement, u64);
    type PrepaintState = ();
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let before = if cfg!(test) || perf_trace::enabled() {
            perf_trace::render_count(self.transcript.entity_id())
        } else {
            0
        };
        let mut child = if self.reuse && REUSE_ALLOWED.with(std::cell::Cell::get) {
            self.transcript
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        } else {
            self.transcript.clone().into_any_element()
        };
        (child.request_layout(window, cx), (child, before))
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        state.0.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        state.0.paint(window, cx);
        perf_trace::scene_reuse(self.transcript.entity_id(), state.1);
    }
}

struct SceneScope {
    allow: bool,
    child: AnyElement,
}

impl IntoElement for SceneScope {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SceneScope {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            with_reuse_scope(self.allow, || self.child.request_layout(window, cx)),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        with_reuse_scope(self.allow, || self.child.prepaint(window, cx));
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        with_reuse_scope(self.allow, || self.child.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fully_settled_route_paint_is_reusable() {
        assert!(reusable(false, false, true, 1.0));
        for opacity in [0.0, 0.25, 0.5, 0.75, 0.999] {
            assert!(!reusable(false, false, true, opacity));
        }
        assert!(!reusable(true, false, true, 1.0));
        assert!(!reusable(false, true, true, 1.0));
        assert!(!reusable(false, false, false, 1.0));
    }

    #[test]
    fn inherited_scope_cannot_be_reenabled_by_a_child_or_leak_to_another_window() {
        assert!(REUSE_ALLOWED.with(std::cell::Cell::get));
        with_reuse_scope(false, || {
            assert!(!REUSE_ALLOWED.with(std::cell::Cell::get));
            with_reuse_scope(true, || assert!(!REUSE_ALLOWED.with(std::cell::Cell::get)));
        });
        assert!(REUSE_ALLOWED.with(std::cell::Cell::get));
    }
}
