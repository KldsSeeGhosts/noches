//! Scheduling and retained-history regressions; no provider/network required.
use super::*;

fn enqueue_at(store: &Store, thread: &str, id: &str, request: &EffectRequest, at: i64) {
    store
        .write(|tx| {
            effects::enqueue(
                tx,
                id,
                &CommandId(id.into()),
                &ThreadId(thread.into()),
                request,
                at,
            )
        })
        .unwrap();
}

async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("worker made no progress");
}

async fn stop_workers(stop: CancellationToken, workers: Vec<tokio::task::JoinHandle<()>>) {
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(1), async {
        for worker in workers {
            worker.await.unwrap();
        }
    })
    .await
    .expect("idle workers did not shut down promptly");
}

#[test]
fn only_committed_row_changes_notify_workers_including_lost_responses() {
    let fixture = Fixture::new();
    fixture.create("t");
    let store = &fixture.kernel.store;
    let mut changes = store.subscribe_writes();
    store.thread(&ThreadId("t".into())).unwrap();
    store.write(|_| Ok(())).unwrap();
    store.claim_effect("empty", NOW, 100).unwrap();
    assert!(!store.cancel_effect("missing", NOW).unwrap());
    // Replaying an accepted command also makes no row changes.
    store.dispatch(&create_thread("t"), NOW).unwrap();
    assert!(!changes.has_changed().unwrap());

    store.inject_failure(WriteBoundary::BeforeCommit, 1);
    assert!(
        store
            .write(|tx| {
                tx.execute("UPDATE orchestration_host SET epoch=epoch+1", [])?;
                Ok(())
            })
            .is_err()
    );
    assert!(!changes.has_changed().unwrap(), "rollback is not work");

    store.inject_failure(WriteBoundary::AfterCommit, 1);
    assert!(
        store
            .write(|tx| {
                tx.execute("UPDATE orchestration_host SET epoch=epoch+1", [])?;
                Ok(())
            })
            .is_err()
    );
    assert!(changes.has_changed().unwrap(), "durable work must wake");
    changes.borrow_and_update();
    store.claim_effect("empty", NOW, 100).unwrap();
    assert!(!changes.has_changed().unwrap());
}

#[tokio::test]
async fn commit_between_empty_probe_and_wait_is_not_lost() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    let mut changes = store.subscribe_writes();
    changes.borrow_and_update();
    assert!(store.claim_effect("empty", NOW, 100).unwrap().is_none());
    enqueue_at(store, "t", "racing", &EffectRequest::TerminalCleanup, NOW);
    assert!(
        tokio::time::timeout(
            Duration::from_millis(100),
            super::super::wake::wait(
                &mut changes,
                &CancellationToken::new(),
                Duration::from_secs(30),
            )
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn all_five_outbox_workers_park_at_idle_and_wake_on_new_work() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    let executor = Arc::new(FakeExecutor::default());
    let publisher = Arc::new(FakePublisher::default());
    let stop = CancellationToken::new();
    let mut workers: Vec<_> = (0..DEFAULT_WORKER_CONCURRENCY)
        .map(|index| {
            EffectWorker::new(store.clone(), executor.clone(), format!("idle-{index}"))
                .spawn(stop.clone())
        })
        .collect();
    workers.push(
        PublicationWorker {
            store: store.clone(),
            publisher: publisher.clone(),
        }
        .spawn(stop.clone()),
    );
    until(|| store.access_counts() == [5, 4]).await;
    let parked = store.access_counts();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        store.access_counts(),
        parked,
        "idle workers must not keep probing the database"
    );
    assert!(executor.calls.lock().unwrap().is_empty());
    assert_eq!(*publisher.writes.lock().unwrap(), 0);

    enqueue_at(
        store,
        "t",
        "wake",
        &EffectRequest::TerminalCleanup,
        crate::now_ms(),
    );
    fixture.create("published");
    until(|| {
        !executor.accepted.lock().unwrap().is_empty() && *publisher.writes.lock().unwrap() == 2
    })
    .await;
    assert_eq!(*executor.calls.lock().unwrap(), ["wake"]);
    stop_workers(stop, workers).await;
}

