//! F1 owner routing and the root-checkout auto-pull read model.
use std::collections::HashMap;

use gpui::{Context, Task};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use zeron_proto::git_actions::{GitCheckout, PullPolicy, PullState};
use zeron_rpc::{RpcClient, RpcError, git_actions::methods};

use crate::state::AppState;

pub fn is_root_checkout(chat: &zeron_proto::Chat, space: &zeron_proto::Space) -> bool {
    match (&chat.checkout_id, &space.checkout_id) {
        (Some(chat), Some(root)) => chat == root,
        _ => {
            chat.cwd
                .as_deref()
                .unwrap_or(&space.path)
                .trim_end_matches('/')
                == space.path.trim_end_matches('/')
        }
    }
}

/// Always address the owning host, including when it is the connected host.
/// Typed F1 methods do not yet accept the routing envelope.
pub fn routed(owner: &str, request: impl Serialize) -> Value {
    let mut value = serde_json::to_value(request).expect("F1 request is serializable");
    value["targetDeviceId"] = json!(owner);
    value
}

pub async fn call<T: DeserializeOwned>(
    client: &RpcClient,
    owner: &str,
    method: &str,
    request: impl Serialize,
) -> Result<T, RpcError> {
    client.call_as(method, routed(owner, request)).await
}

pub fn skip_reason(reason: &str) -> String {
    match reason {
        "disabled" => "disabled",
        "checkout_not_idle" => "checkout busy",
        "not_main_checkout" => "not the root checkout",
        "checkout_locked" => "checkout locked",
        "not_default_branch" => "not on the default branch",
        "remote_changed" => "remote changed",
        "upstream_mismatch" => "upstream mismatch",
        "default_branch_unverified" => "default branch unverified",
        "changed_or_untracked_files" => "changed files",
        "local_commits" => "local commits",
        other => return other.replace('_', " "),
    }
    .into()
}

pub fn pull_summary(state: &PullState, now_ms: i64) -> String {
    if let Some(reason) = &state.last_skip_reason {
        return format!("skipped · {}", skip_reason(reason));
    }
    let elapsed = state.last_checked_at.map(|at| {
        let seconds = (now_ms.saturating_sub(at) / 1000).max(0);
        if seconds < 60 {
            "just now".into()
        } else if seconds < 3600 {
            format!("{}m ago", seconds / 60)
        } else {
            format!("{}h ago", seconds / 3600)
        }
    });
    match state.last_result.as_deref() {
        Some("pulled") => format!("pulled · {}", elapsed.unwrap_or_else(|| "just now".into())),
        Some("skipped_up_to_date") => format!(
            "up to date · checked {}",
            elapsed.unwrap_or_else(|| "just now".into())
        ),
        _ => "Not checked yet".into(),
    }
}

#[derive(Clone, Default)]
pub struct PullRow {
    pub state: PullState,
    pub checkout: Option<GitCheckout>,
    pub error: Option<String>,
    pub busy: bool,
}

#[derive(Default)]
pub struct GitStore {
    pub pulls: HashMap<String, PullRow>,
    watches: HashMap<String, Task<()>>,
}

impl AppState {
    pub fn ensure_pull_watch(&mut self, space_id: &str, cx: &mut Context<Self>) {
        if self.git_actions.watches.contains_key(space_id) {
            return;
        }
        let Some(space) = self.spaces.iter().find(|s| s.id == space_id).cloned() else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let key = space.id.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                let result: Result<PullState, _> = call(
                    engine.client(),
                    &space.device_id,
                    methods::GET_PULL,
                    json!({"spaceId": space.id}),
                )
                .await;
                let checkout: Result<GitCheckout, _> = call(
                    engine.client(),
                    &space.device_id,
                    methods::GET_GIT_STATUS,
                    json!({"cwd": space.path}),
                )
                .await;
                if this
                    .update(cx, |state, cx| {
                        let row = state.git_actions.pulls.entry(space.id.clone()).or_default();
                        match result {
                            Ok(value) => {
                                row.state = value;
                                row.error = None;
                            }
                            Err(error) => row.error = Some(error.to_string()),
                        }
                        match checkout {
                            Ok(value) => row.checkout = Some(value),
                            Err(error) => {
                                row.checkout = None;
                                row.error = Some(error.to_string());
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(30))
                    .await;
            }
        });
        self.git_actions.watches.insert(key, task);
    }

