//! Details lineage keyboard workflow: Tab reaches available rows, ↑/↓ step
//! between them, Enter opens the focused one. Unavailable rows are skipped.
use super::*;
use gpui::{AppContext, TestAppContext};
use std::{cell::RefCell, rc::Rc};

struct DetailsHost {
    shell: Entity<Shell>,
    model: crate::details::DetailsModel,
    opened: Rc<RefCell<Vec<String>>>,
}

impl Render for DetailsHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.clone();
        let opened = self.opened.clone();
        self.shell.update(cx, |shell, cx| {
            let mut actions = shell.live_details_actions("chat".into());
            actions.open_thread = Rc::new(move |_, chat, _| opened.borrow_mut().push(chat));
            div().w(px(320.0)).h(px(640.0)).child(crate::details::details_panel_body(
                "chat",
                &model,
                &crate::details::DetailsUi::default(),
                Utc::now(),
                &Theme::default(),
                &actions,
                Rc::new(|_, (), _| {}),
                cx,
            ))
        })
    }
}

/// gpui's keyboard click fires on key-up; the test platform sends only the down.
fn activate(cx: &mut gpui::VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
}

fn relation(id: &str, available: bool) -> crate::details::ConversationRelation {
    crate::details::ConversationRelation {
        chat_id: id.into(),
        title: id.into(),
        label: "Fork".into(),
        available,
    }
}

#[gpui::test]
fn lineage_rows_are_keyboard_navigable_and_skip_unavailable_threads(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_base::init(cx);
        cx.set_global(Theme::default());
        motion::set_reduced_motion(cx, true);
        settings::init(UiSettings::default(), dir.path(), cx);
    });
    let opened = Rc::new(RefCell::new(Vec::new()));
    let (_host, cx) = cx.add_window_view(|_, cx| {
        let shell = cx.new(|cx| {
            let state = cx.new(|_| AppState::new());
            Shell::new(
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
            )
        });
        DetailsHost {
            shell,
            model: crate::details::DetailsModel {
                lineage: vec![
                    relation("fork-a", true),
                    relation("gone", false),
                    relation("fork-c", true),
                ],
                ..Default::default()
            },
            opened: opened.clone(),
        }
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear();
        window.focus_next(cx);
    });
    activate(cx, "enter");
    assert_eq!(*opened.borrow(), ["fork-a"]);
    // The unavailable thread is not a tab stop, so ↓ lands on the next row.
    cx.simulate_keystrokes("down");
    activate(cx, "enter");
    assert_eq!(*opened.borrow(), ["fork-a", "fork-c"]);
    cx.simulate_keystrokes("up");
    activate(cx, "space");
    assert_eq!(*opened.borrow(), ["fork-a", "fork-c", "fork-a"]);
}