#[test]
fn deadlines_obey_retry_fifo_and_uncertainty_barriers_without_spinning() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    assert_eq!(store.next_effect_wake().unwrap(), None);
    enqueue_at(
        store,
        "a",
        "head",
        &EffectRequest::TerminalCleanup,
        NOW + 100,
    );
    enqueue_at(store, "a", "follower", &EffectRequest::TerminalCleanup, NOW);
    assert_eq!(store.next_effect_wake().unwrap(), Some(NOW + 100));
    assert!(store.claim_effect("w", NOW, 20).unwrap().is_none());
    let head = store.claim_effect("w", NOW + 100, 20).unwrap().unwrap();
    assert_eq!(head.id, "head");
    assert_eq!(store.next_effect_wake().unwrap(), Some(NOW + 120));
    store.begin_effect(&head, NOW + 100).unwrap();
    store
        .finish_effect(&head, &EffectOutcome::Uncertain, NOW + 101, 5)
        .unwrap();
    assert_eq!(store.next_effect_wake().unwrap(), None);
    enqueue_at(
        store,
        "b",
        "other",
        &EffectRequest::TerminalCleanup,
        NOW + 50,
    );
    assert_eq!(store.next_effect_wake().unwrap(), Some(NOW + 50));
    store.cancel_effect("other", NOW).unwrap();
    assert_eq!(store.next_effect_wake().unwrap(), None);
    store.resolve_uncertain("head", false, NOW + 102).unwrap();
    assert_eq!(store.next_effect_wake().unwrap(), Some(NOW));
}

#[tokio::test]
async fn sleeping_worker_retries_at_deadline_and_then_releases_its_follower() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    let executor = Arc::new(FakeExecutor::default());
    executor
        .outcomes
        .lock()
        .unwrap()
        .push_back(EffectOutcome::Retry);
    let now = crate::now_ms();
    enqueue_at(store, "t", "retry", &EffectRequest::TerminalCleanup, now);
    enqueue_at(store, "t", "follower", &EffectRequest::TerminalCleanup, now);
    let stop = CancellationToken::new();
    let worker =
        EffectWorker::new(store.clone(), executor.clone(), "deadline".into()).spawn(stop.clone());
    until(|| store.effect("retry").unwrap().unwrap().attempt_count == 2).await;
    until(|| executor.accepted.lock().unwrap().len() == 2).await;
    assert_eq!(
        *executor.calls.lock().unwrap(),
        ["retry", "retry", "follower"]
    );
    stop_workers(stop, vec![worker]).await;
}

#[tokio::test]
async fn sleeping_worker_recovers_expired_claim_without_another_commit() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    let now = crate::now_ms();
    enqueue_at(store, "t", "expired", &EffectRequest::TerminalCleanup, now);
    store.claim_effect("gone", now, 80).unwrap().unwrap();
    let executor = Arc::new(FakeExecutor::default());
    let stop = CancellationToken::new();
    let worker = EffectWorker::new(store.clone(), executor.clone(), "replacement".into())
        .spawn(stop.clone());
    until(|| executor.accepted.lock().unwrap().len() == 1).await;
    assert_eq!(*executor.calls.lock().unwrap(), ["expired"]);
    assert_eq!(store.effect("expired").unwrap().unwrap().attempt_count, 2);
    stop_workers(stop, vec![worker]).await;
}

#[tokio::test]
async fn malformed_later_publication_cannot_poison_the_first_valid_batch() {
    let fixture = Fixture::new();
    fixture.create("t");
    fixture
        .kernel
        .store
        .write(|tx| {
            tx.execute(
                "INSERT INTO orchestration_publication_batches
                 (batch_id,host_id,host_epoch,through_sequence,status,payload_json)
                 VALUES('malformed','host',1,0,'pending','not-json')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let publisher = Arc::new(FakePublisher::default());
    let worker = PublicationWorker {
        store: fixture.kernel.store.clone(),
        publisher: publisher.clone(),
    };
    assert!(worker.step().await.unwrap());
    assert_eq!(*publisher.writes.lock().unwrap(), 2);
    assert!(
        worker.step().await.is_err(),
        "bad head must still fail closed"
    );
}

#[test]
fn effect_deadline_is_capped_for_external_repair_and_handles_past_times() {
    use super::super::wake::{REPAIR_INTERVAL, effect_delay};
    assert_eq!(effect_delay(None, NOW), REPAIR_INTERVAL);
    assert_eq!(effect_delay(Some(NOW + 60_000), NOW), REPAIR_INTERVAL);
    assert_eq!(effect_delay(Some(NOW + 17), NOW), Duration::from_millis(17));
    assert_eq!(effect_delay(Some(NOW - 1), NOW), Duration::ZERO);
}
