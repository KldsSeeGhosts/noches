//! Sidebar rename regressions adapted from upstream #751.
use super::*;
use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext};

struct SidebarHost(Entity<Shell>);

impl Render for SidebarHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            shell.focus_rename_chat(window, cx);
            div()
                .w(px(280.0))
                .h(px(800.0))
                .capture_key_down(cx.listener(Shell::on_key_down_capture))
                .child(shell.render_chat_sidebar(&Theme::default(), cx))
        })
    }
}

pub(super) fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    let dir = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_base::init(cx);
        cx.set_global(Theme::default());
        motion::set_reduced_motion(cx, true);
        crate::app_menus::init(cx);
        settings::init(UiSettings::default(), dir.path(), cx);
        crate::history::init(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            cx,
        );
        apply_keymap(
            cx,
            &KeymapConfig::default(),
            ComposerSendBehavior::default(),
        );
    });
    let (host, cx) = cx.add_window_view(|_, cx| {
        SidebarHost(cx.new(|cx| {
            let state = cx.new(|_| AppState::new());
            let mut shell = Shell::new(
                state,
                EngineBootConfig {
                    remote: None,
                    data_dir: dir.path().into(),
                    ipc_port: 0,
                    edge_url: "http://127.0.0.1:1".into(),
                    edge_token: None,
                    org_id: None,
                    workos_client_id: None,
                    default_harness: zeron_proto::HarnessId::Mock,
                },
                cx,
            );
            shell.settings.sidebar_organization = SidebarOrganization::InOneList;
            shell.reduced_motion = true;
            shell.state.update(cx, |state, _| {
                state.workspace_scope = Some(WorkspaceScope::Local);
                state.local_device_id = Some("local".into());
                state.no_project = true;
                state.chats = ["older", "newer"]
                    .into_iter()
                    .enumerate()
                    .map(|(ix, id)| chat(id, false, ix as i64))
                    .collect();
            });
            shell
        }))
    });
    let shell = host.read_with(cx, |host, _| host.0.clone());
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear();
    });
    (shell, cx)
}

fn chat(id: &str, archived: bool, age: i64) -> zeron_proto::Chat {
    serde_json::from_value(serde_json::json!({
        "id": id, "title": id, "deviceId": "local", "archived": archived,
        "createdAt": Utc::now() - chrono::Duration::minutes(10 + age),
    }))
    .unwrap()
}

pub(super) fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear();
    });
}

fn double_click(cx: &mut VisualTestContext, position: Point<Pixels>) {
    for click_count in [1, 2] {
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position,
            click_count,
            ..Default::default()
        });
        cx.simulate_event(gpui::MouseUpEvent {
            button: MouseButton::Left,
            position,
            click_count,
            ..Default::default()
        });
    }
    redraw(cx);
}

