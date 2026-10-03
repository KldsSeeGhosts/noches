//! Project-header canvas routing, adapted from upstream #737.
use super::chat_rename_tests::{redraw, setup};
use super::*;
use gpui::TestAppContext;

fn space(id: &str, device: &str) -> zeron_proto::Space {
    zeron_proto::Space {
        id: id.into(),
        device_id: device.into(),
        path: format!("/{id}"),
        name: None,
        git_detected: true,
        git_checked_at: None,
        checkout_id: Some(format!("checkout-{id}")),
        created_at: Utc::now(),
    }
}

#[gpui::test]
fn project_new_session_overrides_filter_and_routes_device(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.state.update(cx, |state, _| {
            state.apply_spaces(vec![space("one", "local"), space("two", "remote")]);
        });
        shell.settings.space_filter = Some("one".into());
        shell.open_new_session_in_space("two".into(), cx);
        let state = shell.state.read(cx);
        assert!(state.selected_chat.is_none());
        assert_eq!(state.selected_space.as_deref(), Some("two"));
        assert_eq!(state.selected_device.as_deref(), Some("remote"));
        assert!(!state.no_project);
        assert_eq!(
            state.space_row("two").unwrap().checkout_id.as_deref(),
            Some("checkout-two")
        );
        assert!(shell.solo_session);
        assert!(!shell.terminal_open(cx));
        // The global action still respects the user's standing filter.
        shell.open_new_session(cx);
        assert_eq!(shell.state.read(cx).selected_space.as_deref(), Some("one"));
        assert_eq!(
            shell.state.read(cx).selected_device.as_deref(),
            Some("local")
        );
    });
}

#[gpui::test]
fn project_header_plus_opens_canvas_without_collapsing_group(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.settings.sidebar_organization = SidebarOrganization::ByDevice;
        shell.state.update(cx, |state, _| {
            state.apply_spaces(vec![space("one", "local")]);
            state.chats[0].space_id = Some("one".into());
        });
        cx.notify();
    });
    redraw(cx);
    let button = cx.debug_bounds("sidebar-group-new-session-one").unwrap();
    cx.simulate_mouse_move(button.center(), None, gpui::Modifiers::default());
    redraw(cx);
    cx.simulate_click(button.center(), gpui::Modifiers::default());
    redraw(cx);
    shell.read_with(cx, |shell, cx| {
        let state = shell.state.read(cx);
        assert_eq!(state.selected_space.as_deref(), Some("one"));
        assert_eq!(state.selected_device.as_deref(), Some("local"));
        assert!(state.selected_chat.is_none());
        assert!(shell.sidebar_collapsed_groups.is_empty());
    });
}

#[gpui::test]
fn missing_project_does_not_change_canvas_routing(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.open_chat("older".into(), cx);
        shell.open_new_session_in_space("missing".into(), cx);
        assert_eq!(shell.state.read(cx).selected_chat.as_deref(), Some("older"));
    });
}
