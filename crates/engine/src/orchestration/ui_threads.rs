//! Passive thread summaries/timeline; unlike MCP read, never observes a task.
use zeron_proto::orchestration::ThreadId;
use zeron_proto::orchestration_threads::*;

use super::{Error, Result, Store, threads::timeline};

impl Store {
    pub fn ui_thread_summaries(&self, input: ThreadSummariesRequest) -> Result<ThreadSummaries> {
        Ok(ThreadSummaries {
            threads: self.thread_summaries(&input.project_id)?,
        })
    }

    pub fn ui_thread_timeline(&self, input: ThreadTimelineRequest) -> Result<ThreadTimeline> {
        if input.limit.as_ref().is_some_and(|n| !(1..=100).contains(n))
            || input
                .run_limit
                .as_ref()
                .is_some_and(|n| !(1..=50).contains(n))
            || input
                .max_chars_per_item
                .as_ref()
                .is_some_and(|n| !(1..=50_000).contains(n))
            || input.after_position.as_ref().is_some_and(|n| *n < 0)
            || input.text_offset.as_ref().is_some_and(|n| *n < 0)
        {
            return Err(Error::Invariant("Invalid thread pagination.".into()));
        }
        let target = self
            .thread(&ThreadId(input.thread_id.clone()))?
            .filter(|t| t.thread.deleted_at.is_none())
            .ok_or_else(|| Error::Invariant("Thread was not found.".into()))?;
        let (page, _) = timeline::page(self, &target, &input)?;
        Ok(ThreadTimeline {
            version: target.through_sequence,
            page: page.result,
        })
    }
}
