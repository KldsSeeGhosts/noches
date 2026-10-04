//! Host-only publication seam for Loro/chat2. The adapter applies stable entity
//! IDs and monotonic versions, then returns; only then is a SQL ack committed.
//! No effect, credential, or live runtime-request payload is replicated.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeron_proto::orchestration::ThreadId;

use super::projection::{self, SCHEMA_VERSION, decode};
use super::store::{Store, WriteBoundary};
use super::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationDocument {
    pub doc_id: String,
    pub version: i64,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationBatch {
    pub schema_version: i64,
    pub batch_id: String,
    pub host_id: String,
    pub host_epoch: i64,
    pub through_sequence: i64,
    /// All referenced versions must arrive before actionable reads are enabled.
    pub documents: Vec<PublicationDocument>,
}

pub(crate) fn enqueue(
    conn: &Connection,
    host_id: &str,
    identity: &zeron_proto::orchestration::CommandId,
    through_sequence: i64,
    threads: BTreeSet<String>,
) -> Result<()> {
    let epoch = conn.query_row(
        "SELECT epoch FROM orchestration_host WHERE singleton=1",
        [],
        |row| row.get(0),
    )?;
    let mut documents = vec![];
    let mut summaries = vec![];
    for id in threads {
        let projection = projection::read_thread(conn, &ThreadId(id.clone()))?
            .ok_or_else(|| Error::Invariant("publication without projection".into()))?;
        let thread = &projection.thread;
        summaries.push(serde_json::json!({
            "id":thread.id,"projectId":thread.project_id,"title":thread.title,
            "lineage":thread.lineage,"archivedAt":thread.archived_at,"deletedAt":thread.deleted_at,
            "lastVisitedAt":thread.last_visited_at,
            "version":projection.through_sequence,
            "lifecycle":super::ui_queue::state(conn,&ThreadId(id.clone()))?.lifecycle
        }));
        documents.push(PublicationDocument {
            doc_id: format!("orchestration/thread/{id}"),
            version: projection.through_sequence,
            payload: {
                let mut payload = serde_json::to_value(&projection)?;
                if let Some(records) = payload["records"].as_object_mut() {
                    // Runtime bindings/callback authority are host-local.
                    // Task/message/transfer publication must not accidentally
                    // replicate the kernel's broader internal read model.
                    records.remove("provider-session");
                    records.remove("runtime-request");
                }
                payload["uiState"] = super::ui::state(conn, &ThreadId(id.clone()))?;
                payload
            },
        });
    }
    documents.push(PublicationDocument {
        doc_id: "orchestration/registry".into(),
        version: through_sequence,
        payload: serde_json::json!({"threads":summaries}),
    });
    let batch = PublicationBatch {
        schema_version: SCHEMA_VERSION,
        batch_id: format!(
            "publication:{}",
            super::event::encode_component(&identity.0)
        ),
        host_id: host_id.into(),
        host_epoch: epoch,
        through_sequence,
        documents,
    };
    conn.execute(
        "INSERT INTO orchestration_publication_batches
         (batch_id,host_id,host_epoch,through_sequence,status,payload_json)
         VALUES(?1,?2,?3,?4,'pending',?5)",
        params![
            batch.batch_id,
            host_id,
            epoch,
            batch.through_sequence,
            serde_json::to_string(&batch)?
        ],
    )?;
    Ok(())
}

impl Store {
    pub fn pending_publications(&self) -> Result<Vec<PublicationBatch>> {
        self.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT payload_json FROM orchestration_publication_batches WHERE status='pending' ORDER BY ordinal",
            )?;
            stmt.query_map([], |row| row.get::<_, String>(0))?
                .map(|row| decode(&row?)).collect()
        })
    }

    pub fn acknowledge_publication(
        &self,
        batch_id: &str,
        doc_id: &str,
        version: i64,
    ) -> Result<()> {
        self.write(|tx| {
            let raw: Option<String> = tx.query_row(
                "SELECT payload_json FROM orchestration_publication_batches WHERE batch_id=?1",
                [batch_id], |row| row.get(0),
            ).optional()?;
            let batch: PublicationBatch = decode(&raw.ok_or_else(|| Error::Invariant("unknown publication".into()))?)?;
            if batch.host_id != self.host_id.as_ref()
                || !batch.documents.iter().any(|doc| doc.doc_id == doc_id && doc.version == version)
            {
                return Err(Error::Invariant("publication ack does not match its barrier".into()));
            }
            tx.execute(
                "INSERT OR IGNORE INTO orchestration_publication_acks VALUES(?1,?2,?3)",
                params![batch_id, doc_id, version],
            )?;
            self.boundary(WriteBoundary::PublicationAck)?;
            let count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM orchestration_publication_acks WHERE batch_id=?1",
                [batch_id], |row| row.get(0),
            )?;
            if count == batch.documents.len() as i64 {
                tx.execute(
                    "UPDATE orchestration_publication_batches SET status='published' WHERE batch_id=?1",
                    [batch_id],
                )?;
                self.boundary(WriteBoundary::PublicationBatchAck)?;
            }
            Ok(())
        })
    }

    fn publication_acked(&self, batch: &str, doc: &str) -> Result<bool> {
        self.read(|conn| Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM orchestration_publication_acks WHERE batch_id=?1 AND doc_id=?2)",
            params![batch, doc], |row| row.get(0),
        )?))
    }
}

