//! Scheduled tasks ("Automations", T3 P7) as the desktop reads them: one live
//! watch per owner device (`WatchScheduledTasks`), decoded into the few fields
//! the Details panel and Settings › Automations draw. Mutations are typed,
//! owner-routed calls; an offline owner fails at the transport and nothing
//! ever runs on the viewer.

use chrono::{DateTime, Utc};
use gpui::{Context, Task};
use serde_json::Value;
use zeron_proto::scheduler::{
    ScheduledTaskActionRequest, ScheduledTaskUpdateRequest, ScheduledTasksListRequest,
};

use crate::details::RunStatus;
use crate::state::AppState;

#[derive(Clone, Debug, PartialEq)]
pub struct AutomationRow {
    pub id: String,
    pub owner_host_id: String,
    pub title: String,
    pub prompt: String,
    pub project_id: String,
    /// Bound thread (= chat id); `None` launches a fresh thread per run.
    pub thread_id: Option<String>,
    pub enabled: bool,
    pub cadence: String,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run: RunStatus,
    pub last_error: Option<String>,
}

fn time(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
}

/// Decode one `ScheduledTasksView` snapshot. Rows that don't parse are
/// skipped rather than failing the whole list.
pub fn decode_view(view: &Value) -> Vec<AutomationRow> {
    let owner = view["ownerHostId"].as_str().unwrap_or_default().to_string();
    view["tasks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            // `ScheduledTaskView` flattens the task beside `cadence` and
            // `lastRun`; accept the nested shape too.
            let task = row.get("task").filter(|t| t.is_object()).unwrap_or(row);
            Some(AutomationRow {
                id: task["id"].as_str()?.to_string(),
                owner_host_id: owner.clone(),
                title: task["title"].as_str().unwrap_or("Untitled").to_string(),
                prompt: task["prompt"].as_str().unwrap_or_default().to_string(),
                project_id: task["projectId"].as_str().unwrap_or_default().to_string(),
                thread_id: task["threadId"].as_str().map(str::to_string),
                enabled: task["enabled"].as_bool().unwrap_or(false),
                cadence: row["cadence"].as_str().unwrap_or_default().to_string(),
                next_run_at: time(&task["nextRunAt"]),
                last_run: match row["lastRun"]["status"].as_str() {
                    Some("running") => RunStatus::Running,
                    Some("succeeded") => RunStatus::Succeeded,
                    Some("failed") => RunStatus::Failed,
                    _ => RunStatus::Never,
                },
                last_error: row["lastRun"]["error"].as_str().map(str::to_string),
            })
        })
        .collect()
}

#[derive(Default)]
pub struct AutomationsStore {
    by_owner: std::collections::HashMap<String, Vec<AutomationRow>>,
    watches: std::collections::HashMap<String, Task<()>>,
    pub error: Option<String>,
}

impl AutomationsStore {
    pub fn rows(&self) -> impl Iterator<Item = &AutomationRow> {
        self.by_owner.values().flatten()
    }

    pub fn for_thread<'a>(&'a self, chat_id: &'a str) -> impl Iterator<Item = &'a AutomationRow> {
        self.rows()
            .filter(move |row| row.thread_id.as_deref() == Some(chat_id))
    }

    pub fn watching(&self, owner: &str) -> bool {
        self.watches.contains_key(owner)
    }

    pub fn find(&self, id: &str) -> Option<&AutomationRow> {
        self.rows().find(|row| row.id == id)
    }
}

impl AppState {
    /// Keep a live watch on `owner`'s scheduled tasks. Idempotent; the watch
    /// reconnects with a short backoff and stops with the engine handle.
    pub fn ensure_automations_watch(&mut self, owner: &str, cx: &mut Context<Self>) {
        if owner.is_empty() || self.automations.watches.contains_key(owner) {
            return;
        }
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let owner = owner.to_string();
        let key = owner.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                let request = ScheduledTasksListRequest {
                    owner_host_id: owner.clone(),
                    project_id: None,
                };
                if let Ok(mut updates) = engine.client().watch_scheduled_tasks(request).await {
                    while let Some(value) = updates.recv().await {
                        let rows = decode_view(&value);
                        if this
                            .update(cx, |state, cx| {
                                state.automations.by_owner.insert(owner.clone(), rows);
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(5))
                    .await;
            }
        });
        self.automations.watches.insert(key, task);
    }

    pub fn set_automation_enabled(&mut self, id: &str, enabled: bool, cx: &mut Context<Self>) {
        let Some(row) = self.automations.find(id).cloned() else {
            return;
        };
        // Optimistic: the next watch snapshot is the truth either way.
        if let Some(rows) = self.automations.by_owner.get_mut(&row.owner_host_id)
            && let Some(local) = rows.iter_mut().find(|r| r.id == id)
        {
            local.enabled = enabled;
        }
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let request: ScheduledTaskUpdateRequest = serde_json::from_value(serde_json::json!({
            "ownerHostId": row.owner_host_id,
            "id": row.id,
            "enabled": enabled,
        }))
        .expect("partial update request");
        cx.spawn(async move |this, cx| {
            if let Err(err) = engine.client().update_scheduled_task(request).await {
                this.update(cx, |state, cx| {
                    state.automations.error = Some(format!("Could not update automation: {err}"));
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }

    pub fn run_automation_now(&mut self, id: &str, cx: &mut Context<Self>) {
        self.automation_action(id, AutomationAction::RunNow, cx);
    }

    pub fn delete_automation(&mut self, id: &str, cx: &mut Context<Self>) {
        self.automation_action(id, AutomationAction::Delete, cx);
    }

    fn automation_action(&mut self, id: &str, action: AutomationAction, cx: &mut Context<Self>) {
        let (Some(row), Some(engine)) =
            (self.automations.find(id).cloned(), self.engine().cloned())
        else {
            return;
        };
        if action == AutomationAction::Delete
            && let Some(rows) = self.automations.by_owner.get_mut(&row.owner_host_id)
        {
            rows.retain(|r| r.id != id);
        }
        let request = ScheduledTaskActionRequest {
            owner_host_id: row.owner_host_id,
            id: row.id,
        };
        cx.spawn(async move |this, cx| {
            let result = match action {
                AutomationAction::RunNow => engine
                    .client()
                    .run_scheduled_task_now(request)
                    .await
                    .map(|_| ()),
                AutomationAction::Delete => engine
                    .client()
                    .delete_scheduled_task(request)
                    .await
                    .map(|_| ()),
            };
            if let Err(err) = result {
                this.update(cx, |state, cx| {
                    state.automations.error = Some(match action {
                        AutomationAction::RunNow => format!("Could not run automation: {err}"),
                        AutomationAction::Delete => format!("Could not delete automation: {err}"),
                    });
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutomationAction {
    RunNow,
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_scheduler_view() {
        let rows = decode_view(&serde_json::json!({
            "ownerHostId": "mac",
            "tasks": [{
                "id": "t1", "title": "Nightly audit", "prompt": "Audit deps", "enabled": true,
                "projectId": "space-1", "threadId": "chat-1", "nextRunAt": "2026-10-05T02:00:00Z",
                "cadence": "Every day 02:00",
                "lastRun": {"status": "failed", "error": "boom"}
            }, {"title": "no id"}]
        }));
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.owner_host_id, "mac");
        assert_eq!(row.thread_id.as_deref(), Some("chat-1"));
        assert_eq!(row.last_run, RunStatus::Failed);
        assert_eq!(row.last_error.as_deref(), Some("boom"));
        assert!(row.next_run_at.is_some());
    }
}
