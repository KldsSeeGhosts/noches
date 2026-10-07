// Included in transcript::tests so tests can verify actual retained state.
mod scene_regressions {
    use super::*;
    use gpui::TestAppContext;

    struct Surface {
        transcripts: Vec<Entity<Transcript>>,
        reuse: bool,
        opacity: f32,
    }

    impl Render for Surface {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let children = div()
                .size_full()
                .flex()
                .children(self.transcripts.iter().map(|transcript| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .child(crate::transcript_scene::scene(
                            transcript.clone(),
                            self.reuse,
                        ))
                }));
            div()
                .size_full()
                .opacity(self.opacity)
                .child(crate::transcript_scene::scope(
                    self.opacity == 1.0,
                    children,
                ))
        }
    }

    fn setup(cx: &mut TestAppContext) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        cx.update(|cx| {
            gpui_base::init(cx);
            cx.set_global(Theme::dark());
            crate::settings::init(Default::default(), dir.path(), cx);
        });
        dir
    }

    fn settled(state: Entity<AppState>, fixed: bool, cx: &mut Context<Transcript>) -> Transcript {
        let mut transcript = if fixed {
            Transcript::for_session(state, "chat".into(), cx)
        } else {
            Transcript::new(state, cx)
        };
        transcript.rail_enabled = false;
        transcript.pinned = false;
        transcript.sync(cx);
        transcript.refresh_chrome(cx);
        transcript
    }

    #[gpui::test]
    fn warm_solo_and_split_scene_frames_skip_transcript_render(cx: &mut TestAppContext) {
        let _dir = setup(cx);
        for count in [1, 2, 4] {
            let (surface, visual) = cx.add_window_view(|_, cx| {
                let state = cx.new(|_| {
                    let mut state = AppState::new();
                    state.selected_chat = Some("chat".into());
                    state.transcript_replayed = true;
                    state
                });
                let transcripts = (0..count)
                    .map(|_| cx.new(|cx| settled(state.clone(), count > 1, cx)))
                    .collect();
                Surface {
                    transcripts,
                    reuse: false,
                    opacity: 1.0,
                }
            });
            let render_frames = |reuse, n, visual: &mut gpui::VisualTestContext| {
                surface.update(visual, |surface, cx| {
                    surface.reuse = reuse;
                    cx.notify();
                });
                visual.update(|window, cx| window.draw(cx).clear());
                let before = crate::perf_trace::snapshot();
                for _ in 0..n {
                    surface.update(visual, |_, cx| cx.notify());
                    visual.update(|window, cx| window.draw(cx).clear());
                }
                let after = crate::perf_trace::snapshot();
                (
                    after.transcript_renders - before.transcript_renders,
                    after.transcript_cache_hits - before.transcript_cache_hits,
                )
            };
            let baseline = render_frames(false, 50, visual);
            let cached = render_frames(true, 50, visual);
            assert!(baseline.0 >= count * 50);
            assert_eq!(cached.0, 0);
            assert!(cached.1 >= count * 50);
            eprintln!(
                "scene evidence: panes={count}, 50 warm invalidations: renders {} -> {}, hits={}",
                baseline.0, cached.0, cached.1
            );
        }
    }

    #[gpui::test]
    fn cached_status_changes_and_staleness_repaint_without_row_changes(cx: &mut TestAppContext) {
        let _dir = setup(cx);
        let (surface, visual) = cx.add_window_view(|_, cx| {
            let state = cx.new(|_| {
                let mut state = AppState::new();
                state.selected_chat = Some("chat".into());
                state.transcript_replayed = true;
                state
            });
            Surface {
                transcripts: vec![cx.new(|cx| settled(state, false, cx))],
                reuse: true,
                opacity: 1.0,
            }
        });
        visual.update(|window, cx| window.draw(cx).clear());
        let transcript = surface.read_with(visual, |surface, _| surface.transcripts[0].clone());
        let state = transcript.read_with(visual, |this, _| this.state.clone());
        let revision = state.read_with(visual, |state, _| state.transcript_revision);
        for stale in [false, true] {
            let before = crate::perf_trace::render_count(transcript.entity_id());
            state.update(visual, |state, cx| {
                let now = chrono::Utc::now();
                state.set_sessions(vec![zeron_proto::Session {
                    chat_id: "chat".into(),
                    device_id: "local".into(),
                    status: zeron_proto::SessionStatus::Working,
                    started_at: Some(now - chrono::TimeDelta::seconds(10)),
                    updated_at: now - chrono::TimeDelta::seconds(if stale { 46 } else { 0 }),
                    last_completed_turn: None,
                }]);
                cx.notify();
            });
            visual.run_until_parked();
            visual.update(|window, cx| window.draw(cx).clear());
            assert!(crate::perf_trace::render_count(transcript.entity_id()) > before);
            assert_eq!(
                transcript.read_with(visual, |this, _| this.chrome.unwrap().working),
                !stale
            );
            assert_eq!(
                state.read_with(visual, |state, _| state.transcript_revision),
                revision
            );
        }
    }

    #[gpui::test]
    fn cached_list_wheel_cancels_competing_rail_glide(cx: &mut TestAppContext) {
        let _dir = setup(cx);
        let (surface, visual) = cx.add_window_view(|_, cx| {
            let state = cx.new(|_| {
                let mut state = AppState::new();
                state.selected_chat = Some("chat".into());
                state.transcript_replayed = true;
                state
            });
            Surface {
                transcripts: vec![cx.new(|cx| settled(state, false, cx))],
                reuse: true,
                opacity: 1.0,
            }
        });
        let transcript = surface.read_with(visual, |s, _| s.transcripts[0].clone());
        transcript.update(visual, |t, cx| {
            t.rows = (0..100)
                .map(|i| viewport_row(&format!("row-{i}"), "entry"))
                .collect();
            t.list.reset(t.rows.len());
            cx.notify();
        });
        visual.update(|w, cx| w.draw(cx).clear());
        transcript.update(visual, |t, cx| {
            t.scroll_to_row(0, cx);
            assert!(t.scroll_anim.is_some());
        });
        visual.update(|w, cx| {
            w.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: gpui::point(px(300.), px(300.)),
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(40.))),
                    ..Default::default()
                }),
                cx,
            );
        });
        transcript.read_with(visual, |t, _| {
            assert!(
                t.scroll_anim.is_none(),
                "the list's real input callback must cancel the rail task synchronously"
            );
            assert!(!t.pinned);
        });
    }

    #[gpui::test]
    fn cache_hit_preserves_focus_and_text_selection_dispatch(cx: &mut TestAppContext) {
        let _dir = setup(cx);
        let (surface, visual) = cx.add_window_view(|_, cx| {
            let state = cx.new(|_| {
                let mut state = AppState::new();
                state.selected_chat = Some("chat".into());
                state.apply_transcript(vec![assistant(
                    "reply",
                    MessageStatus::Complete,
                    vec![text_part("text", "Selectable cached response text.")],
                )]);
                state
            });
            Surface {
                transcripts: vec![cx.new(|cx| settled(state, false, cx))],
                reuse: true,
                opacity: 1.0,
            }
        });
        visual.update(|window, cx| window.draw(cx).clear());
        let transcript = surface.read_with(visual, |surface, _| surface.transcripts[0].clone());
        let focus = transcript.read_with(visual, |this, _| this.attachment_preview_focus.clone());
        visual.update(|window, cx| window.focus(&focus, cx));
        visual.update(|window, cx| window.draw(cx).clear());
        surface.update(visual, |_, cx| cx.notify());
        visual.run_until_parked();
        let before = crate::perf_trace::render_count(transcript.entity_id());
        visual.update(|window, cx| {
            window.draw(cx).clear();
            assert!(focus.is_focused(window));
        });
        assert_eq!(
            crate::perf_trace::render_count(transcript.entity_id()),
            before
        );
        let bounds = render::selection_test_bounds("reply#text.0:0");
        let start = bounds.origin + gpui::point(px(1.0), px(8.0));
        visual.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::default());
        assert!(crate::markdown::selection::is_dragging());
        visual.simulate_mouse_move(
            start + gpui::point(px(90.0), px(0.0)),
            Some(MouseButton::Left),
            gpui::Modifiers::default(),
        );
        assert!(crate::markdown::selection::selected_text().is_some());
        visual.simulate_mouse_up(
            start + gpui::point(px(90.0), px(0.0)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        assert_eq!(
            transcript
                .read_with(visual, |this, _| this.chat_id.clone())
                .as_deref(),
            Some("chat")
        );
        crate::markdown::selection::clear_if_owner("reply#text.0:0");
    }

    #[gpui::test]
    fn intermediate_fades_and_scope_changes_never_replay_stale_scenes(cx: &mut TestAppContext) {
        let _dir = setup(cx);
        let (surface, visual) = cx.add_window_view(|_, cx| {
            let state = cx.new(|_| AppState::new());
            Surface {
                transcripts: vec![cx.new(|cx| settled(state, false, cx))],
                reuse: true,
                opacity: 1.0,
            }
        });
        visual.update(|window, cx| window.draw(cx).clear());
        let transcript = surface.read_with(visual, |surface, _| surface.transcripts[0].clone());
        for opacity in [0.75, 0.5, 0.25, 0.0, 0.5] {
            let before = crate::perf_trace::render_count(transcript.entity_id());
            surface.update(visual, |surface, cx| {
                surface.opacity = opacity;
                cx.notify();
            });
            visual.update(|window, cx| window.draw(cx).clear());
            assert!(crate::perf_trace::render_count(transcript.entity_id()) > before);
        }
        surface.update(visual, |surface, cx| {
            surface.opacity = 1.0;
            cx.notify();
        });
        visual.update(|window, cx| window.draw(cx).clear());
        let before = crate::perf_trace::render_count(transcript.entity_id());
        transcript.update(visual, |this, cx| this.set_scene_fade_band(140.0, cx));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear());
        assert!(crate::perf_trace::render_count(transcript.entity_id()) > before);
    }
}
