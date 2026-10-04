//! T3 settlement selector, separated from host lookups and user organization.
use super::watch::millis;
use crate::orchestration::{projection::ThreadProjection, task};
use serde_json::Value;
use zeron_proto::orchestration::{PullRequestState, ThreadPullRequestLink};

#[derive(Debug, Clone, Default)]
pub struct Candidate {
    pub created_at: Option<i64>,
    pub user_at: Option<i64>,
    pub requested_at: Option<i64>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub has_run: bool,
    pub failed: bool,
    pub archived: bool,
    pub override_present: bool,
    pub pinned: bool,
    pub auto_disabled: bool,
    pub pending_request: bool,
    pub active: bool,
    pub background: bool,
    pub snoozed_until: Option<i64>,
    pub snoozed_at: Option<i64>,
}

pub fn candidate(projection: &ThreadProjection) -> Candidate {
    let thread = serde_json::to_value(&projection.thread).expect("thread");
    let last = projection.runs.last();
    let timestamp = |name: &str| thread[name].as_str().and_then(millis);
    Candidate {
        created_at: timestamp("createdAt"),
        // Monitor wakes are not user engagement.
        user_at: task::records(projection, "message")
            .iter()
            .filter(|m| m["role"] == "user" && m["notification"].is_null())
            .filter_map(|m| m["createdAt"].as_str().and_then(millis))
            .max(),
        requested_at: last.and_then(|r| millis(&r.requested_at)),
        started_at: last.and_then(|r| r.started_at.as_deref()).and_then(millis),
        completed_at: last
            .and_then(|r| r.completed_at.as_deref())
            .and_then(millis),
        has_run: last.is_some(),
        failed: last.is_some_and(|r| {
            r.status == zeron_proto::orchestration::OrchestrationV2RunStatus::Failed
        }),
        archived: !thread["archivedAt"].is_null() || !thread["deletedAt"].is_null(),
        override_present: !thread["settledOverride"].is_null(),
        pinned: timestamp("pinnedAt").is_some(),
        auto_disabled: timestamp("autoSettleDisabledAt").is_some(),
        pending_request: task::records(projection, "runtime-request")
            .iter()
            .any(|r| r["status"] == "pending"),
        active: task::active_run(projection).is_some(),
        background: task::records(projection, "provider-thread")
            .iter()
            .any(|p| {
                p["pendingBackgroundTasks"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty())
            })
            || task::records(projection, "subagent")
                .iter()
                .any(|s| s["status"].as_str().is_some_and(|s| !task::terminal(s))),
        snoozed_until: timestamp("snoozedUntil"),
        snoozed_at: timestamp("snoozedAt"),
    }
}

pub fn queued_start(c: &Candidate, now: i64) -> bool {
    let Some(at) = c.user_at else {
        return false;
    };
    if c.failed || now.abs_diff(at) > 120_000 {
        return false;
    }
    !c.has_run
        || [c.requested_at, c.started_at, c.completed_at]
            .iter()
            .all(|v| v.is_none_or(|v| v < at))
}

pub fn eligible(c: &Candidate, now: i64) -> bool {
    if c.archived
        || c.override_present
        || c.pinned
        || c.auto_disabled
        || c.pending_request
        || c.active
        || c.background
        || queued_start(c, now)
    {
        return false;
    }
    if c.snoozed_until.is_none_or(|until| until <= now) {
        return true;
    }
    let completed_after = c
        .snoozed_at
        .zip(c.completed_at)
        .is_some_and(|(s, end)| end > s);
    (c.failed && (c.snoozed_at.is_none() || completed_after)) || completed_after
}

pub fn resolve(
    c: &Candidate,
    links: &[ThreadPullRequestLink],
    now: i64,
    auto_merge: bool,
    after_days: Option<i64>,
) -> Option<i64> {
    let links: Vec<_> = links.iter().filter(super::chains::visible).collect();
    if links.iter().any(|l| {
        l.snapshot
            .as_ref()
            .is_none_or(|s| s.state == PullRequestState::Open)
    }) {
        return None;
    }
    if !eligible(c, now) {
        return None;
    }
    let activity = [c.user_at, c.requested_at, c.started_at, c.completed_at]
        .into_iter()
        .flatten()
        .max();
    let anchor = [c.created_at, c.user_at, c.requested_at]
        .into_iter()
        .flatten()
        .max()?;
    let latest = links
        .iter()
        .filter_map(|l| l.snapshot.as_ref())
        .map(|s| {
            let value: Value = serde_json::to_value(s).unwrap();
            let at = value[if s.state == PullRequestState::Merged {
                "mergedAt"
            } else {
                "closedAt"
            }]
            .as_str()
            .and_then(millis);
            (s.state.clone(), at)
        })
        .max_by_key(|(_, at)| *at);
    if let Some((state, Some(at))) = latest
        && at >= anchor
        && (state == PullRequestState::Closed || (state == PullRequestState::Merged && auto_merge))
    {
        return activity.or(c.created_at);
    }
    activity.filter(|at| after_days.is_some_and(|days| *at < now - days * 86_400_000))
}