#[async_trait]
pub trait ProjectionPublisher: Send + Sync {
    /// Must reject foreign owners, apply stable IDs idempotently and enforce
    /// monotonic host-epoch/version. Batch barrier metadata accompanies each doc.
    /// Registry payloads are patches, not replacements of unrelated threads.
    async fn publish(&self, batch: &PublicationBatch, document: &PublicationDocument)
    -> Result<()>;
}

pub struct PublicationWorker {
    pub store: Store,
    pub publisher: Arc<dyn ProjectionPublisher>,
}

impl PublicationWorker {
    pub async fn step(&self) -> Result<bool> {
        let _lane = self.store.publication_lane.lock().await;
        let Some(batch) = self.store.pending_publications()?.into_iter().next() else {
            return Ok(false);
        };
        for document in &batch.documents {
            if self
                .store
                .publication_acked(&batch.batch_id, &document.doc_id)?
            {
                continue;
            }
            // A failure/crash here leaves the durable batch pending. The next
            // call republishes the same version, never creates another message.
            self.publisher.publish(&batch, document).await?;
            self.store.acknowledge_publication(
                &batch.batch_id,
                &document.doc_id,
                document.version,
            )?;
        }
        Ok(true)
    }
}

/// Replica-side read barrier. This type has no execution/dispatch methods.
/// The authenticated sync owner, not an arbitrary incoming doc, supplies owner.
#[derive(Debug)]
pub struct ReplicaBarrier {
    owner: String,
    epoch: i64,
    versions: BTreeMap<String, i64>,
}

impl ReplicaBarrier {
    pub fn new(owner: String, epoch: i64) -> Self {
        Self {
            owner,
            epoch,
            versions: BTreeMap::new(),
        }
    }

    pub fn observe(&mut self, batch: &PublicationBatch, document: &PublicationDocument) -> bool {
        if batch.host_id != self.owner
            || batch.schema_version != SCHEMA_VERSION
            || batch.host_epoch < self.epoch
            || !batch
                .documents
                .iter()
                .any(|candidate| candidate == document)
        {
            return false;
        }
        if batch.host_epoch > self.epoch {
            self.versions.clear();
            self.epoch = batch.host_epoch;
        }
        let version = self.versions.entry(document.doc_id.clone()).or_insert(-1);
        if document.version <= *version {
            return false;
        }
        *version = document.version;
        true
    }

    pub fn ready(&self, batch: &PublicationBatch) -> bool {
        batch.host_id == self.owner
            && batch.host_epoch == self.epoch
            && batch.schema_version == SCHEMA_VERSION
            && batch.documents.iter().all(|doc| {
                self.versions
                    .get(&doc.doc_id)
                    .is_some_and(|version| *version >= doc.version)
            })
    }
}
