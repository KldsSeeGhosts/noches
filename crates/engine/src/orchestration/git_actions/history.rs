//! Read-only bounded CLI transcripts. Imports never launch/resume a provider.
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError};

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use zeron_doc::{MessagePart, MessageRole, MessageStatus, SessionDoc, SessionMessageEntry};
use zeron_proto::git_actions::*;

use super::{GitActionsService, get, put};

const MAX_FILES: usize = 5000;
const MAX_DISCOVERY_OPS: usize = 20_000;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BATCH_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SCAN_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
const MAX_LINE_BYTES: usize = 1024 * 1024;
const MAX_MESSAGES: usize = 200;
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// Disposable OS threads, not the runtime's blocking pool: a wedged home mount
/// cannot prevent RPC cancellation or engine teardown. A permit remains held
/// until the actual read exits, so timed-out jobs cannot leak unlimited threads.
async fn bounded_read<T: Send + 'static>(
    cancel: &CancellationToken,
    job: impl FnOnce(CancellationToken) -> T + Send + 'static,
) -> anyhow::Result<T> {
    static LANES: std::sync::LazyLock<Arc<tokio::sync::Semaphore>> =
        std::sync::LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(4)));
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let child = cancel.child_token();
    let _guard = child.clone().drop_guard();
    let permit = tokio::select! {
        _ = cancel.cancelled() => anyhow::bail!("History read cancelled"),
        permit = tokio::time::timeout_at(deadline, LANES.clone().acquire_owned()) =>
            permit.context("History filesystem lanes busy")??,
    };
    let (sender, receiver) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("cli-history-read".into())
        .spawn(move || {
            let _permit = permit;
            let _ = sender.send(job(child));
        })?;
    tokio::select! {
        _ = cancel.cancelled() => anyhow::bail!("History read cancelled"),
        result = tokio::time::timeout_at(deadline,receiver) =>
            result.context("History filesystem operation timed out")?.context("History read worker failed"),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScanJob {
    pub state: HistoryScanState,
    pub project_root: String,
    pub files: Vec<(HistorySource, PathBuf)>,
    pub cursor: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Imported {
    chat_id: String,
    space_id: String,
    provenance: HistoryProvenance,
}

pub(super) fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

fn native_key(provenance: &HistoryProvenance) -> String {
    digest(format!(
        "{:?}\0{}",
        provenance.source, provenance.native_session_id
    ))
}

fn message_text(message: &Value) -> String {
    match &message["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|part| {
                ["text", "input_text", "output_text"].contains(&part["type"].as_str().unwrap_or(""))
            })
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Mirrors T3's turn-aware canonical event precedence. Markup in a response-only
/// transcript is preserved; it is NOT blanket-filtered as hidden instructions.
fn canonical_response_users(bytes: &[u8]) -> HashSet<usize> {
    let mut suppressed = HashSet::new();
    let mut canonical = HashSet::new();
    let mut responses: Vec<(usize, String, String)> = Vec::new();
    let finish = |canonical: &mut HashSet<String>,
                  responses: &mut Vec<(usize, String, String)>,
                  suppressed: &mut HashSet<usize>| {
        let turns: HashSet<_> = responses
            .iter()
            .filter(|(_, _, text)| canonical.contains(text))
            .map(|(_, turn, _)| turn.clone())
            .collect();
        for (ordinal, turn, _) in responses.drain(..) {
            if turns.contains(&turn) {
                suppressed.insert(ordinal);
            }
        }
        canonical.clear();
    };
    for (ordinal, line) in bytes.split(|b| *b == b'\n').take(MAX_RECORDS).enumerate() {
        if line.len() > MAX_LINE_BYTES {
            break;
        }
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let payload = &record["payload"];
        if record["type"] == "response_item"
            && payload["type"] == "message"
            && payload["role"] == "assistant"
        {
            finish(&mut canonical, &mut responses, &mut suppressed);
        } else if record["type"] == "event_msg" && payload["type"] == "user_message" {
            if let Some(text) = payload["message"].as_str() {
                canonical.insert(text.trim().to_owned());
            }
        } else if record["type"] == "response_item"
            && payload["type"] == "message"
            && payload["role"] == "user"
        {
            if let Some(turn) = payload["internal_chat_message_metadata_passthrough"]["turn_id"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
            {
                responses.push((ordinal, turn.into(), message_text(payload).trim().into()));
            }
        }
    }
    finish(&mut canonical, &mut responses, &mut suppressed);
    suppressed
}

fn discover(
    roots: &[(HistorySource, PathBuf)],
    cancel: &CancellationToken,
) -> (Vec<(HistorySource, PathBuf)>, bool) {
    let mut files = Vec::new();
    let mut remaining = MAX_DISCOVERY_OPS;
    let mut truncated = false;
    for (source, root) in roots {
        let mut pending = vec![(root.clone(), 0)];
        while let Some((directory, depth)) = pending.pop() {
            if cancel.is_cancelled() {
                return (files, true);
            }
            if remaining == 0 || files.len() >= MAX_FILES {
                truncated = true;
                break;
            }
            remaining -= 1;
            if !std::fs::symlink_metadata(&directory).is_ok_and(|m| m.is_dir()) {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&directory) {
                let mut children = Vec::new();
                for entry in entries {
                    if remaining == 0 {
                        truncated = true;
                        break;
                    }
                    remaining -= 1;
                    if let Ok(entry) = entry {
                        children.push(entry.path());
                    }
                }
                children.sort();
                for path in children {
                    if files.len() >= MAX_FILES {
                        truncated = true;
                        break;
                    }
                    let Ok(meta) = std::fs::symlink_metadata(&path) else {
                        continue;
                    };
                    if meta.is_dir() && depth < 6 {
                        pending.push((path, depth + 1));
                    } else if meta.is_file() && path.extension().is_some_and(|s| s == "jsonl") {
                        files.push((*source, path));
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    (files, truncated)
}

/// No fallback filename identity: a file without an explicit native id is not
/// resumable and cannot accidentally dedupe unrelated transcripts.
pub(super) fn parse(
    source: HistorySource,
    file: &Path,
    project: &Path,
    cancel: &CancellationToken,
) -> anyhow::Result<(Option<HistoryPreview>, u64)> {
    let meta = std::fs::symlink_metadata(file)?;
    ensure!(meta.is_file(), "Transcript is not a regular file");
    if meta.len() > MAX_FILE_BYTES {
        return Ok((None, 0));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(file)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_FILE_BYTES,
        "Transcript grew beyond byte limit"
    );
    let hash = digest(&bytes);
    let suppressed = if source == HistorySource::Codex {
        canonical_response_users(&bytes)
    } else {
        HashSet::new()
    };
    let mut reader = BufReader::new(bytes.as_slice());
    let mut native = None;
    let mut cwd = None;
    let mut messages = Vec::new();
    let mut truncated = false;
    let mut unsupported = false;
    let mut text_bytes = 0;
    let mut seen = HashSet::new();
    let mut origins = Vec::new();
    let mut title = None;
    for ordinal in 0..MAX_RECORDS {
        ensure!(!cancel.is_cancelled(), "Scan cancelled");
        let mut line = Vec::new();
        // Bounded line allocation even when the JSON record has no newline.
        let read = reader
            .by_ref()
            .take((MAX_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        if read > MAX_LINE_BYTES {
            truncated = true;
            break;
        }
        let Ok(record) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        if record["isSidechain"] == true
            || record["isMeta"] == true
            || record["isCompactSummary"] == true
        {
            continue;
        }
        if let Some(value) = record["aiTitle"].as_str() {
            title = Some(value.chars().take(80).collect::<String>());
        }
        let payload = &record["payload"];
        let native_value = match source {
            HistorySource::ClaudeCode => record["sessionId"].as_str(),
            HistorySource::Codex if record["type"] == "session_meta" => payload["id"].as_str(),
            _ => None,
        };
        if let Some(id) = native_value {
            ensure!(
                id.len() <= 256 && !id.is_empty(),
                "Invalid native session id"
            );
            if source == HistorySource::ClaudeCode {
                if let Some(prior) = &native {
                    ensure!(prior == id, "Mixed native sessions in transcript");
                }
            }
            if native.is_none() {
                native = Some(id.to_owned());
            }
        }
        if let Some(path) = record["cwd"].as_str().or_else(|| payload["cwd"].as_str()) {
            let path = std::fs::canonicalize(path).ok();
            if cwd.is_none() {
                cwd = path;
            } else if let Some(path) = path {
                ensure!(cwd.as_ref() == Some(&path), "Mixed projects in transcript");
            }
        }
        let canonical = source == HistorySource::Codex
            && record["type"] == "event_msg"
            && payload["type"] == "user_message";
        let event_message = serde_json::json!({"role":"user", "content":payload["message"]});
        let message = match source {
            HistorySource::ClaudeCode
                if record["type"] == "user" || record["type"] == "assistant" =>
            {
                &record["message"]
            }
            HistorySource::Codex
                if record["type"] == "response_item" && payload["type"] == "message" =>
            {
                payload
            }
            HistorySource::Codex if canonical => &event_message,
            _ => {
                if source == HistorySource::Codex && record["type"] == "response_item" {
                    unsupported = true;
                }
                continue;
            }
        };
        let Some(role) = message["role"]
            .as_str()
            .filter(|r| *r == "user" || *r == "assistant")
        else {
            continue;
        };
        let mut text = String::new();
        match &message["content"] {
            Value::String(value) => text = value.clone(),
            Value::Array(parts) => {
                for part in parts {
                    if ["text", "input_text", "output_text"]
                        .contains(&part["type"].as_str().unwrap_or(""))
                    {
                        if let Some(value) = part["text"].as_str() {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(value);
                        }
                    } else {
                        unsupported = true;
                    }
                }
            }
            _ => unsupported = true,
        }
        if text.trim().is_empty() {
            continue;
        }
        if suppressed.contains(&ordinal) {
            continue;
        }
        if source == HistorySource::Codex && role == "user" {
            let matching = messages
                .iter()
                .enumerate()
                .rev()
                .take_while(|(_, m): &(usize, &HistoryMessage)| m.role != "assistant")
                .find(|(i, m)| m.text.trim() == text.trim() && origins[*i] != canonical)
                .map(|(i, _)| i);
            if let Some(index) = matching {
                if !canonical {
                    continue;
                }
                text_bytes -= messages.remove(index).text.len();
                origins.remove(index);
            }
        }
        let id = record["uuid"].as_str().map(str::to_owned);
        if id.is_some_and(|id| !seen.insert(id)) {
            continue;
        }
        if messages.len() >= MAX_MESSAGES || text_bytes + text.len() > MAX_TEXT_BYTES {
            truncated = true;
            break;
        }
        text_bytes += text.len();
        let created_at = record["timestamp"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map_or(0, |t| t.timestamp_millis());
        messages.push(HistoryMessage {
            role: role.into(),
            text,
            created_at,
        });
        origins.push(canonical);
    }
    if !reader.fill_buf()?.is_empty() {
        truncated = true;
    }
    let Some(native) = native else {
        return Ok((None, bytes.len() as u64));
    };
    if cwd.as_deref() != Some(project) || !messages.iter().any(|m| m.role == "user") {
        return Ok((None, bytes.len() as u64));
    }
    let provenance = HistoryProvenance {
        source,
        native_session_id: native,
        file_path: file.to_string_lossy().into_owned(),
        sha256: hash,
        project_root: project.to_string_lossy().into_owned(),
        truncated,
        unsupported_content: unsupported,
    };
    let candidate_id = digest(format!(
        "{}\0{}",
        native_key(&provenance),
        provenance.sha256
    ));
    let title = title.unwrap_or_else(|| {
        messages
            .iter()
            .find(|m| m.role == "user")
            .unwrap_or(&messages[0])
            .text
            .lines()
            .next()
            .unwrap_or("Imported CLI history")
            .chars()
            .take(80)
            .collect()
    });
    Ok((
        Some(HistoryPreview {
            candidate_id,
            provenance,
            title,
            messages,
            already_imported_chat_id: None,
        }),
        bytes.len() as u64,
    ))
}

impl GitActionsService {
    pub async fn scan_history(
        &self,
        request: HistoryScanRequest,
    ) -> anyhow::Result<HistoryScanState> {
        let space = self.local_space(&request.space_id)?;
        let root = std::fs::canonicalize(&space.path)?;
        let mut job = if let Some(id) = request.scan_id {
            let job: ScanJob = self.state("scan", &id)?;
            ensure!(
                job.state.space_id == request.space_id && Path::new(&job.project_root) == root,
                "Scan project changed"
            );
            job
        } else {
            ScanJob {
                state: HistoryScanState {
                    scan_id: crate::new_id(),
                    space_id: request.space_id,
                    ..Default::default()
                },
                project_root: root.to_string_lossy().into_owned(),
                ..Default::default()
            }
        };
        if job.state.status == "completed" {
            return Ok(job.state);
        }
        let cancel = CancellationToken::new();
        let id = job.state.scan_id.clone();
        {
            let mut tokens = self
                .0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            ensure!(!tokens.contains_key(&id), "Scan already running");
            ensure!(
                tokens.len() < 4,
                "At most four history operations may run concurrently"
            );
            tokens.insert(id.clone(), cancel.clone());
        }
        job.state.status = "running".into();
        job.state.error = None;
        if let Err(error) = self.publish("scan", &id, &job) {
            self.0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&id);
            return Err(error);
        }
        let initial = job.state.clone();
        let this = self.clone();
        self.spawn(async move {
            let result = this.scan_batch(&mut job, &cancel).await;
            if cancel.is_cancelled() {
                job.state.status = "cancelled".into();
            } else if let Err(error) = result {
                job.state.status = "failed".into();
                job.state.error = Some(error.to_string());
            }
            let _ = this.publish("scan", &id, &job);
            this.0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&id);
        });
        Ok(initial)
    }

    async fn scan_batch(
        &self,
        job: &mut ScanJob,
        cancel: &CancellationToken,
    ) -> anyhow::Result<()> {
        if job.files.is_empty() && job.cursor == 0 {
            let roots = self.0.history_roots.clone();
            let (files, truncated) =
                bounded_read(cancel, move |token| discover(&roots, &token)).await?;
            job.files = files;
            job.state.truncated |= truncated;
            job.state.total_files = job.files.len();
        }
        let mut batch_bytes = 0;
        let mut batch_files = 0;
        while job.cursor < job.files.len() && !cancel.is_cancelled() {
            if job.state.bytes_read >= MAX_SCAN_BYTES {
                job.state.truncated = true;
                job.state.status = "completed".into();
                return Ok(());
            }
            if batch_bytes >= MAX_BATCH_BYTES || batch_files >= 256 {
                job.state.status = "paused".into();
                return Ok(());
            }
            let (source, path) = job.files[job.cursor].clone();
            let root = PathBuf::from(&job.project_root);
            let parsed =
                bounded_read(cancel, move |token| parse(source, &path, &root, &token)).await?;
            if cancel.is_cancelled() {
                break;
            }
            match parsed {
                Ok((Some(mut preview), bytes)) => {
                    if let Some(imported) = get::<Imported>(
                        &self.0.store,
                        "native_import",
                        &native_key(&preview.provenance),
                    )? {
                        ensure!(
                            imported.space_id == job.state.space_id,
                            "Native session already imported into another project"
                        );
                        preview.already_imported_chat_id = Some(imported.chat_id);
                    }
                    put(&self.0.store, "candidate", &preview.candidate_id, &preview)?;
                    if !job.state.candidate_ids.contains(&preview.candidate_id) {
                        job.state.candidate_ids.push(preview.candidate_id);
                    }
                    job.state.truncated |= preview.provenance.truncated;
                    batch_bytes += bytes;
                    job.state.bytes_read += bytes;
                }
                Ok((None, bytes)) => {
                    batch_bytes += bytes;
                    job.state.bytes_read += bytes;
                    if bytes == 0 {
                        job.state.truncated = true;
                    }
                }
                Err(_) => job.state.truncated = true,
            }
            job.cursor += 1;
            batch_files += 1;
            job.state.scanned_files = job.cursor;
            self.publish("scan", &job.state.scan_id, job)?;
        }
        job.state.status = if cancel.is_cancelled() {
            "cancelled"
        } else {
            "completed"
        }
        .into();
        Ok(())
    }

    pub fn cancel_history(&self, id: &str) -> anyhow::Result<()> {
        let tokens = self
            .0
            .cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        tokens.get(id).context("Operation is not running")?.cancel();
        Ok(())
    }

    pub fn history_preview(&self, id: &str) -> anyhow::Result<HistoryPreview> {
        self.state("candidate", id)
    }

    pub async fn import_history(
        &self,
        request: HistoryImportRequest,
    ) -> anyhow::Result<HistoryImportState> {
        self.local_space(&request.space_id)?;
        ensure!(
            !request.candidate_ids.is_empty() && request.candidate_ids.len() <= 100,
            "Select 1..100 histories"
        );
        // Validate the entire selection before writes, not just the first item.
        for id in &request.candidate_ids {
            let preview = self.history_preview(id)?;
            self.validate_import_project(&request.space_id, &preview)?;
        }
        let permit = self.0.sessions.admit_work()?;
        let import_id = crate::new_id();
        let state = HistoryImportState {
            import_id: import_id.clone(),
            status: "running".into(),
            ..Default::default()
        };
        let token = CancellationToken::new();
        {
            let mut tokens = self
                .0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            ensure!(
                tokens.len() < 4,
                "At most four history operations may run concurrently"
            );
            tokens.insert(import_id.clone(), token.clone());
        }
        if let Err(error) = self.publish("import", &import_id, &state) {
            self.0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&import_id);
            return Err(error);
        }
        let this = self.clone();
        let initial = state.clone();
        self.spawn(async move {
            let _permit = permit;
            let mut state = state;
            for id in &request.candidate_ids {
                if token.is_cancelled() {
                    state.status = "cancelled".into();
                    break;
                }
                let result = this
                    .import_one_cancellable(&request.space_id, id, &token)
                    .await;
                match result {
                    Ok(chat_id) => {
                        state.completed_candidate_ids.push(id.clone());
                        if !state.chat_ids.contains(&chat_id) {
                            state.chat_ids.push(chat_id);
                        }
                    }
                    Err(error) => {
                        state.status = if token.is_cancelled() {
                            "cancelled"
                        } else {
                            "failed"
                        }
                        .into();
                        state.error = Some(error.to_string());
                        break;
                    }
                }
                let _ = this.publish("import", &import_id, &state);
            }
            if state.status == "running" {
                state.status = "completed".into();
            }
            let _ = this.publish("import", &import_id, &state);
            this.0
                .cancellations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&import_id);
        });
        Ok(initial)
    }

    fn validate_import_project(
        &self,
        space_id: &str,
        preview: &HistoryPreview,
    ) -> anyhow::Result<()> {
        let space = self.local_space(space_id)?;
        ensure!(
            std::fs::canonicalize(space.path)? == Path::new(&preview.provenance.project_root),
            "Import project changed"
        );
        Ok(())
    }

    #[cfg(test)]
    pub(super) async fn import_one(&self, space_id: &str, id: &str) -> anyhow::Result<String> {
        self.import_one_cancellable(space_id, id, &CancellationToken::new())
            .await
    }

    async fn import_one_cancellable(
        &self,
        space_id: &str,
        id: &str,
        cancel: &CancellationToken,
    ) -> anyhow::Result<String> {
        let preview = self.history_preview(id)?;
        self.validate_import_project(space_id, &preview)?;
        let key = native_key(&preview.provenance);
        let _lock = self
            .0
            .locks
            .acquire([zeron_proto::orchestration::ThreadId(format!(
                "history:{key}"
            ))])
            .await;
        if let Some(imported) = get::<Imported>(&self.0.store, "native_import", &key)? {
            ensure!(
                imported.space_id == space_id,
                "Native session belongs to another project"
            );
            return Ok(imported.chat_id);
        }
        let source = preview.provenance.source;
        let file = PathBuf::from(&preview.provenance.file_path);
        let root = PathBuf::from(&preview.provenance.project_root);
        let (fresh, _) =
            bounded_read(cancel, move |token| parse(source, &file, &root, &token)).await??;
        let fresh = fresh.context("History no longer belongs to this project")?;
        ensure!(
            fresh.provenance.sha256 == preview.provenance.sha256,
            "Transcript changed; scan and preview again"
        );
        let chat_id = format!("cli-history-{key}");
        let reservation = Imported {
            chat_id: chat_id.clone(),
            space_id: space_id.into(),
            provenance: preview.provenance.clone(),
        };
        if let Some(prior) = get::<Imported>(&self.0.store, "import_reservation", &key)? {
            ensure!(
                prior.space_id == space_id && prior.provenance.sha256 == preview.provenance.sha256,
                "Unfinished import reserved another transcript snapshot; resume the original selection"
            );
        } else {
            ensure!(
                self.0.workspace.chat(&chat_id)?.is_none(),
                "Native import chat exists without provenance"
            );
            put(&self.0.store, "import_reservation", &key, &reservation)?;
        }
        // Canonical chat2 doc first, registry row last; a crash retry reuses the
        // same native id and never overwrites an existing conversation.
        if !self.0.store.docs.has_snapshot(&chat_id)? {
            let doc = SessionDoc::init(&chat_id)?;
            for (ordinal, message) in preview.messages.iter().enumerate() {
                let message_id = format!("{chat_id}:{ordinal}");
                doc.push_message(&SessionMessageEntry {
                    id: message_id.clone(),
                    role: if message.role == "user" {
                        MessageRole::User
                    } else {
                        MessageRole::Assistant
                    },
                    parts: vec![MessagePart::Text {
                        id: format!("{message_id}:text"),
                        text: message.text.clone(),
                    }],
                    created_at: message.created_at,
                    device_id: self.0.doc_host.device_id().into(),
                    status: Some(MessageStatus::Complete),
                    continuation_of: None,
                })?;
            }
            if preview.provenance.truncated || preview.provenance.unsupported_content {
                doc.push_message(&SessionMessageEntry {
                    id: format!("{chat_id}:import-note"), role: MessageRole::System,
                    parts: vec![MessagePart::Text {
                        id: format!("{chat_id}:import-note:text"),
                        text: "Imported CLI history is partial; tool calls/results, images and non-text content are not imported. Continuation requires a separate explicit action.".into(),
                    }],
                    created_at: crate::now_ms(), device_id: self.0.doc_host.device_id().into(),
                    status: Some(MessageStatus::Complete), continuation_of: None,
                })?;
            }
            self.0
                .store
                .docs
                .save_snapshot_with_cursor(&chat_id, &doc.export_snapshot()?, 0, 2)?;
        }
        if self.0.workspace.chat(&chat_id)?.is_none() {
            let row = zeron_proto::Chat {
                id: chat_id.clone(),
                device_id: self.0.doc_host.device_id().into(),
                title: Some(preview.title.clone()),
                archived: false,
                cwd: Some(preview.provenance.project_root.clone()),
                branch: None,
                checkout_id: None,
                source_context: None,
                config: None,
                last_message_preview: preview
                    .messages
                    .last()
                    .map(|m| m.text.chars().take(160).collect()),
                last_message_at: preview
                    .messages
                    .last()
                    .and_then(|m| chrono::DateTime::from_timestamp_millis(m.created_at)),
                created_at: preview
                    .messages
                    .first()
                    .and_then(|m| chrono::DateTime::from_timestamp_millis(m.created_at))
                    .unwrap_or_else(chrono::Utc::now),
                harness_session_id: None,
                harness_session_cwd: None,
                room_gen: Some(2),
                space_id: Some(space_id.into()),
                last_seen_at: None,
            };
            self.0.workspace.import_chat_row(&row)?;
        }
        put(&self.0.store, "native_import", &key, &reservation)?;
        Ok(chat_id)
    }

    /// Explicit identity binding only; does not send a prompt or start a run.
    pub async fn continue_history(
        &self,
        space_id: &str,
        candidate_id: &str,
        chat_id: &str,
    ) -> anyhow::Result<()> {
        let _permit = self
            .0
            .sessions
            .admission()
            .begin_update(|| self.checkout_idle())
            .context("Stop live harnesses and terminals before binding continuation")?;
        let preview = self.history_preview(candidate_id)?;
        self.validate_import_project(space_id, &preview)?;
        let imported: Imported = self.state("native_import", &native_key(&preview.provenance))?;
        let _lock = self
            .0
            .locks
            .acquire([
                zeron_proto::orchestration::ThreadId(format!(
                    "history:{}",
                    native_key(&preview.provenance)
                )),
                zeron_proto::orchestration::ThreadId(chat_id.into()),
            ])
            .await;
        ensure!(
            imported.chat_id == chat_id && imported.space_id == space_id,
            "Imported identity mismatch"
        );
        ensure!(
            uuid::Uuid::parse_str(&preview.provenance.native_session_id).is_ok(),
            "Native session is not resumable"
        );
        ensure!(
            !self.0.sessions.turn_in_flight(chat_id),
            "Stop active run before continuing history"
        );
        self.thread_checkout(chat_id, &preview.provenance.project_root)?;
        let chat = self
            .0
            .workspace
            .chat(chat_id)?
            .context("Imported chat missing")?;
        ensure!(
            chat.harness_session_id.is_none(),
            "Continuation already bound"
        );
        let handle = self.0.doc_host.open_local(chat_id)?;
        let entries = handle.doc().read_entries()?;
        ensure!(
            entries.len()
                == preview.messages.len()
                    + usize::from(
                        preview.provenance.truncated || preview.provenance.unsupported_content
                    )
                && entries
                    .iter()
                    .enumerate()
                    .all(|(i, entry)| entry.id == format!("{chat_id}:{i}")
                        || entry.id == format!("{chat_id}:import-note")),
            "Imported chat already contains non-imported activity"
        );
        let source = preview.provenance.source;
        let file = PathBuf::from(&preview.provenance.file_path);
        let root = PathBuf::from(&preview.provenance.project_root);
        let (fresh, _) = bounded_read(&CancellationToken::new(), move |token| {
            parse(source, &file, &root, &token)
        })
        .await??;
        ensure!(
            fresh.is_some_and(|p| p.provenance.sha256 == preview.provenance.sha256),
            "Transcript changed before continuation"
        );
        let harness = match source {
            HistorySource::ClaudeCode => zeron_proto::HarnessId::ClaudeCode,
            HistorySource::Codex => zeron_proto::HarnessId::Codex,
        };
        self.0.registry.resolve(harness)?;
        let config = zeron_proto::ChatConfig {
            harness,
            model: None,
            reasoning: None,
            model_options: Default::default(),
            sandbox: zeron_proto::SandboxLevel::WorkspaceWrite,
            runtime_mode: zeron_proto::RuntimeMode::ApprovalRequired,
            interaction_mode: Default::default(),
        };
        self.0.workspace.set_chat_config(chat_id, &config)?;
        self.0.workspace.set_chat_harness_session(
            chat_id,
            &preview.provenance.native_session_id,
            &preview.provenance.project_root,
        );
        let bound = self
            .0
            .workspace
            .chat(chat_id)?
            .context("Imported chat disappeared")?;
        ensure!(
            bound.harness_session_id.as_deref()
                == Some(preview.provenance.native_session_id.as_str())
                && bound.harness_session_cwd.as_deref()
                    == Some(preview.provenance.project_root.as_str()),
            "Native continuation identity binding failed"
        );
        Ok(())
    }
}