// Without an engine, reaching mutate leaves an "Engine not connected" notice.
fn mutated(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> bool {
    shell.read_with(cx, |shell, _| shell.sidebar_notice.is_some())
}

#[gpui::test]
fn inline_rename_in_full_and_compact_rows(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell
            .state
            .update(cx, |state, _| state.chats.push(chat("archived", true, 5)));
        cx.notify();
    });
    redraw(cx);
    // Noches' full thread cards and compact archived rows are distinct renderers.
    for (id, row_id, field_id, title_id) in [
        (
            "older",
            "chat-older",
            "chat-title-editor-older",
            "chat-title-older",
        ),
        (
            "archived",
            "archived-archived",
            "chat-title-editor-archived",
            "chat-title-archived",
        ),
    ] {
        let row = cx.debug_bounds(row_id).unwrap();
        double_click(cx, row.center());
        let field = cx.debug_bounds(field_id).unwrap();
        assert!(row.contains(&field.center()));
        assert_eq!(
            cx.debug_bounds(row_id).unwrap().size.height,
            row.size.height
        );
        assert!(cx.debug_bounds(title_id).is_none());
        let input = shell.read_with(cx, |shell, _| {
            let rename = shell.chat_rename.as_ref().unwrap();
            assert_eq!(rename.chat_id, id);
            rename.input.clone()
        });
        cx.update(|window, cx| {
            assert!(input.focus_handle(cx).is_focused(window));
            assert_eq!(input.read(cx).text(), id);
        });
        cx.simulate_click(field.center(), Modifiers::default());
        redraw(cx);
        assert!(shell.read_with(cx, |shell, _| shell.chat_rename.is_some()));
        cx.simulate_input("Discard this edit");
        assert!(shell.read_with(cx, |shell, cx| shell.overlay_owns_keyboard(cx)));
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(shell.read_with(cx, |shell, cx| {
            shell.chat_rename.is_none() && shell.active_composer().read(cx).focus_pending
        }));
        assert!(!mutated(&shell, cx));

        double_click(cx, row.center());
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert!(!mutated(&shell, cx));

        double_click(cx, row.center());
        assert!(
            shell.read_with(cx, |shell, _| shell.chat_rename.is_some()),
            "{id}"
        );
        cx.simulate_input("Renamed");
        assert_eq!(
            shell.read_with(cx, |shell, cx| {
                shell
                    .chat_rename
                    .as_ref()
                    .unwrap()
                    .input
                    .read(cx)
                    .text()
                    .to_string()
            }),
            "Renamed"
        );
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert!(shell.read_with(cx, |shell, cx| {
            shell.chat_rename.is_none() && shell.active_composer().read(cx).focus_pending
        }));
        assert!(mutated(&shell, cx));
        shell.update(cx, |shell, _| shell.sidebar_notice = None);
        redraw(cx);
    }
}

#[gpui::test]
fn menu_rename_commits_on_blur_without_stealing_focus(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| shell.open_rename_chat("newer".into(), cx));
    redraw(cx);
    cx.simulate_input("Moved on");
    cx.update(|window, _| window.blur());
    redraw(cx);
    assert!(shell.read_with(cx, |shell, cx| {
        shell.chat_rename.is_none() && !shell.active_composer().read(cx).focus_pending
    }));
    assert!(mutated(&shell, cx));
}

#[gpui::test]
fn empty_title_does_not_mutate(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| shell.open_rename_chat("older".into(), cx));
    redraw(cx);
    cx.simulate_input("   ");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert!(shell.read_with(cx, |shell, _| shell.chat_rename.is_none()));
    assert!(!mutated(&shell, cx));
}

#[gpui::test]
fn rename_reveals_collapsed_group_and_sidebar(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.settings.sidebar_organization = SidebarOrganization::ByDevice;
        shell.settings.sidebar_collapsed = true;
        shell
            .sidebar_collapsed_groups
            .insert("device:local:".into());
        shell.open_rename_chat("older".into(), cx);
    });
    redraw(cx);
    assert!(cx.debug_bounds("chat-title-editor-older").is_some());
    shell.read_with(cx, |shell, _| {
        assert!(!shell.settings.sidebar_collapsed);
        assert!(shell.sidebar_collapsed_groups.is_empty());
        assert!(shell.sidebar_reveal_motions.is_empty());
        assert_eq!(
            shell.sidebar_disclosure_motion["group:device:local:"].from,
            0.0
        );
    });
}

#[gpui::test]
fn rename_pages_and_scrolls_archived_row_into_view(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.state.update(cx, |state, _| {
            state
                .chats
                .extend((0..40).map(|ix| chat(&format!("archived-{ix:02}"), true, ix)));
        });
        shell.archived_open = Some(false);
        shell.open_rename_chat("archived-39".into(), cx);
    });
    for _ in 0..4 {
        cx.run_until_parked();
        redraw(cx);
    }
    let field = cx.debug_bounds("chat-title-editor-archived-39").unwrap();
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.archived_open, Some(true));
        assert_eq!(shell.archived_shown, 60);
        let viewport = shell.sidebar_scroll.bounds();
        assert!(shell.sidebar_scroll.offset().y < px(0.0));
        assert!(field.top() >= viewport.top() && field.bottom() <= viewport.bottom());
    });
}

