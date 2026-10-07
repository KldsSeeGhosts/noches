//! Pi RPC transport: `pi --mode rpc` speaks LF-delimited JSON records.
//!
//! Commands go to stdin as `{"type": "...", "id": "..."}`; every stdout record
//! is either a `response` (correlated by `id`) or a session event, surfaced on
//! the event channel in arrival order. Framing follows Pi's spec: LF only (a
//! trailing CR is stripped), never U+2028/U+2029, so a JSON string containing
//! them cannot split a record. Records that are not JSON objects are dropped
//! rather than failing the transport: a chatty extension must not take the
//! session down.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

use crate::HarnessError;
use crate::process::{ChildStdin, ChildStdout};

/// Pi's own transcript records stay far below this; a longer line is a broken
/// writer, and buffering it would let a runaway process exhaust memory.
const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;

/// Pi's error text can echo remote output; bound what reaches a chat error.
const ERROR_DETAIL_CHARS: usize = 400;

type Reply = Result<Value, HarnessError>;

/// Splits a byte stream into LF-delimited records without ever buffering more
/// than [`MAX_RECORD_BYTES`] of one line.
#[derive(Default)]
pub(crate) struct Framer {
    buffer: Vec<u8>,
    dropping: bool,
}

impl Framer {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        let mut lines = Vec::new();
        let mut rest = chunk;
        while !rest.is_empty() {
            let newline = rest.iter().position(|b| *b == b'\n');
            let end = newline.unwrap_or(rest.len());
            if !self.dropping {
                if self.buffer.len() + end > MAX_RECORD_BYTES {
                    self.buffer.clear();
                    self.dropping = true;
                } else {
                    self.buffer.extend_from_slice(&rest[..end]);
                }
            }
            let Some(newline) = newline else { break };
            if !self.dropping {
                if self.buffer.last() == Some(&b'\r') {
                    self.buffer.pop();
                }
                if !self.buffer.is_empty() {
                    lines.push(std::mem::take(&mut self.buffer));
                }
            }
            self.buffer.clear();
            self.dropping = false;
            rest = &rest[newline + 1..];
        }
        lines
    }

    /// The unterminated tail at EOF (Pi flushes whole lines, but a crash can
    /// leave a final record without its LF).
    pub(crate) fn finish(&mut self) -> Option<Vec<u8>> {
        let mut line = std::mem::take(&mut self.buffer);
        let dropped = std::mem::take(&mut self.dropping);
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        (!dropped && !line.is_empty()).then_some(line)
    }
}

fn parse_record(line: &[u8]) -> Option<Value> {
    serde_json::from_slice::<Value>(line)
        .ok()
        .filter(Value::is_object)
}

/// Who is waiting for a correlated response.
enum Waiter {
    /// A caller awaiting the reply directly.
    Reply(oneshot::Sender<Reply>),
    /// The response is re-injected into the event stream, tagged, at exactly
    /// the position Pi wrote it. Callers that must order a response against
    /// the events around it (a prompt ack vs. the user message it starts) use
    /// this, since two channels would race.
    Ordered(u64),
}

struct Shared {
    writer: mpsc::UnboundedSender<Vec<u8>>,
    pending: Mutex<HashMap<String, Waiter>>,
    next_id: AtomicU64,
    /// Set once the reader or writer ends; later submissions fail fast.
    closed: Mutex<Option<String>>,
}

impl Shared {
    fn close(&self, reason: &str) {
        let pending = {
            let mut closed = self
                .closed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if closed.is_none() {
                *closed = Some(reason.to_owned());
            }
            std::mem::take(
                &mut *self
                    .pending
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            )
        };
        for (_, waiter) in pending {
            if let Waiter::Reply(reply) = waiter {
                let _ = reply.send(Err(HarnessError::Transport(reason.to_owned())));
            }
        }
    }