    pub fn change_pull(&mut self, space_id: &str, enabled: Option<bool>, cx: &mut Context<Self>) {
        let Some(space) = self.spaces.iter().find(|s| s.id == space_id).cloned() else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let Some(row) = self.git_actions.pulls.get_mut(space_id) else {
            return;
        };
        if row.busy {
            return;
        }
        let policy = if let Some(enabled) = enabled {
            if !enabled && !row.state.policy.default_branch.is_empty() {
                Some(PullPolicy {
                    enabled,
                    ..row.state.policy.clone()
                })
            } else {
                let Some(checkout) = &row.checkout else {
                    return;
                };
                let Some(upstream) = &checkout.upstream else {
                    row.error = Some("This checkout has no upstream.".into());
                    cx.notify();
                    return;
                };
                let Some((remote, branch)) = upstream.split_once('/') else {
                    return;
                };
                let Some(remote) = checkout.remotes.iter().find(|r| r.name == remote) else {
                    return;
                };
                Some(PullPolicy {
                    space_id: space.id.clone(),
                    enabled,
                    remote: remote.clone(),
                    default_branch: branch.into(),
                    cadence_seconds: 300,
                })
            }
        } else {
            None
        };
        row.busy = true;
        let (method, request) = match policy {
            Some(policy) => (methods::SET_PULL, serde_json::to_value(policy).unwrap()),
            None => (methods::RETRY_PULL, json!({"spaceId": space.id})),
        };
        cx.spawn(async move |this, cx| {
            let result: Result<PullState, _> =
                call(engine.client(), &space.device_id, method, request).await;
            this.update(cx, |state, cx| {
                let row = state.git_actions.pulls.entry(space.id).or_default();
                row.busy = false;
                match result {
                    Ok(value) => {
                        row.state = value;
                        row.error = None;
                    }
                    Err(error) => row.error = Some(error.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skip_reasons_are_words_not_protocol_tokens() {
        for (token, words) in [
            ("changed_or_untracked_files", "changed files"),
            ("checkout_not_idle", "checkout busy"),
            ("not_main_checkout", "not the root checkout"),
            ("checkout_locked", "checkout locked"),
            ("not_default_branch", "not on the default branch"),
            ("remote_changed", "remote changed"),
            ("upstream_mismatch", "upstream mismatch"),
            ("default_branch_unverified", "default branch unverified"),
            ("local_commits", "local commits"),
            ("disabled", "disabled"),
        ] {
            assert_eq!(skip_reason(token), words);
        }
        assert_eq!(skip_reason("new_reason"), "new reason");
    }
    #[test]
    fn routing_never_falls_back_to_viewer() {
        assert_eq!(
            routed("host", json!({"cwd":"/repo"}))["targetDeviceId"],
            "host"
        );
    }
    #[test]
    fn pull_copy_uses_engine_result_and_checked_time() {
        let mut state = PullState {
            last_checked_at: Some(0),
            last_result: Some("skipped_up_to_date".into()),
            ..Default::default()
        };
        assert_eq!(pull_summary(&state, 120000), "up to date · checked 2m ago");
        state.last_result = Some("pulled".into());
        assert_eq!(pull_summary(&state, 180000), "pulled · 3m ago");
        state.last_skip_reason = Some("changed_or_untracked_files".into());
        assert_eq!(pull_summary(&state, 180000), "skipped · changed files");
    }
}
