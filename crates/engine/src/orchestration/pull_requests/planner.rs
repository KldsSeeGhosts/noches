use super::{PrOperation, chains, links_of, new_link};
use crate::orchestration::{
    Error, Result,
    command::{Command, Plan},
    event::iso,
    projection, task,
};
use rusqlite::Connection;
use zeron_proto::orchestration::*;

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    operation: &PrOperation,
    now: i64,
) -> Result<Plan> {
    let projection = projection::read_thread(conn, &command.thread_id)?
        .ok_or_else(|| Error::Invariant(format!("Thread {} was not found.", command.thread_id)))?;
    let mut thread = projection.thread.clone();
    let mut links = links_of(&thread);
    let mut plan = Plan::default();
    let time = iso(now)?;
    let mut event = "thread.metadata-updated";
    if let PrOperation::Settle {
        expected_sequence,
        settled_at,
        auto_merge,
        after_days,
    } = operation
    {
        if projection.through_sequence != *expected_sequence {
            return Err(Error::Invariant("Thread changed before settlement.".into()));
        }
        // The selector is revalidated at the transaction boundary.
        let candidate = super::settlement::candidate(&projection);
        if super::settlement::resolve(&candidate, &links, now, *auto_merge, *after_days).is_none() {
            return Err(Error::Invariant(
                "Thread is no longer an automatic settlement candidate.".into(),
            ));
        }
        thread.settled_at = Some(settled_at.clone());
        thread.settled_override = Some(OrchestrationV2AppThreadSettledOverride::Settled);
        thread.unsettled_at = Optional::Present(None);
        thread.pinned_at = Optional::Present(None);
        thread.pin_order_key = Optional::Present(None);
        thread.active_order_key = Optional::Present(None);
        thread.updated_at = time.clone();
        event = "thread.settled";
    } else {
        let target = match operation {
            PrOperation::Link { target, .. }
            | PrOperation::Unlink { target }
            | PrOperation::Watch { target, .. }
            | PrOperation::Sync { target, .. }
            | PrOperation::WatchSync { target, .. } => target,
            _ => unreachable!(),
        };
        let index = links
            .iter()
            .position(|l| chains::identity(l).key() == target.key());
        match operation {
            PrOperation::Link { source, .. } => {
                let undismiss = index.is_some_and(|i| {
                    links[i].source == ThreadPullRequestLinkSource::StackDismissed
                }) && !matches!(
                    source,
                    ThreadPullRequestLinkSource::Stack
                        | ThreadPullRequestLinkSource::StackDismissed
                );
                if index.is_none() || undismiss {
                    let mut link = index
                        .map(|i| links.remove(i))
                        .unwrap_or_else(|| new_link(target, source.clone(), &time));
                    link.source = source.clone();
                    link.url = target.url.clone();
                    if *source == ThreadPullRequestLinkSource::Manual
                        && links.iter().all(|l| !chains::visible(&l))
                        && let Some(Some(branch)) = thread.branch_pull_request.as_ref()
                        && let Some(branch) = super::identity::parse_url(&branch.url)
                        && branch.key() != target.key()
                        && !links
                            .iter()
                            .any(|l| chains::identity(l).key() == branch.key())
                    {
                        links.insert(
                            0,
                            new_link(&branch, ThreadPullRequestLinkSource::Manual, &time),
                        );
                    }
                    links.push(link);
                    thread.updated_at = time.clone();
                }
            }
            PrOperation::Unlink { .. } => {
                if let Some(i) = index {
                    let stack = links[i].source == ThreadPullRequestLinkSource::Stack
                        || links[i].stack.is_some()
                        || links.iter().any(|l| {
                            let key = chains::identity(l);
                            key.host == target.host
                                && key.repository == target.repository
                                && l.stack.as_ref().is_some_and(|s| {
                                    s.layers.iter().any(|l| l.number == target.number)
                                })
                        });
                    if stack {
                        links[i].source = ThreadPullRequestLinkSource::StackDismissed;
                        links[i].watch = Optional::Absent;
                    } else {
                        links.remove(i);
                    }
                }
                thread.updated_at = time.clone();
            }
            PrOperation::Watch { watching, .. } => {
                let before_links = links.clone();
                let mut index = index
                    .filter(|i| links[*i].source != ThreadPullRequestLinkSource::StackDismissed);
                if *watching && index.is_none() {
                    links.retain(|l| chains::identity(l).key() != target.key());
                    links.push(new_link(target, ThreadPullRequestLinkSource::Agent, &time));
                    index = Some(links.len() - 1);
                }
                if let Some(i) = index {
                    if *watching
                        && links[i]
                            .snapshot
                            .as_ref()
                            .is_some_and(|s| s.state != PullRequestState::Open)
                    {
                        return Err(Error::Invariant("The pull request is not open.".into()));
                    }
                    links[i].watch = if !watching {
                        Optional::Absent
                    } else {
                        Optional::Present(links[i].watch.as_ref().cloned().unwrap_or_else(|| {
                            ThreadPullRequestWatch {
                                started_at: time.clone(),
                                head_sha: None,
                                failed_checks: vec![],
                                passed: false,
                                remarks_through: time.clone(),
                                remark_ids: vec![],
                                conflicting: false,
                                wakes: 0,
                            }
                        }))
                    };
                }
                if links != before_links {
                    thread.updated_at = time.clone();
                }
            }
            PrOperation::Sync {
                snapshot,
                stack,
                preserve_stack,
                ..
            } => {
                if let Some(i) = index {
                    // Discover all siblings before recording a terminal snapshot:
                    // unsynced/open layers block settlement of this whole thread.
                    if let Some(stack) = stack {
                        for layer in &stack.layers {
                            if links.iter().any(|l| {
                                let key = chains::identity(l);
                                key.host == target.host
                                    && key.repository == target.repository
                                    && l.number == layer.number
                            }) {
                                continue;
                            }
                            let mut sibling = target.clone();
                            sibling.number = layer.number;
                            sibling.url = super::host::sibling_url(&target.url, layer.number)
                                .ok_or_else(|| Error::Invariant("Invalid stack URL.".into()))?;
                            links.push(new_link(
                                &sibling,
                                ThreadPullRequestLinkSource::Stack,
                                &time,
                            ));
                        }
                    }
                    links[i].snapshot = Some(*snapshot.clone());
                    if !preserve_stack {
                        links[i].stack = stack.clone();
                    }
                    event = "thread.pull-request-synced";
                }
            }
            PrOperation::WatchSync {
                started_at,
                watch,
                wake,
                ..
            } => {
                let i = index
                    .filter(|i| {
                        links[*i].source != ThreadPullRequestLinkSource::StackDismissed
                            && links[*i]
                                .watch
                                .as_ref()
                                .is_some_and(|w| w.started_at == *started_at)
                    })
                    .ok_or_else(|| {
                        Error::Invariant(
                            "The pull request watch ended or its thread settled while it was read."
                                .into(),
                        )
                    })?;
                if wake.is_some()
                    && (thread.archived_at.is_some()
                        || thread.deleted_at.is_some()
                        || thread.settled_at.is_some()
                        || thread.settled_override
                            == Some(OrchestrationV2AppThreadSettledOverride::Settled)
                        || (thread.lineage.relationship_to_parent
                            == Some(OrchestrationV2AppThreadLineageRelationshipToParent::Subagent)
                            && thread.creation_source == OrchestrationV2CreationSource::Provider))
                {
                    return Err(Error::Invariant(
                        "The pull request watch ended or its thread settled while it was read."
                            .into(),
                    ));
                }
                links[i].watch = watch
                    .clone()
                    .map(Optional::Present)
                    .unwrap_or(Optional::Absent);
                if let Some(wake) = wake {
                    // T3 explicitly uses queue_after_active for PR news, not steering.
                    let ordinal = projection.runs.iter().map(|r| r.ordinal).max().unwrap_or(0) + 1;
                    let status = if task::active_run(&projection).is_some() {
                        "queued"
                    } else {
                        "starting"
                    };
                    let driver = task::records(&projection, "provider-thread")
                        .first()
                        .and_then(|p| p["driver"].as_str())
                        .unwrap_or("unknown");
                    let message_id = format!("message:pr-watch:{}", command.id.0);
                    let mut seed =
                        task::execution_seed(&thread, ordinal, &message_id, status, driver, now)?;
                    if status == "queued" {
                        seed.run.queue_position = Optional::Present(Some(ordinal));
                        seed.run.queue_held = Optional::Present(false);
                        plan.emit(command, "run.created", &seed.run, now)?;
                        plan.emit(command, "run-attempt.created", &seed.attempt, now)?;
                        plan.emit(command, "node.updated", &seed.root, now)?;
                    } else {
                        task::emit_execution(&mut plan, command, &thread.id, &seed, now)?;
                    }
                    let mut message = task::message(
                        &thread.id,
                        Some(&seed.run.id),
                        Some(&seed.root.id),
                        &message_id,
                        &wake.text,
                        "user",
                        now,
                    )?;
                    message["creationSource"] = serde_json::json!("server");
                    message["notification"] = wake.notification.clone();
                    plan.emit(command, "message.updated", &message, now)?;
                }
            }
            _ => unreachable!(),
        }
    }
    if let Some(Some(legacy)) = thread.linked_pull_request.as_ref() {
        let key = super::identity::parse_url(&legacy.url).map(|i| i.key());
        if !links
            .iter()
            .any(|l| chains::visible(&l) && Some(chains::identity(l).key()) == key)
        {
            thread.linked_pull_request = Optional::Present(None);
        }
    }
    thread.pull_requests = Optional::Present(links);
    plan.emit(command, event, &thread, now)?;
    Ok(plan)
}
