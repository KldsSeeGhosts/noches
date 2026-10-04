//! Live tool approvals deliberately do not enter the question wizard.
use super::*;
use zeron_doc::MessageStatus;
use zeron_proto::{PermissionDecision, PermissionRequest};

/// Unlike orphan content questions, dead approvals cannot resume a run.
pub(super) fn pending_permission_request(
    transcript: &[SessionMessageEntry],
) -> Option<PermissionRequest> {
    let entry = transcript
        .iter()
        .rev()
        .find(|entry| entry.role == MessageRole::Assistant)?;
    if entry.status != Some(MessageStatus::Streaming) {
        return None;
    }
    entry.parts.iter().find_map(|part| match part {
        MessagePart::Permission { request, .. }
            if request.state == zeron_proto::RequestState::Pending =>
        {
            Some(request.clone())
        }
        _ => None,
    })
}

impl Composer {
    pub(super) fn render_permission(
        &mut self,
        request: PermissionRequest,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = Theme::of(cx).clone();
        let color = crate::status_palette::SessionState::AwaitingInput
            .color(&theme)
            .unwrap_or(theme.text_muted);
        let mut actions = div().flex().flex_wrap().gap(px(8.0));
        for (ix, (decision, label)) in [
            (PermissionDecision::Accept, "Allow once"),
            (PermissionDecision::AcceptForSession, "Allow for session"),
            (PermissionDecision::Decline, "Deny"),
        ]
        .into_iter()
        .enumerate()
        {
            let option = request
                .options
                .iter()
                .find(|o| o.decision == decision)
                .map(|o| o.id.clone());
            let request_id = request.id.clone();
            actions = actions.child(
                div()
                    .id(("permission-answer", ix))
                    .px(px(10.0))
                    .py(px(6.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(5.0))
                    .text_size(px(12.0))
                    .when(option.is_none(), |el| el.opacity(0.4))
                    .when_some(option, |el, option_id| {
                        el.cursor_pointer()
                            .hover(|s| s.bg(theme.wash(0.08)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.answer_permission(request_id.clone(), option_id.clone(), cx);
                            }))
                    })
                    .child(label),
            );
        }
        div()
            .id("permission-surface")
            .p(px(12.0))
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap(px(8.0))
            .text_color(theme.text_muted)
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(color)
                    .child("Permission required"),
            )
            .child(div().text_size(px(12.0)).child(request.description))
            .children(
                request
                    .options
                    .iter()
                    .filter_map(|o| o.warning.clone())
                    .map(|warning| div().text_size(px(11.0)).child(warning)),
            )
            .child(actions)
            .into_any_element()
    }

    fn answer_permission(&mut self, request_id: String, option_id: String, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        let Some(engine) = state.engine().cloned() else {
            return;
        };
        let Some(chat_id) = self.target.chat_id(state).map(str::to_owned) else {
            return;
        };
        let command = SessionCommandPayload::RespondPermission {
            request_id,
            option_id,
        };
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::QUEUE_COMMAND,
                    serde_json::json!({"chatId":chat_id,"command":command}),
                )
                .await;
            if let Err(err) = result {
                let _ = this.update(cx, |this, cx| {
                    this.failure = Some(format!("Approval failed: {err}").into());
                    this.failure_key = Some(chat_id);
                    cx.notify();
                });
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_resolved_and_superseded_permissions_never_appear_answerable() {
        let mut entry = SessionMessageEntry {
            id: "a".into(),
            role: MessageRole::Assistant,
            created_at: 0,
            device_id: "device".into(),
            status: Some(MessageStatus::Streaming),
            continuation_of: None,
            parts: vec![MessagePart::Permission {
                id: "p".into(),
                request: PermissionRequest::standard("Bash", "ls", true),
            }],
        };
        assert!(pending_permission_request(&[entry.clone()]).is_some());
        for state in [
            zeron_proto::RequestState::Resolved,
            zeron_proto::RequestState::Expired,
        ] {
            if let MessagePart::Permission { request, .. } = &mut entry.parts[0] {
                request.state = state;
            }
            assert!(pending_permission_request(&[entry.clone()]).is_none());
        }
        if let MessagePart::Permission { request, .. } = &mut entry.parts[0] {
            request.state = zeron_proto::RequestState::Pending;
        }
        entry.status = Some(MessageStatus::Aborted);
        assert!(pending_permission_request(&[entry.clone()]).is_none());
        entry.status = Some(MessageStatus::Streaming);
        let mut newer = entry.clone();
        newer.parts.clear();
        assert!(pending_permission_request(&[entry, newer]).is_none());
    }
}