#[gpui::test]
fn rename_returns_focus_to_the_focused_pane_without_rebinding_it(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.state.update(cx, |state, cx| {
            state.apply_spaces(vec![
                serde_json::from_value(serde_json::json!({
                    "id": "project", "deviceId": "local", "path": "/tmp",
                    "gitDetected": false, "createdAt": Utc::now(),
                }))
                .unwrap(),
            ]);
            state.chats[0].space_id = Some("project".into());
            state.select_chat(Some("older".into()), cx);
        });
        shell.on_state_changed(&shell.state.clone(), cx);
        shell.split_workspace_view(Direction::Right, cx);
        assert!(shell.workspace_mode());
        for surface in shell.workspace.chat_surfaces.values() {
            surface.composer.update(cx, |composer, cx| {
                composer
                    .input
                    .update(cx, |input, cx| input.set_text("Pane draft", cx));
                composer.focus_pending = true;
            });
        }
        shell.open_rename_chat("older".into(), cx);
        assert!(
            shell
                .workspace
                .chat_surfaces
                .values()
                .all(|surface| !surface.composer.read(cx).focus_pending)
        );
    });
    redraw(cx);
    let before = shell.read_with(cx, |shell, cx| {
        (
            shell.workspace.focused_pane(),
            shell.state.read(cx).selected_chat.clone(),
            shell.active_composer().entity_id(),
        )
    });
    cx.simulate_keystrokes("escape");
    redraw(cx);
    shell.read_with(cx, |shell, cx| {
        assert_eq!(shell.workspace.focused_pane(), before.0);
        assert_eq!(shell.state.read(cx).selected_chat, before.1);
        assert_eq!(shell.active_composer().entity_id(), before.2);
        assert!(shell.active_composer().read(cx).focus_pending);
        for (pane, surface) in &shell.workspace.chat_surfaces {
            let composer = surface.composer.read(cx);
            assert_eq!(composer.input.read(cx).text(), "Pane draft");
            assert_eq!(composer.focus_pending, Some(*pane) == before.0);
        }
    });
}

#[gpui::test]
fn rename_scrolls_active_row_into_view(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.state.update(cx, |state, _| {
            state
                .chats
                .extend((0..40).map(|ix| chat(&format!("chat-{ix:02}"), false, ix + 20)));
        });
        shell.open_rename_chat("chat-39".into(), cx);
    });
    for _ in 0..4 {
        cx.run_until_parked();
        redraw(cx);
    }
    let field = cx.debug_bounds("chat-title-editor-chat-39").unwrap();
    shell.read_with(cx, |shell, _| {
        let viewport = shell.sidebar_scroll.bounds();
        assert!(shell.sidebar_scroll.offset().y < px(0.0));
        assert!(field.top() >= viewport.top() && field.bottom() <= viewport.bottom());
    });
}

#[gpui::test]
fn rename_refuses_hidden_or_missing_rows(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.settings.space_filter = Some("elsewhere".into());
        shell.open_rename_chat("older".into(), cx);
        assert!(shell.chat_rename.is_none());
        assert!(shell.sidebar_notice.is_some());
        assert_eq!(shell.settings.space_filter.as_deref(), Some("elsewhere"));
        shell.open_rename_chat("missing".into(), cx);
        assert!(shell.chat_rename.is_none());
    });
}

#[gpui::test]
fn header_rename_works_without_a_visible_sidebar_row_and_stays_out_of_it(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        // The sidebar filter hides the row, which refuses a sidebar rename...
        shell.settings.space_filter = Some("elsewhere".into());
        shell.open_rename_chat_in_header("older".into(), cx);
        // ...but the header editor opens anyway, and only the header sees it.
        assert!(shell.chat_rename.is_some());
        assert!(shell.header_rename_input_for("older").is_some());
        assert!(shell.rename_input_for("older").is_none());
        assert_eq!(
            shell.header_rename().map(|(id, _)| id).as_deref(),
            Some("older")
        );
        assert!(shell.header_rename_input_for("newer").is_none());
        // Opening the thread menu from the title cancels nothing else.
        shell.open_chat_title_menu("older".into(), gpui::point(px(10.0), px(10.0)), cx);
        assert!(shell.header_title_menu_task.is_none());
    });
}
