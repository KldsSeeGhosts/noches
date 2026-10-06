//! Event-driven outbox scheduling with a repair bound for independent handles.
use std::time::Duration;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub(super) const REPAIR_INTERVAL: Duration = Duration::from_secs(30);
pub(super) const ERROR_BACKOFF: Duration = Duration::from_millis(100);

pub(super) fn effect_delay(deadline: Option<i64>, now: i64) -> Duration {
    deadline
        .map(|at| Duration::from_millis(at.saturating_sub(now).max(0) as u64))
        .unwrap_or(REPAIR_INTERVAL)
        .min(REPAIR_INTERVAL)
}

/// The caller marks `changes` seen BEFORE querying work, never after.
/// Returns false on shutdown (including loss of the Store).
pub(super) async fn wait(
    changes: &mut watch::Receiver<()>,
    stop: &CancellationToken,
    delay: Duration,
) -> bool {
    tokio::select! {
        biased;
        _ = stop.cancelled() => false,
        result = changes.changed() => result.is_ok(),
        _ = tokio::time::sleep(delay) => true,
    }
}
