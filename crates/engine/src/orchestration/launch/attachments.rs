use super::{HostLaunchService, ToolError, id, invalid, unavailable};
use crate::mcp::auth::InvocationScope;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::PathBuf;
use zeron_proto::orchestration::ThreadId;

pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
const URL_TTL: i64 = 600_000;

pub fn pending(a: &Value) -> bool {
    static PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)^pending-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(?:-[a-z0-9]{1,10})?$").unwrap()
    });
    a["id"].as_str().is_some_and(|s| PATTERN.is_match(s))
}

fn extension(upload: &Value) -> String {
    if upload["type"] != "file" {
        return match upload["mimeType"].as_str().unwrap_or("") {
            "image/jpeg" => ".jpg",
            "image/webp" => ".webp",
            "image/gif" => ".gif",
            _ => ".png",
        }
        .into();
    }
    let ext = std::path::Path::new(upload["name"].as_str().unwrap_or(""))
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext.is_empty()
        || ext.len() > 10
        || !ext.bytes().all(|b| b.is_ascii_alphanumeric())
        || ext == "part"
    {
        ".bin".into()
    } else {
        format!(".{ext}")
    }
}

// RFC 2104 HMAC-SHA256; fixed 32-byte private key. Signature compare is constant-time.
pub(super) fn signature(payload: &str, key: &[u8]) -> String {
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for (n, byte) in key.iter().enumerate().take(64) {
        inner[n] ^= byte;
        outer[n] ^= byte;
    }
    let mut hash = Sha256::new();
    hash.update(inner);
    hash.update(payload.as_bytes());
    let digest = hash.finalize();
    let mut hash = Sha256::new();
    hash.update(outer);
    hash.update(digest);
    BASE64.encode(hash.finalize())
}

impl HostLaunchService {
    pub(crate) fn prepare_upload(
        &self,
        scope: &InvocationScope,
        input: &Value,
    ) -> Result<Value, ToolError> {
        self.caller(scope, true)?;
        let mut upload = input["upload"].clone();
        if upload.get("type").is_none() {
            upload["type"] = json!("image");
        }
        let size = upload["sizeBytes"].as_u64().ok_or_else(unavailable)?;
        let maximum = if upload["type"] == "file" {
            MAX_UPLOAD_BYTES
        } else {
            10 * 1024 * 1024
        };
        if size == 0 || size > maximum as u64 {
            return Err(invalid("Invalid attachment size."));
        }
        let now = crate::now_ms();
        // Pending TTL is 24h (signed upload URL TTL is separately 10min).
        for row in self
            .rows("orchestration_launch_uploads")
            .map_err(|_| unavailable())?
        {
            if row["createdAt"]
                .as_i64()
                .is_some_and(|t| now - t > 86_400_000)
            {
                let _ = std::fs::remove_file(self.upload_path(&row));
            }
        }
        let ext = extension(&upload);
        let aid = format!(
            "pending-{}{}",
            id(),
            if upload["type"] == "file" {
                format!("-{}", ext.trim_start_matches('.'))
            } else {
                String::new()
            }
        );
        let expires = now + URL_TTL;
        let claims = json!({"version":1,"kind":"attachment-upload","type":upload["type"],"attachmentId":aid,
            "name":upload["name"],"mimeType":upload["mimeType"],"sizeBytes":size,"expiresAt":expires});
        let payload = BASE64.encode(claims.to_string());
        let row =
            json!({"id":aid,"upload":upload,"extension":ext,"createdAt":now,"uploaded":false});
        self.save("orchestration_launch_uploads", &aid, &row)
            .map_err(|_| unavailable())?;
        Ok(
            json!({"attachmentId":aid,"relativeUrl":format!("/api/attachments/upload/{payload}.{}",signature(&payload,&self.signing_key)),"expiresAt":expires}),
        )
    }

    fn upload_path(&self, row: &Value) -> PathBuf {
        self.data_dir.join("attachments").join(format!(
            "{}{}",
            row["id"].as_str().unwrap_or("invalid"),
            row["extension"].as_str().unwrap_or(".bin")
        ))
    }

