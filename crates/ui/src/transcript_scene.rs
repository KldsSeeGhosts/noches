//! Definite-size scene boundary for stationary transcripts. Inherited paint
//! scopes are not part of GPUI's key: route motion must stay uncached.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Element, Entity, EntityId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, StyleRefinement, Window,
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
    let id = transcript.entity_id();
    let child = if reuse {
        transcript
            .cached(StyleRefinement::default().size_full())
            .into_any_element()
    } else {
        transcript.into_any_element()
    };
    if cfg!(test) || perf_trace::enabled() {
        TracedScene { id, child }.into_any_element()
    } else {
        child
    }
}

struct TracedScene {
    id: EntityId,
    child: AnyElement,
}

impl IntoElement for TracedScene {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TracedScene {
    type RequestLayoutState = u64;
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
    ) -> (LayoutId, u64) {
        let before = perf_trace::render_count(self.id);
        (self.child.request_layout(window, cx), before)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut u64,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        before: &mut u64,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        perf_trace::scene_reuse(self.id, *before);
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
}
