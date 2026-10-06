//! Deterministic outbox workloads, independent of providers or a desktop.
use super::*;
use std::time::Instant;

struct DiscardPublisher;

#[async_trait]
impl ProjectionPublisher for DiscardPublisher {
    async fn publish(&self, _: &PublicationBatch, _: &PublicationDocument) -> Result<()> {
        Ok(())
    }
}

fn seed_outbox_history(store: &Store, count: usize) {
    store
        .write(|tx| {
            let mut effects = tx.prepare(
                "INSERT INTO orchestration_effect_outbox
                 (effect_id,command_id,thread_id,effect_type,lane,payload_json,
                  process_bound,status,available_at,created_at)
                 VALUES(?1,?1,'history','terminal.cleanup','provider','{}',0,'succeeded',0,0)",
            )?;
            let mut publications = tx.prepare(
                "INSERT INTO orchestration_publication_batches
                 (batch_id,host_id,host_epoch,through_sequence,status,payload_json)
                 VALUES(?1,'host',1,0,'published','{}')",
            )?;
            for index in 0..count {
                let id = format!("history-{index}");
                effects.execute([&id])?;
                publications.execute([&id])?;
            }
            Ok(())
        })
        .unwrap();
}

#[tokio::test]
#[ignore = "comparative optimized-test benchmark; run without concurrent builds"]
async fn profile_orchestration_outboxes() {
    let fixture = Fixture::new();
    let store = &fixture.kernel.store;
    seed_outbox_history(store, 50_000);
    let start = Instant::now();
    for _ in 0..1_000 {
        assert!(
            store
                .claim_effect("profile", NOW, 30_000)
                .unwrap()
                .is_none()
        );
    }
    let claim_us = start.elapsed().as_micros();
    let worker = PublicationWorker {
        store: store.clone(),
        publisher: Arc::new(DiscardPublisher),
    };
    let start = Instant::now();
    for _ in 0..1_000 {
        assert!(!worker.step().await.unwrap());
    }
    let empty_publication_us = start.elapsed().as_micros();

    // A pending backlog used to be decoded in its entirety for *each* step.
    // The publisher does no I/O, so the measurement isolates outbox overhead.
    store
        .write(|tx| {
            for index in 0..100 {
                let batch = PublicationBatch {
                    schema_version: 1,
                    batch_id: format!("backlog-{index}"),
                    host_id: "host".into(),
                    host_epoch: 1,
                    through_sequence: index,
                    documents: vec![PublicationDocument {
                        doc_id: format!("thread-{index}"),
                        version: index,
                        payload: json!({"text": "x".repeat(32 * 1024)}),
                    }],
                };
                tx.execute(
                    "INSERT INTO orchestration_publication_batches
                     (batch_id,host_id,host_epoch,through_sequence,status,payload_json)
                     VALUES(?1,'host',1,?2,'pending',?3)",
                    rusqlite::params![
                        batch.batch_id,
                        batch.through_sequence,
                        serde_json::to_string(&batch)?
                    ],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        assert!(worker.step().await.unwrap());
    }
    let backlog_us = start.elapsed().as_micros();
    assert!(!worker.step().await.unwrap());
    eprintln!(
        "outbox_profile history=50000 empty_claims=1000 claim_us={claim_us} \
         empty_publications=1000 empty_publication_us={empty_publication_us} \
         backlog_batches=100 batch_bytes=32768 backlog_us={backlog_us}"
    );
}