    pub(crate) fn store_upload(&self, token: &str, bytes: &[u8]) -> (u16, Value) {
        let Some((payload, sig)) = token.split_once('.') else {
            return (401, json!({"detail":"Invalid or expired upload token."}));
        };
        let expected = signature(payload, &self.signing_key);
        if sig.len() != expected.len()
            || sig
                .bytes()
                .zip(expected.bytes())
                .fold(0u8, |a, (x, y)| a | (x ^ y))
                != 0
        {
            return (401, json!({"detail":"Invalid or expired upload token."}));
        }
        let claims = BASE64
            .decode(payload)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        let Some(claims) = claims.filter(|c| {
            c["kind"] == "attachment-upload"
                && c["expiresAt"].as_i64().is_some_and(|t| t > crate::now_ms())
        }) else {
            return (401, json!({"detail":"Invalid or expired upload token."}));
        };
        let aid = claims["attachmentId"].as_str().unwrap_or("");
        let mut row = match self
            .rows("orchestration_launch_uploads")
            .ok()
            .and_then(|rows| rows.into_iter().find(|r| r["id"] == aid))
        {
            Some(row) => row,
            None => return (401, json!({"detail":"Invalid or expired upload token."})),
        };
        let size = claims["sizeBytes"].as_u64().unwrap_or(0);
        if bytes.len() as u64 != size {
            return (
                400,
                json!({"detail":format!("Body was {} bytes, expected {size}.",bytes.len())}),
            );
        }
        if bytes.len() > MAX_UPLOAD_BYTES {
            return (413, json!({"detail":"Upload exceeds byte limit."}));
        }
        let path = self.upload_path(&row);
        let result = (|| -> std::io::Result<()> {
            use std::io::Write;
            std::fs::create_dir_all(path.parent().unwrap())?;
            let mut part = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
            part.write_all(bytes)?;
            part.as_file().sync_all()?;
            part.persist(&path).map_err(|e| e.error)?;
            Ok(())
        })();
        if result.is_err() {
            return (500, json!({"detail":"Failed to persist upload."}));
        }
        row["uploaded"] = json!(true);
        if self
            .save("orchestration_launch_uploads", aid, &row)
            .is_err()
        {
            return (500, json!({"detail":"Failed to persist upload."}));
        }
        (200, json!({"ok":true}))
    }

    pub(crate) fn discard(
        &self,
        scope: &InvocationScope,
        input: &Value,
    ) -> Result<Value, ToolError> {
        self.caller(scope, true)?;
        let aid = input["attachmentId"].as_str().unwrap();
        if aid.starts_with("pending-") {
            if let Some(row) = self
                .rows("orchestration_launch_uploads")
                .map_err(|_| unavailable())?
                .iter()
                .find(|r| r["id"] == aid)
            {
                let _ = std::fs::remove_file(self.upload_path(row));
            }
        }
        Ok(json!({}))
    }

    pub(crate) fn claim(
        &self,
        thread: &str,
        requested: Vec<Value>,
    ) -> Result<(Vec<Value>, Vec<PathBuf>), ToolError> {
        if requested.len() > 8 {
            return Err(ToolError::new(
                super::Code::OrchestrationError,
                "You can attach up to 8 files per message or question response.",
            ));
        }
        let image_bytes: u64 = requested
            .iter()
            .filter(|a| {
                a["type"] == "image"
                    || matches!(
                        a["mimeType"].as_str(),
                        Some("image/png" | "image/jpeg" | "image/webp" | "image/gif")
                    )
            })
            .filter_map(|a| a["sizeBytes"].as_u64())
            .sum();
        if image_bytes > 80 * 1024 * 1024 {
            return Err(ToolError::new(
                super::Code::OrchestrationError,
                "Images can total up to 80 MiB per message or question response. Use smaller images or send fewer at once.",
            ));
        }
        if requested
            .iter()
            .map(|a| a["id"].as_str())
            .collect::<HashSet<_>>()
            .len()
            != requested.len()
        {
            return Err(ToolError::new(
                super::Code::OrchestrationError,
                "Duplicate attachment ids are not allowed.",
            ));
        }
        let segment: String = thread
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let segment = regex::Regex::new("-+").unwrap().replace_all(&segment, "-");
        let segment: String = segment.trim_matches(['-', '_']).chars().take(80).collect();
        let segment = segment.trim_end_matches(['-', '_']);
        let segment = if segment == "pending" {
            "_pending"
        } else {
            segment
        };
        if segment.is_empty() {
            return Err(invalid("Invalid thread id."));
        }
        let rows = self
            .rows("orchestration_launch_uploads")
            .map_err(|_| unavailable())?;
        let mut attachments = vec![];
        let mut paths = vec![];
        let result = (|| {
            for mut a in requested {
                if !pending(&a) {
                    attachments.push(a);
                    continue;
                }
                let name = a["name"].as_str().unwrap_or("").to_owned();
                let err = |detail: &str| {
                    ToolError::new(
                        super::Code::OrchestrationError,
                        format!("Attachment '{name}' cannot be sent: {detail}."),
                    )
                };
                let row = rows
                    .iter()
                    .find(|r| r["id"] == a["id"])
                    .ok_or_else(|| err("attachment not found (removed or expired)"))?;
                let source = self.upload_path(row);
                let metadata =
                    std::fs::metadata(&source).map_err(|_| err("attachment not found"))?;
                if metadata.len() != a["sizeBytes"].as_u64().unwrap_or(0) {
                    return Err(err("stored size does not match"));
                }
                if extension(&a) != row["extension"].as_str().unwrap() {
                    return Err(err("attachment type does not match the upload"));
                }
                let aid = format!(
                    "{segment}-{}{}",
                    id(),
                    if a["type"] == "file" {
                        format!("-{}", extension(&a).trim_start_matches('.'))
                    } else {
                        String::new()
                    }
                );
                let path = self
                    .data_dir
                    .join("attachments")
                    .join(format!("{aid}{}", extension(&a)));
                // Copy rather than hard-link: edits cannot corrupt the pending retry source.
                std::fs::copy(&source, &path).map_err(|_| err("failed to claim attachment"))?;
                paths.push(path.clone());
                a["id"] = json!(aid);
                a["mimeType"] = json!(a["mimeType"].as_str().unwrap_or("").to_lowercase());
                self.save(
                    "orchestration_launch_claims",
                    &aid,
                    &json!({"id":aid,"threadId":thread,"path":path,"attachment":a}),
                )
                .map_err(|_| unavailable())?;
                attachments.push(a);
            }
            Ok(())
        })();
        if let Err(error) = result {
            for path in paths {
                let _ = std::fs::remove_file(path);
            }
            return Err(error);
        }
        Ok((attachments, paths))
    }