    fn closed_reason(&self) -> Option<String> {
        self.closed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// The command side of one Pi process. Cheap to clone.
#[derive(Clone)]
pub(crate) struct PiRpc(Arc<Shared>);

/// A command that was written and is awaiting its correlated response.
pub(crate) struct Submitted {
    id: String,
    reply: oneshot::Receiver<Reply>,
}

impl Submitted {
    pub(crate) async fn wait(self) -> Reply {
        self.reply.await.unwrap_or_else(|_| {
            Err(HarnessError::Transport(
                "Pi dropped the command without answering".into(),
            ))
        })
    }
}

impl PiRpc {
    /// Start the reader and writer tasks. The receiver yields every non-response
    /// record in arrival order and ends when stdout closes.
    pub(crate) fn new(
        mut stdin: ChildStdin,
        mut stdout: ChildStdout,
    ) -> (Self, mpsc::UnboundedReceiver<Value>) {
        let (writer, mut lines) = mpsc::unbounded_channel::<Vec<u8>>();
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            writer,
            pending: Mutex::default(),
            next_id: AtomicU64::new(0),
            closed: Mutex::default(),
        });

        let reader = shared.clone();
        tokio::spawn(async move {
            let mut framer = Framer::default();
            let mut chunk = vec![0u8; 64 * 1024];
            loop {
                let read = match stdout.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(read) => read,
                };
                for line in framer.push(&chunk[..read]) {
                    if let Some(record) = parse_record(&line) {
                        reader.route(record, &events_tx);
                    }
                }
            }
            if let Some(record) = framer.finish().as_deref().and_then(parse_record) {
                reader.route(record, &events_tx);
            }
            reader.close("Pi closed its output");
            // Dropping `events_tx` ends the consumer's stream.
        });

        let writer = shared.clone();
        tokio::spawn(async move {
            while let Some(line) = lines.recv().await {
                if stdin.write_all(&line).await.is_err() || stdin.flush().await.is_err() {
                    writer.close("Pi closed its input");
                    return;
                }
            }
            // Reached only once every sender is gone. The reader task keeps
            // one until stdout closes, so while Pi lives this never runs: Pi
            // is stopped by `Child::shutdown` (TERM, then KILL), not by EOF.
            let _ = stdin.shutdown().await;
        });

        (Self(shared), events_rx)
    }

    /// Fire-and-forget write (`extension_ui_response`, which has no response).
    pub(crate) fn send(&self, record: Value) -> Result<(), HarnessError> {
        if let Some(reason) = self.0.closed_reason() {
            return Err(HarnessError::Transport(reason));
        }
        let mut line = serde_json::to_vec(&record)
            .map_err(|e| HarnessError::Protocol(format!("unserializable Pi command: {e}")))?;
        line.push(b'\n');
        self.0
            .writer
            .send(line)
            .map_err(|_| HarnessError::Transport("Pi closed its input".into()))
    }

    /// Write a command and return a handle for its correlated response.
    /// Nothing is awaited here, so a command that Pi answers late (an
    /// extension slash command blocked on a dialog) never stalls the caller.
    pub(crate) fn submit(&self, mut record: Value) -> Result<Submitted, HarnessError> {
        let id = format!("noches-{}", self.0.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, reply) = oneshot::channel();
        if let Some(object) = record.as_object_mut() {
            object.insert("id".into(), Value::String(id.clone()));
        }
        self.0
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.clone(), Waiter::Reply(tx));
        if let Err(error) = self.send(record) {
            self.0
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&id);
            return Err(error);
        }
        // A close that raced the insert already drained `pending`; if our
        // entry survived the drain it is failed here.
        if let Some(reason) = self.0.closed_reason() {
            self.0.close(&reason);
        }
        Ok(Submitted { id, reply })
    }

    /// Write a command whose response is delivered on the event stream as the
    /// raw `response` record plus `"tag": tag`, in Pi's own record order.
    pub(crate) fn submit_ordered(&self, mut record: Value, tag: u64) -> Result<(), HarnessError> {
        let id = format!("noches-{}", self.0.next_id.fetch_add(1, Ordering::Relaxed));
        if let Some(object) = record.as_object_mut() {
            object.insert("id".into(), Value::String(id.clone()));
        }
        self.0
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.clone(), Waiter::Ordered(tag));
        if let Err(error) = self.send(record) {
            self.0
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&id);
            return Err(error);
        }
        Ok(())
    }

    /// Submit and wait. A timeout is a `Transport` error: Pi may still apply
    /// the command, so the caller must not assume it was rejected.
    pub(crate) async fn request(&self, record: Value, timeout: Duration) -> Reply {
        let command = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("request")
            .to_owned();
        let submitted = self.submit(record)?;
        let id = submitted.id.clone();
        match tokio::time::timeout(timeout, submitted.wait()).await {
            Ok(reply) => reply,
            Err(_) => {
                self.0
                    .pending
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&id);
                Err(HarnessError::Transport(format!(
                    "Pi did not answer `{command}` within {}s",
                    timeout.as_secs()
                )))
            }
        }
    }
}

