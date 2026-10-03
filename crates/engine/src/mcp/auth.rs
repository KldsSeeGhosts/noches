//! Opaque bearer capabilities. Only SHA-256 digests are retained; no SQL/Loro.
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use sha2::{Digest, Sha256};
use zeron_proto::provider_instance::ModelSelection;

use crate::orchestration::service::CallerScope;

pub const LIVENESS_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone)]
pub struct InvocationScope {
    pub environment_id: String,
    pub caller: CallerScope,
    pub selection: ModelSelection,
    pub capabilities: BTreeSet<String>,
    pub issued_at: u64,
    /// Optional narrower task capability; direct-parent checks remain domain-owned.
    pub task_id: Option<String>,
}

struct Record {
    scope: InvocationScope,
    last_alive_at: u64,
}

pub struct CredentialRegistry {
    records: Mutex<HashMap<[u8; 32], Record>>,
    now: Arc<dyn Fn() -> u64 + Send + Sync>,
    window_ms: u64,
}

impl Default for CredentialRegistry {
    fn default() -> Self {
        Self::new(
            LIVENESS_WINDOW,
            Arc::new(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            }),
        )
    }
}

/// Not serializable or printable. The sole raw copy is transferred to the
/// host-local harness context, which also registers exact-value redaction.
pub struct IssuedCredential {
    pub authorization: String,
    pub session_id: String,
}

impl CredentialRegistry {
    pub fn new(window: Duration, now: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        Self {
            records: Mutex::default(),
            now,
            window_ms: window.as_millis() as u64,
        }
    }

    pub fn issue(&self, mut scope: InvocationScope) -> std::io::Result<IssuedCredential> {
        let mut random = [0_u8; 32];
        getrandom::fill(&mut random).map_err(std::io::Error::other)?;
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random);
        let timestamp = (self.now)();
        scope.issued_at = timestamp;
        // An MCP credential session id is not a kernel process-session key.
        scope.caller.session_id = uuid::Uuid::new_v4().to_string();
        let session_id = scope.caller.session_id.clone();
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records
            .retain(|_, record| timestamp.saturating_sub(record.last_alive_at) <= self.window_ms);
        records.insert(
            Sha256::digest(token.as_bytes()).into(),
            Record {
                scope,
                last_alive_at: timestamp,
            },
        );
        Ok(IssuedCredential {
            authorization: format!("Bearer {token}"),
            session_id,
        })
    }

    pub fn resolve(&self, authorization: &str) -> Option<InvocationScope> {
        let token = authorization.strip_prefix("Bearer ")?.trim();
        if token.is_empty() {
            return None;
        }
        let hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let timestamp = (self.now)();
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records
            .retain(|_, record| timestamp.saturating_sub(record.last_alive_at) <= self.window_ms);
        let record = records.get_mut(&hash)?;
        record.last_alive_at = timestamp;
        Some(record.scope.clone())
    }

    pub fn touch(&self, thread: &str) {
        let timestamp = (self.now)();
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records.retain(|_, r| timestamp.saturating_sub(r.last_alive_at) <= self.window_ms);
        for record in records
            .values_mut()
            .filter(|r| r.scope.caller.thread_id.as_ref() == thread)
        {
            record.last_alive_at = timestamp;
        }
    }

    pub fn revoke_session(&self, id: &str) {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, r| r.scope.caller.session_id != id);
    }
    pub fn revoke_thread(&self, id: &str) {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, r| r.scope.caller.thread_id.as_ref() != id);
    }
    pub fn revoke_all(&self) {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}