    pub(crate) async fn send_attachments(
        &self,
        scope: &InvocationScope,
        input: Value,
    ) -> Result<Value, ToolError> {
        let caller = self.caller(scope, true)?;
        let target = input["threadId"]
            .as_str()
            .unwrap_or(&scope.caller.thread_id.0);
        let p = self
            .kernel
            .store
            .thread(&ThreadId(target.into()))
            .map_err(|_| unavailable())?
            .ok_or_else(|| {
                ToolError::new(super::Code::ThreadNotFound, "The thread was not found.")
            })?;
        if p.thread.project_id != caller.project_id {
            return Err(ToolError::new(
                super::Code::ThreadNotFound,
                "The thread was not found.",
            ));
        }
        if !caller.runtime_mode.permits(p.thread.runtime_mode) {
            return Err(ToolError::new(
                super::Code::RuntimeModeEscalationDenied,
                format!(
                    "Child runtime mode {} is broader than parent mode {}.",
                    serde_json::to_value(p.thread.runtime_mode)
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    serde_json::to_value(caller.runtime_mode)
                        .unwrap()
                        .as_str()
                        .unwrap()
                ),
            ));
        }
        if !caller.interaction_mode.permits(p.thread.interaction_mode) {
            return Err(ToolError::new(
                super::Code::InteractionModeEscalationDenied,
                format!(
                    "Child interaction mode {} is broader than parent mode {}.",
                    serde_json::to_value(p.thread.interaction_mode)
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    serde_json::to_value(caller.interaction_mode)
                        .unwrap()
                        .as_str()
                        .unwrap()
                ),
            ));
        }
        if p.thread.archived_at.is_some() {
            return Err(invalid(
                "Unarchive the target thread before sending attachments.",
            ));
        }
        let mut references = vec![];
        for a in input["attachments"].as_array().unwrap() {
            if pending(a) {
                references.push(a.clone());
            } else {
                let canonical=super::super::task::records(&p,"message").into_iter().flat_map(|m|m["attachments"].as_array().cloned().unwrap_or_default()).find(|stored|stored["id"]==a["id"])
                    .ok_or_else(||invalid("Attachments must be pending uploads or already belong to the target thread."))?;
                references.push(canonical);
            }
        }
        let (attachments, paths) = self.claim(target, references)?;
        match self
            .intake
            .send(
                target,
                &id(),
                input["message"].as_str().unwrap_or(""),
                attachments,
                false,
            )
            .await
        {
            Ok(result) => Ok(result),
            Err(error) => {
                if !error.uncertain {
                    for path in paths {
                        let _ = std::fs::remove_file(path);
                    }
                }
                Err(error.error)
            }
        }
    }
}
