//! The staged-tree confirmation surface (DESIGN-W3 §7).
use gpui::{
    AnyElement, Context, Entity, EventEmitter, FontWeight, SharedString, Subscription, Window, div,
    prelude::*, px,
};
use serde_json::json;
use zeron_proto::git_actions::*;
use zeron_rpc::git_actions::methods;

use crate::{
    composer::{ComposerInput, ComposerInputEvent},
    controls::{self, Size, Variant},
    git_store::{call, routed},
    icons::{self, icon},
    motion, popover,
    settings::widgets,
    state::AppState,
    status_palette::SessionState,
    theme::Theme,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    pub owner_repo: String,
    pub https: String,
}

/// Same canonical host/owner/repo shape required by the engine. No credentials.
pub fn parse_remote(url: &str) -> Option<Repository> {
    let remote = zeron_engine::parse_git_remote(url)?;
    if [&remote.host, &remote.owner, &remote.repository]
        .iter()
        .any(|part| part.contains(['?', '#']))
    {
        return None;
    }
    Some(Repository {
        owner_repo: format!("{}/{}", remote.owner, remote.repository),
        https: format!(
            "https://{}/{}/{}",
            remote.host, remote.owner, remote.repository
        ),
    })
}

/// Progress text names commits by full SHA; the dialog reads them at the
/// short length everywhere else uses.
pub fn short_shas(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let bare = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if bare.len() == 40 && bare.chars().all(|c| c.is_ascii_hexdigit()) {
                word.replacen(bare, &bare[..7], 1)
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn chain_label(push: bool, pr: bool) -> &'static str {
    if push && pr {
        "Commit, push & open PR"
    } else if push {
        "Commit & push"
    } else {
        "Commit"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

pub fn progress_steps(
    state: &GitActionState,
    push: bool,
    pr: bool,
) -> Vec<(&'static str, StepStatus, String)> {
    let mut steps: Vec<_> = [
        ("commit", "Commit", true),
        ("push", "Push", push),
        ("pr", "Open pull request", push && pr),
        ("link", "Link to thread", push && pr),
    ]
    .into_iter()
    .filter(|(_, _, enabled)| *enabled)
    .map(|(phase, label, _)| {
        let event = state
            .progress
            .iter()
            .filter(|e| e.phase == phase)
            .max_by_key(|e| e.sequence);
        let mut status = match event.map(|e| e.kind.as_str()) {
            Some("phase_completed") => StepStatus::Done,
            Some("phase_started") => StepStatus::Running,
            Some("failed") => StepStatus::Failed,
            _ => StepStatus::Pending,
        };
        if matches!(state.status.as_str(), "failed" | "uncertain") && status == StepStatus::Running
        {
            status = StepStatus::Failed;
        }
        (
            label,
            status,
            event.map(|e| e.text.clone()).unwrap_or_default(),
        )
    })
    .collect();
    if matches!(state.status.as_str(), "failed" | "uncertain")
        && !steps.iter().any(|s| s.1 == StepStatus::Failed)
        && let Some(step) = steps.iter_mut().find(|s| s.1 != StepStatus::Done)
    {
        step.1 = StepStatus::Failed;
        step.2 = state.error.clone().unwrap_or_else(|| step.2.clone());
    }
    steps
}

pub fn mono(text: impl Into<SharedString>, theme: &Theme, color: gpui::Hsla) -> gpui::Div {
    div()
        .font_family(theme.font_mono.clone())
        .text_size(crate::typography::ui_rems(11.0))
        .text_color(color)
        .child(text.into())
}

/// Use gpui's existing checkbox, with the shared action role.
pub fn checkbox(
    id: impl Into<SharedString>,
    checked: bool,
    disabled: bool,
    label: &'static str,
    theme: &Theme,
) -> gpui_base::Checkbox {
    gpui_base::Checkbox::new(id.into())
        .checked(checked)
        .disabled(disabled)
        .accessibility_label(label)
        .size(px(16.0))
        .border_1()
        .rounded(px(3.0))
        .border_color(if checked {
            theme.action()
        } else {
            theme.border
        })
        .bg(if checked {
            theme.action()
        } else {
            gpui::transparent_black()
        })
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |el| {
            el.child(
                icon(icons::CHECK)
                    .size(px(12.0))
                    .text_color(theme.on_action()),
            )
        })
        .when(disabled, |el| el.opacity(0.5))
}

/// Shared quiet input shell for Git, PR linking, handoff and restore dialogs.
pub(crate) fn dialog_field(
    input: &Entity<ComposerInput>,
    min_lines: usize,
    max_lines: usize,
    theme: &Theme,
    cx: &gpui::App,
) -> AnyElement {
    let height = input
        .read(cx)
        .measured_text_height()
        .clamp(min_lines as f32 * 18.0, max_lines as f32 * 18.0);
    let pad_y = if max_lines == 1 { 4.0 } else { 8.0 };
    div()
        .w_full()
        .h(px(height + pad_y * 2.0))
        .overflow_hidden()
        .px(px(if max_lines == 1 { 8.0 } else { 12.0 }))
        .py(px(pad_y))
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.border)
        .text_size(crate::typography::ui_rems(13.0))
        .child(input.clone())
        .into_any_element()
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Inspecting,
    Writing,
    Ready,
    Running,
    Done,
    Failed,
}
pub struct Closed;

pub struct GitDialog {
    state: Entity<AppState>,
    thread_id: String,
    owner: String,
    cwd: String,
    checkout: Option<GitCheckout>,
    preview: Option<GitMessagePreview>,
    phase: Phase,
    error: Option<String>,
    push: bool,
    pr: bool,
    remote: usize,
    repository: usize,
    menu: Option<bool>,
    action: Option<GitActionState>,
    request_id: Option<String>,
    writer_model: String,
    commit: Entity<ComposerInput>,
    title: Entity<ComposerInput>,
    body: Entity<ComposerInput>,
    base: Entity<ComposerInput>,
    _inputs: Vec<Subscription>,
}

impl EventEmitter<Closed> for GitDialog {}

impl GitDialog {
    pub fn new(state: Entity<AppState>, thread_id: String, cx: &mut Context<Self>) -> Self {
        motion::init_hover_owner(cx);
        let (owner, cwd) = {
            let state = state.read(cx);
            let chat = state.chats.iter().find(|c| c.id == thread_id);
            let space = chat
                .and_then(|c| c.space_id.as_ref())
                .and_then(|id| state.spaces.iter().find(|s| &s.id == id));
            (
                chat.map(|c| c.device_id.clone()).unwrap_or_default(),
                chat.and_then(|c| c.cwd.clone())
                    .or_else(|| space.map(|s| s.path.clone()))
                    .unwrap_or_default(),
            )
        };
        let input = |placeholder, single, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let field = ComposerInput::new(placeholder, cx).with_text_metrics(13.0, 18.0);
                if single {
                    field.with_single_line()
                } else {
                    field
                }
            })
        };
        let commit = cx.new(|cx| {
            ComposerInput::new("Commit message", cx)
                .with_text_metrics(13.0, 18.0)
                .with_max_visible_lines(12)
        });
        let title = input("Pull request title", true, cx);
        let body = cx.new(|cx| {
            ComposerInput::new("Pull request body", cx)
                .with_text_metrics(13.0, 18.0)
                .with_max_visible_lines(6)
        });
        let base = cx.new(|cx| {
            ComposerInput::new("Base branch", cx)
                .with_single_line()
                .with_monospace()
                .with_text_metrics(11.0, 18.0)
        });
        let inputs = [&commit, &title, &body, &base]
            .into_iter()
            .map(|i| cx.subscribe(i, |_, _, _: &ComposerInputEvent, cx| cx.notify()))
            .collect();
        let mut this = Self {
            state,
            thread_id,
            owner,
            cwd,
            checkout: None,
            preview: None,
            phase: Phase::Inspecting,
            error: None,
            push: false,
            pr: false,
            remote: 0,
            repository: 0,
            menu: None,
            action: None,
            request_id: None,
            writer_model: "configured model".into(),
            commit,
            title,
            body,
            base,
            _inputs: inputs,
        };
        this.inspect(cx);
        this
    }

    fn inspect(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            self.error = Some("Engine unavailable".into());
            self.phase = Phase::Failed;
            return;
        };
        let owner = self.owner.clone();
        let cwd = self.cwd.clone();
        cx.spawn(async move |this, cx| {
            let result = call::<GitCheckout>(
                engine.client(),
                &owner,
                methods::GET_GIT_STATUS,
                json!({"cwd":cwd}),
            )
            .await;
            this.update(cx, |view, cx| match result {
                Ok(checkout) => {
                    view.remote = checkout
                        .remotes
                        .iter()
                        .position(|r| r.name == "origin")
                        .unwrap_or(0);
                    view.repository = view.remote;
                    let base = checkout
                        .upstream
                        .as_deref()
                        .and_then(|u| u.split_once('/').map(|(_, b)| b))
                        .unwrap_or(&checkout.branch);
                    view.base.update(cx, |i, cx| i.set_text(base, cx));
                    let empty = checkout.staged_paths.is_empty();
                    view.checkout = Some(checkout);
                    if empty {
                        view.phase = Phase::Ready;
                        cx.notify();
                    } else {
                        view.generate(cx);
                    }
                }
                Err(error) => {
                    view.error = Some(error.to_string());
                    view.phase = Phase::Failed;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn generate(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.phase = Phase::Writing;
        self.error = None;
        let owner = self.owner.clone();
        let request = GitPreviewRequest {
            thread_id: self.thread_id.clone(),
            cwd: self.cwd.clone(),
            base_branch: self.base.read(cx).text().into(),
            ..Default::default()
        };
        cx.spawn(async move |this, cx| {
            if let Ok(settings) = call::<SourceControlSettings>(
                engine.client(),
                &owner,
                methods::GET_WRITER,
                json!({}),
            )
            .await
            {
                this.update(cx, |view, cx| {
                    view.writer_model = settings.model.unwrap_or_else(|| "configured model".into());
                    cx.notify();
                })
                .ok();
            }
            let result =
                call::<GitMessagePreview>(engine.client(), &owner, methods::PREVIEW_GIT, &request)
                    .await;
            this.update(cx, |view, cx| {
                match result {
                    Ok(preview) => {
                        view.commit
                            .update(cx, |i, cx| i.set_text(&preview.commit_message, cx));
                        view.title
                            .update(cx, |i, cx| i.set_text(&preview.pr_title, cx));
                        view.body
                            .update(cx, |i, cx| i.set_text(&preview.pr_body, cx));
                        view.base
                            .update(cx, |i, cx| i.set_text(&preview.base_branch, cx));
                        view.checkout = Some(preview.checkout.clone());
                        view.preview = Some(preview);
                        view.phase = Phase::Ready;
                    }
                    Err(error) => {
                        view.error = Some(error.to_string());
                        view.phase = Phase::Failed;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        if self.phase != Phase::Ready || self.commit.read(cx).text().trim().is_empty() {
            return;
        }
        let Some(preview) = self.preview.clone() else {
            return;
        };
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let push_remote = preview.checkout.remotes.get(self.remote).cloned();
        let pr_repository = preview
            .checkout
            .remotes
            .get(self.repository)
            .and_then(|r| parse_remote(&r.push_url))
            .map(|r| r.https);
        if self.push && push_remote.is_none() || self.pr && pr_repository.is_none() {
            return;
        }
        let request = GitPreviewRequest {
            thread_id: self.thread_id.clone(),
            cwd: self.cwd.clone(),
            base_branch: self.base.read(cx).text().into(),
            commit_message: Some(self.commit.read(cx).text().into()),
            pr_title: Some(self.title.read(cx).text().into()),
            pr_body: Some(self.body.read(cx).text().into()),
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        self.request_id = Some(request_id.clone());
        self.phase = Phase::Running;
        let (owner, push, pr) = (self.owner.clone(), self.push, self.pr);
        cx.spawn(async move |this, cx| {
            let result = async {
                let refreshed: GitMessagePreview =
                    call(engine.client(), &owner, methods::PREVIEW_GIT, &request)
                        .await
                        .map_err(|e| e.to_string())?;
                if refreshed.checkout != preview.checkout {
                    return Err("Checkout/staged scope changed. Close and inspect it again.".into());
                }
                let start = GitActionRequest {
                    request_id: request_id.clone(),
                    preview_id: refreshed.preview_id,
                    confirmed_checkout: preview.checkout,
                    authorize_commit: true,
                    authorize_push: push,
                    authorize_create_pr: push && pr,
                    push_remote: push.then_some(push_remote).flatten(),
                    pr_repository: (push && pr).then_some(pr_repository).flatten(),
                };
                call::<GitActionState>(engine.client(), &owner, methods::START_GIT, &start)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            let action = match result {
                Ok(action) => action,
                Err(error) => {
                    this.update(cx, |view, cx| {
                        view.phase = Phase::Failed;
                        view.error = Some(error);
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let id = action.action_id.clone();
            let running = action.status == "running";
            this.update(cx, |view, cx| view.receive(action, cx)).ok();
            if !running {
                return;
            }
            // Reconnect observation only. Never repeat Start.
            loop {
                match engine
                    .client()
                    .subscribe_checked(
                        methods::WATCH_GIT_ACTION,
                        routed(&owner, json!({"actionId":id})),
                    )
                    .await
                {
                    Ok(mut stream) => {
                        while let Some(value) = stream.recv().await {
                            if let Ok(action) = serde_json::from_value::<GitActionState>(value) {
                                let done = action.status != "running";
                                if this
                                    .update(cx, |view, cx| view.receive(action, cx))
                                    .is_err()
                                    || done
                                {
                                    return;
                                }
                            }
                        }
                    }
                    Err(error) => {
                        this.update(cx, |view, cx| {
                            view.error = Some(error.to_string());
                            cx.notify();
                        })
                        .ok();
                    }
                }
                // Durable reconciliation if the watch ends before its terminal snapshot.
                let result: Result<GitActionState, _> = call(
                    engine.client(),
                    &owner,
                    methods::GET_GIT_ACTION,
                    json!({"actionId":id}),
                )
                .await;
                let done = result.as_ref().is_ok_and(|a| a.status != "running");
                if this
                    .update(cx, |view, cx| match result {
                        Ok(action) => view.receive(action, cx),
                        Err(error) => {
                            view.error = Some(error.to_string());
                            cx.notify();
                        }
                    })
                    .is_err()
                    || done
                {
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(3))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    fn receive(&mut self, action: GitActionState, cx: &mut Context<Self>) {
        self.phase = match action.status.as_str() {
            "running" => Phase::Running,
            "completed" => Phase::Done,
            _ => Phase::Failed,
        };
        self.error = action.error.clone();
        self.action = Some(action);
        cx.notify();
    }

    fn field(
        &self,
        input: &Entity<ComposerInput>,
        min_lines: usize,
        max_lines: usize,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        dialog_field(input, min_lines, max_lines, theme, cx)
    }

    fn remote_chip(&self, repository: bool, theme: &Theme, cx: &Context<Self>) -> AnyElement {
        let selected = self.checkout.as_ref().and_then(|c| {
            c.remotes.get(if repository {
                self.repository
            } else {
                self.remote
            })
        });
        // The repository chip names `owner/repo` (the full URL is in its
        // menu); the remote chip names the remote and dims its repo.
        let label = selected
            .map(|r| {
                if repository {
                    parse_remote(&r.push_url)
                        .map(|p| p.owner_repo)
                        .unwrap_or_else(|| r.push_url.clone())
                } else {
                    r.name.clone()
                }
            })
            .unwrap_or_else(|| "No remotes".into());
        controls::button(
            if repository { "git-repo" } else { "git-remote" },
            cx.entity_id(),
            theme,
            Variant::Outline,
            Size::Sm,
            SharedString::from(label),
        )
        .when(repository, |el| {
            el.font_family(theme.font_mono.clone())
                .text_size(crate::typography::ui_rems(11.0))
        })
        .when(!repository, |el| {
            el.when_some(selected, |el, remote| {
                el.child(mono(
                    parse_remote(&remote.push_url)
                        .map(|p| p.owner_repo)
                        .unwrap_or_else(|| remote.push_url.clone()),
                    theme,
                    theme.text_faint,
                ))
            })
        })
        .child(
            icon(icons::ALT_ARROW_DOWN)
                .size(px(12.0))
                .text_color(theme.text_muted),
        )
        .on_click(cx.listener(move |view, _, _, cx| {
            view.menu = if view.menu == Some(repository) {
                None
            } else {
                Some(repository)
            };
            cx.notify();
        }))
        .into_any_element()
    }

    fn ready_body(&self, theme: &Theme, cx: &Context<Self>) -> AnyElement {
        let push_view = cx.entity().downgrade();
        let pr_view = cx.entity().downgrade();
        let paths = self
            .checkout
            .as_ref()
            .map(|c| c.staged_paths.clone())
            .unwrap_or_default();
        let staged = div()
            .id("git-staged")
            .max_h(px(144.0))
            .overflow_y_scroll()
            .children(paths.iter().map(|path| {
                let split = path.rfind('/').map(|i| i + 1).unwrap_or(0);
                div()
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .font_family(theme.font_mono.clone())
                    .text_size(crate::typography::ui_rems(12.0))
                    .child(
                        div()
                            .text_color(theme.text_faint)
                            .child(SharedString::from(path[..split].to_string())),
                    )
                    .child(
                        div()
                            .text_color(theme.text)
                            .child(SharedString::from(path[split..].to_string())),
                    )
            }));
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_size(crate::typography::ui_rems(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(format!("Staged · {}", paths.len())),
            )
            .child(staged)
            .when(paths.is_empty(), |el| {
                el.child(popover::dialog_body(
                    theme,
                    "Nothing is staged. Stage files in Changes first.",
                ))
            });
        if paths.is_empty() {
            return body.into_any_element();
        }
        body = body
            .child(
                div()
                    .relative()
                    .child(self.field(&self.commit, 3, 12, theme, cx))
                    .child(
                        controls::icon_button(
                            "git-regenerate",
                            cx.entity_id(),
                            theme,
                            Variant::Ghost,
                            Size::Xs,
                            icons::RESTART,
                            "Regenerate",
                        )
                        .absolute()
                        .top(px(4.0))
                        .right(px(4.0))
                        .tooltip(crate::tooltip::text("Regenerate"))
                        .on_click(cx.listener(|view, _, _, cx| view.generate(cx))),
                    ),
            )
            .child(
                div()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(checkbox("git-commit-check", true, true, "Commit", theme))
                    .child("Commit"),
            )
            .child(
                div()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        checkbox("git-push", self.push, false, "Push", theme)
                            .cursor_pointer()
                            .on_change(move |_, _, _, cx| {
                                cx.stop_propagation();
                                push_view
                                    .update(cx, |view, cx| {
                                        view.push = !view.push;
                                        if !view.push {
                                            view.pr = false;
                                        }
                                        cx.notify();
                                    })
                                    .ok();
                            }),
                    )
                    .child("Push to")
                    .child(self.remote_chip(false, theme, cx)),
            )
            .child(
                div()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        checkbox("git-pr", self.pr, !self.push, "Open pull request", theme)
                            .cursor_pointer()
                            .on_change(move |_, _, _, cx| {
                                cx.stop_propagation();
                                pr_view
                                    .update(cx, |view, cx| {
                                        if view.push {
                                            view.pr = !view.pr;
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                            }),
                    )
                    .child("Open pull request into")
                    .child(
                        div()
                            .w(px(96.0))
                            .flex_none()
                            .font_family(theme.font_mono.clone())
                            .child(self.field(&self.base, 1, 1, theme, cx)),
                    )
                    .when(self.pr, |row| {
                        row.child(div().text_color(theme.text_muted).child("on"))
                            .child(self.remote_chip(true, theme, cx))
                    }),
            );
        if let Some(repository) = self.menu {
            let remotes = self
                .checkout
                .as_ref()
                .map(|c| c.remotes.clone())
                .unwrap_or_default();
            body = body.child(popover::popover_card(theme).w_full().p(px(4.0)).children(
                remotes.into_iter().enumerate().map(|(index, remote)| {
                    popover::menu_row_owned(
                        cx.entity_id(),
                        theme,
                        false,
                        format!("git-remote-choice-{index}"),
                    )
                    .id(SharedString::from(format!("git-remote-choice-{index}")))
                    .flex()
                    .flex_col()
                    .items_start()
                    .child(SharedString::from(remote.name))
                    .child(mono(remote.push_url, theme, theme.text_faint))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if repository {
                            view.repository = index;
                        } else {
                            view.remote = index;
                            view.repository = index;
                        }
                        view.menu = None;
                        cx.notify();
                    }))
                }),
            ));
        }
        if self.pr {
            let label = |text: &'static str| {
                div()
                    .mb(px(-6.0))
                    .text_size(crate::typography::ui_rems(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(text)
            };
            body = body
                .child(label("Pull request title"))
                .child(self.field(&self.title, 1, 1, theme, cx))
                .child(label("Description"))
                .child(self.field(&self.body, 6, 6, theme, cx));
        }
        body.into_any_element()
    }
}

impl Render for GitDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let owner = cx.entity_id();
        let checkout = self.checkout.as_ref();
        let committed = self.action.as_ref().and_then(|a| a.commit.clone());
        let header = checkout
            .map(|c| {
                format!(
                    "{} · {}",
                    c.branch,
                    committed
                        .as_deref()
                        .unwrap_or(&c.head)
                        .chars()
                        .take(7)
                        .collect::<String>()
                )
            })
            .unwrap_or_default();
        let mut body = div().flex().flex_col().gap(px(12.0));
        if let Some(error) = &self.error {
            body = body.child(widgets::error_strip(&theme, error.clone()));
        }
        match self.phase {
            Phase::Inspecting | Phase::Writing => {
                let model = &self.writer_model;
                let label = if self.phase == Phase::Inspecting {
                    "Inspecting checkout…".into()
                } else {
                    format!("Writing messages with {model}…")
                };
                body = body.child(
                    div()
                        .h(px(96.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(crate::loaders::mini_equalizer(
                            "git-writing",
                            SessionState::Working
                                .color(&theme)
                                .unwrap_or(theme.text_muted),
                            owner,
                            cx,
                        ))
                        .child(div().text_color(theme.text_muted).child(label)),
                );
            }
            Phase::Ready => body = body.child(self.ready_body(&theme, cx)),
            Phase::Running | Phase::Done | Phase::Failed => {
                if self.phase == Phase::Failed {
                    body = body.child(popover::dialog_body(
                        &theme,
                        "Check the repository before trying again.",
                    ));
                }
                let action = self.action.clone().unwrap_or_default();
                for (index, (label, status, text)) in progress_steps(&action, self.push, self.pr)
                    .into_iter()
                    .enumerate()
                {
                    let glyph = match status {
                        StepStatus::Running => crate::loaders::mini_equalizer(
                            format!("git-step-{index}"),
                            SessionState::Working
                                .color(&theme)
                                .unwrap_or(theme.text_muted),
                            owner,
                            cx,
                        )
                        .into_any_element(),
                        StepStatus::Done => icon(icons::CHECK)
                            .size(px(12.0))
                            .text_color(
                                SessionState::Completed
                                    .color(&theme)
                                    .unwrap_or(theme.text_muted),
                            )
                            .into_any_element(),
                        StepStatus::Failed => icon(icons::DANGER_TRIANGLE)
                            .size(px(12.0))
                            .text_color(theme.danger)
                            .into_any_element(),
                        StepStatus::Pending => div()
                            .size(px(4.0))
                            .rounded_full()
                            .bg(theme.text_faint)
                            .into_any_element(),
                    };
                    body = body.child(
                        div()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(div().w(px(12.0)).child(glyph))
                            .child(label)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(crate::typography::ui_rems(12.0))
                                    .text_color(theme.text_muted)
                                    .child(short_shas(&text)),
                            ),
                    );
                }
            }
        }
        // Results: the short SHA copies, the PR opens. Both stay quiet mono
        // metadata; the step glyphs above already carry the state color.
        if matches!(self.phase, Phase::Done | Phase::Failed)
            && let Some(action) = self.action.as_ref().filter(|a| a.pr_url.is_some())
        {
            let result_row = |id: &'static str, label: &'static str| {
                div()
                    .id(id)
                    .h(px(32.0))
                    .px(px(8.0))
                    .mx(px(-8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .hover(|el| el.bg(theme.wash(0.06)))
                    .child(div().flex_1().text_color(theme.text_muted).child(label))
            };
            let mut results = div()
                .mt(px(4.0))
                .pt(px(8.0))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_col();
            if let Some(url) = &action.pr_url {
                let open = url.clone();
                let number = url
                    .rsplit('/')
                    .next()
                    .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
                    .map(|n| format!("#{n}"))
                    .unwrap_or_else(|| url.clone());
                results = results.child(
                    result_row("git-open-pr", "Pull request")
                        .tooltip(crate::tooltip::text(SharedString::from(url.clone())))
                        .on_click(move |_, _, cx| cx.open_url(&open))
                        .child(mono(number, &theme, theme.text))
                        .child(
                            icon(icons::ARROW_UP_RIGHT)
                                .size(px(12.0))
                                .text_color(theme.text_faint),
                        ),
                );
            }
            body = body.child(results);
        }
        let ready = self.phase == Phase::Ready;
        let enabled = ready
            && self.preview.is_some()
            && checkout.is_some_and(|c| !c.staged_paths.is_empty())
            && !self.commit.read(cx).text().trim().is_empty()
            && (!self.push || checkout.is_some_and(|c| c.remotes.get(self.remote).is_some()))
            && (!self.pr
                || checkout.is_some_and(|c| {
                    c.remotes
                        .get(self.repository)
                        .and_then(|r| parse_remote(&r.push_url))
                        .is_some()
                }));
        let footer = popover::dialog_footer(&theme)
            .child(
                mono(
                    match self.phase {
                        Phase::Running => "Runs on the host. Closing this doesn't stop it.",
                        Phase::Done => "",
                        _ => "Hooks are skipped in this version",
                    },
                    &theme,
                    theme.text_faint,
                )
                .flex_1(),
            )
            .when(self.phase != Phase::Done, |el| {
                el.child(
                    controls::button(
                        "git-cancel",
                        owner,
                        &theme,
                        Variant::Ghost,
                        Size::Md,
                        if ready { "Cancel" } else { "Close" },
                    )
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Closed))),
                )
            })
            .when(ready || self.phase == Phase::Done, |el| {
                el.child(
                    controls::button(
                        "git-start",
                        owner,
                        &theme,
                        Variant::Primary,
                        Size::Md,
                        if ready {
                            chain_label(self.push, self.pr)
                        } else {
                            "Done"
                        },
                    )
                    .when(ready && !enabled, |el| {
                        el.opacity(controls::DISABLED_OPACITY)
                    })
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if view.phase == Phase::Done {
                            cx.emit(Closed);
                        } else if enabled {
                            view.start(cx);
                        }
                    })),
                )
            });
        let card = popover::dialog_card(&theme)
            .w(px(560.0))
            .max_h(window.viewport_size().height * 0.8)
            .text_size(crate::typography::ui_rems(13.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        popover::dialog_title(&theme, "Commit changes")
                            .text_size(crate::typography::ui_rems(15.0)),
                    )
                    .child(
                        controls::close_button("git-close", owner, &theme)
                            .tooltip(crate::tooltip::text("Close"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Closed))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(0.0))
                    .child(match committed.clone() {
                        // Once committed, the sub-line names the new HEAD and
                        // copies its full SHA.
                        Some(sha) => mono(header, &theme, theme.text_muted)
                            .id("git-copy-sha")
                            .cursor_pointer()
                            .tooltip(crate::tooltip::text("Copy commit SHA"))
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(sha.clone()))
                            })
                            .into_any_element(),
                        None => mono(header, &theme, theme.text_muted).into_any_element(),
                    })
                    .when(checkout.is_some_and(|c| c.dirty), |el| {
                        el.child(mono(" · unstaged changes", &theme, theme.text_faint))
                    }),
            )
            .child(
                div()
                    .id("git-dialog-body")
                    .mt(px(16.0))
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(footer);
        motion::drive_hover_owner(owner, window);
        card
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forge_remote_parsing_and_canonical_https() {
        for url in [
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
            "https://github.com/owner/repo.git",
            "https://github.com/owner/repo/",
        ] {
            assert_eq!(
                parse_remote(url),
                Some(Repository {
                    owner_repo: "owner/repo".into(),
                    https: "https://github.com/owner/repo".into()
                })
            );
        }
        for url in [
            "/tmp/repo",
            "file:///repo",
            "https://github.com/owner",
            "git@host:owner/nested/repo",
            "https://host/a/b?secret=1",
        ] {
            assert!(parse_remote(url).is_none(), "{url}");
        }
    }
    #[test]
    fn progress_text_uses_short_shas() {
        assert_eq!(
            short_shas("Committed a6991899654ef3feb5a1586377f70f2d2f8cbcb3."),
            "Committed a699189."
        );
        assert_eq!(short_shas("Pushed to origin"), "Pushed to origin");
    }

    #[test]
    fn labels_require_separate_push_authorization() {
        assert_eq!(chain_label(false, false), "Commit");
        assert_eq!(chain_label(false, true), "Commit");
        assert_eq!(chain_label(true, false), "Commit & push");
        assert_eq!(chain_label(true, true), "Commit, push & open PR");
    }
    #[test]
    fn failure_before_a_phase_started_is_not_all_pending() {
        let state = GitActionState {
            status: "uncertain".into(),
            error: Some("Checkout locked".into()),
            ..Default::default()
        };
        let steps = progress_steps(&state, true, true);
        assert_eq!(steps[0].1, StepStatus::Failed);
        assert_eq!(steps[0].2, "Checkout locked");
        assert_eq!(steps[1].1, StepStatus::Pending);
    }
    #[test]
    fn progress_maps_latest_sequence_and_uncertain_running_phase() {
        let state = GitActionState {
            status: "uncertain".into(),
            progress: vec![
                GitProgress {
                    sequence: 2,
                    phase: "commit".into(),
                    kind: "phase_completed".into(),
                    text: "sha".into(),
                },
                GitProgress {
                    sequence: 1,
                    phase: "commit".into(),
                    kind: "phase_started".into(),
                    ..Default::default()
                },
                GitProgress {
                    sequence: 3,
                    phase: "push".into(),
                    kind: "phase_started".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let steps = progress_steps(&state, true, true);
        assert_eq!(
            steps.iter().map(|s| s.1).collect::<Vec<_>>(),
            [
                StepStatus::Done,
                StepStatus::Failed,
                StepStatus::Pending,
                StepStatus::Pending
            ]
        );
        assert_eq!(progress_steps(&state, false, false).len(), 1);
    }
}