impl Shared {
    fn route(&self, record: Value, events: &mpsc::UnboundedSender<Value>) {
        if record.get("type").and_then(Value::as_str) == Some("response")
            && let Some(id) = record.get("id").and_then(Value::as_str)
        {
            let waiter = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(id);
            match waiter {
                Some(Waiter::Reply(reply)) => {
                    let _ = reply.send(response_result(&record));
                    return;
                }
                Some(Waiter::Ordered(tag)) => {
                    let mut record = record;
                    record["tag"] = Value::from(tag);
                    let _ = events.send(record);
                    return;
                }
                None => {}
            }
        }
        let _ = events.send(record);
    }
}

/// `data` of a successful response (`Null` when the command carries none), or
/// Pi's own error text for `success: false`.
fn response_result(record: &Value) -> Reply {
    if record.get("success").and_then(Value::as_bool) == Some(true) {
        return Ok(record.get("data").cloned().unwrap_or(Value::Null));
    }
    let command = record
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("request");
    let detail = match record.get("error") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "unknown error".into(),
    };
    Err(HarnessError::Protocol(format!(
        "Pi `{command}` failed: {}",
        detail.chars().take(ERROR_DETAIL_CHARS).collect::<String>()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framer_splits_on_lf_only_and_strips_cr() {
        let mut framer = Framer::default();
        let first = framer.push(b"{\"a\":1}\r\n{\"b\":\"x\xe2\x80\xa8y\"}\n{\"c\"");
        assert_eq!(first.len(), 2);
        assert_eq!(first[0], b"{\"a\":1}");
        // U+2028 inside a JSON string is not a record boundary.
        assert_eq!(first[1], "{\"b\":\"x\u{2028}y\"}".as_bytes());
        let rest = framer.push(b":3}\n\n");
        assert_eq!(rest, vec![b"{\"c\":3}".to_vec()]);
        assert!(framer.finish().is_none());
    }

    #[test]
    fn framer_drops_oversized_records_and_recovers() {
        let mut framer = Framer::default();
        let big = vec![b'x'; MAX_RECORD_BYTES + 1];
        assert!(framer.push(&big).is_empty());
        let after = framer.push(b"tail\n{\"ok\":true}\n");
        // The remainder of the oversized line is discarded, the next is kept.
        assert_eq!(after, vec![b"{\"ok\":true}".to_vec()]);
    }

    #[test]
    fn framer_keeps_an_unterminated_final_record() {
        let mut framer = Framer::default();
        assert!(framer.push(b"{\"last\":true}").is_empty());
        assert_eq!(framer.finish().unwrap(), b"{\"last\":true}");
    }

    #[test]
    fn non_object_and_malformed_lines_are_not_records() {
        assert!(parse_record(b"not json").is_none());
        assert!(parse_record(b"[1,2]").is_none());
        assert!(parse_record(b"42").is_none());
        assert!(parse_record(b"{\"type\":\"agent_start\"}").is_some());
    }

    #[test]
    fn failed_responses_carry_pi_text_and_successes_their_data() {
        let ok = serde_json::json!({"type":"response","success":true,"data":{"x":1}});
        assert_eq!(response_result(&ok).unwrap()["x"], 1);
        let none = serde_json::json!({"type":"response","success":true});
        assert!(response_result(&none).unwrap().is_null());
        let failed = serde_json::json!({"type":"response","command":"set_model","success":false,"error":"Model not found: cpa/x"});
        let error = response_result(&failed).unwrap_err();
        assert!(matches!(error, HarnessError::Protocol(_)));
        assert!(error.to_string().contains("Model not found: cpa/x"));
    }
}
