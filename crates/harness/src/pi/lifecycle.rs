//! Native fork for Pi sessions.
//!
//! A fork is two steps in a short-lived `pi --mode rpc` that never runs user
//! extensions or tools:
//!
//! 1. `--fork <source session file>` copies the session into the *destination*
//!    working directory. (RPC `fork`/`clone`/`switch_session` alone would keep
//!    the source's cwd; only the CLI flag re-homes the session.)
//! 2. Unless the fork is at the head, RPC `fork {entryId}` re-roots the new
//!    session *before* the first user message of the next turn, so the child
//!    holds exactly the turns up to the selected one. Pi records the step-1
//!    copy as the child's `parentSession`, so that copy is left in place
//!    (like T3 does) rather than leaving a dangling lineage pointer.
//!
//! Pi's fork cuts BEFORE a user entry, so turn refs are user-entry ids and the
//! boundary is the *next* turn's ref (`source_next_turn_id`), like OpenCode.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{PiHarness, Spawned, launch};
use crate::HarnessError;
use crate::session_lifecycle::{NativeForkRequest, SessionLifecycle};

const FORK_TIMEOUT: Duration = Duration::from_secs(60);

/// A source session whose user entry `id` exists: the only kind of cut point
/// Pi's `fork` accepts. Scans line by line; sessions run to hundreds of MB, so
/// only lines that mention the id are parsed.
pub(super) fn is_user_entry(session: &Path, id: &str) -> bool {
    use std::io::BufRead as _;
    if id.is_empty() || id.chars().any(|c| matches!(c, '"' | '\\')) {
        return false;
    }
    let Ok(file) = std::fs::File::open(session) else {
        return false;
    };
    // Pi writes compact JSON, but only the quoted id is relied on here.
    let wanted = format!("\"{id}\"");
    let mut reader = std::io::BufReader::with_capacity(1 << 16, file);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return false,
            Ok(_) => {}
        }
        if !line.windows(wanted.len()).any(|w| w == wanted.as_bytes()) {
            continue;
        }
        let Ok(entry) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        if entry.get("id").and_then(Value::as_str) == Some(id) {
            return entry.get("type").and_then(Value::as_str) == Some("message")
                && entry.pointer("/message/role").and_then(Value::as_str) == Some("user");
        }
    }
}

/// What the end of a session file says about its current leaf.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Leaf {
    /// No entry has been written yet (a missing file or a header only).
    Empty,
    /// The id of the last complete entry: Pi's `leafId` after loading the file.
    Entry(String),
    /// The tail could not be read or holds an entry larger than the read cap.
    Unknown,
}

const TAIL_START: u64 = 1 << 16;
const TAIL_CAP: u64 = 1 << 24;

/// The last complete entry of a session file, found by reading backwards from
/// its end. Sessions run to hundreds of MB, and Pi's `get_entries` without a
/// cursor would serialise all of them (and exceed the framer's cap), so the
/// baseline cursor for `get_entries since` comes from the file instead. Pi
/// loads the last non-header entry as its leaf, which this mirrors.
pub(super) fn session_leaf(session: &Path) -> Leaf {
    use std::io::{Read as _, Seek as _, SeekFrom};
    let mut file = match std::fs::File::open(session) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Leaf::Empty,
        Err(_) => return Leaf::Unknown,
    };
    let Ok(len) = file.metadata().map(|m| m.len()) else {
        return Leaf::Unknown;
    };
    let mut window = TAIL_START;
    loop {
        let from = len.saturating_sub(window);
        let mut buffer = Vec::with_capacity((len - from) as usize);
        if file.seek(SeekFrom::Start(from)).is_err()
            || (&mut file).take(len - from).read_to_end(&mut buffer).is_err()
        {
            return Leaf::Unknown;
        }
        let mut lines = buffer.split(|b| *b == b'\n').collect::<Vec<_>>();
        // A window that starts mid-file begins inside some line.
        if from > 0 {
            lines.remove(0);
        }
        for line in lines.into_iter().rev() {
            // A torn trailing write is not a complete entry.
            let Ok(entry) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            if entry.get("type").and_then(Value::as_str) == Some("session") {
                return Leaf::Empty;
            }
            if let Some(id) = entry.get("id").and_then(Value::as_str) {
                return Leaf::Entry(id.to_owned());
            }
        }
        if from == 0 {
            return Leaf::Empty;
        }
        if window >= TAIL_CAP {
            return Leaf::Unknown;
        }
        window = (window * 4).min(TAIL_CAP);
    }
}

impl PiHarness {
    /// Why this boundary cannot fork natively, or the validated parts.
    fn fork_plan<'a>(
        request: &'a NativeForkRequest,
        roots: &[PathBuf],
    ) -> Result<(PathBuf, Option<&'a str>), String> {
        if request.source_turn_id.as_deref().is_none_or(str::is_empty) {
            return Err(
                "Cannot fork Pi at a specific turn without a native turn reference.".into(),
            );
        }
        if request.rollback_turns.is_some() {
            return Err("Pi cannot trim a head fork by turn count.".into());
        }
        if request.cwd.is_empty() {
            // `--fork` re-homes the copy to the process cwd; never the app's own.
            return Err("A Pi fork needs a destination working directory.".into());
        }
        let source = PathBuf::from(&request.source_thread_id);
        if !source.is_file() || !launch::within_roots(&source, roots) {
            return Err("The source Pi session file is not available on this device.".into());
        }
        let cut = request.source_next_turn_id.as_deref().filter(|id| !id.is_empty());
        if let Some(cut) = cut
            && !is_user_entry(&source, cut)
        {
            return Err(
                "The next turn's entry is not a user message in the source Pi session.".into(),
            );
        }
        Ok((source, cut))
    }
}

#[async_trait]
impl SessionLifecycle for PiHarness {
    /// Turn refs are session-tree user entries, which `fork` re-roots before.
    fn can_fork_from_turn(&self) -> bool {
        true
    }

    async fn can_fork_now(&self, request: &NativeForkRequest) -> Result<bool, HarnessError> {
        let request_copy = NativeForkRequest {
            source_thread_id: request.source_thread_id.clone(),
            source_turn_id: request.source_turn_id.clone(),
            source_next_turn_id: request.source_next_turn_id.clone(),
            rollback_turns: request.rollback_turns,
            cwd: request.cwd.clone(),
            model: request.model.clone(),
            runtime_mode: request.runtime_mode,
            interaction_mode: request.interaction_mode,
            mcp: request.mcp.clone(),
        };
        let roots = self.session_roots_for(&request.cwd)?;
        // File scans of large transcripts stay off the async workers.
        tokio::task::spawn_blocking(move || Self::fork_plan(&request_copy, &roots).is_ok())
            .await
            .map_err(|e| HarnessError::Protocol(e.to_string()))
            .and_then(|plannable| {
                self.resolve_executable()?;
                Ok(plannable)
            })
    }

    async fn can_resume_now(&self, native_thread_id: &str, cwd: &str) -> Result<bool, HarnessError> {
        let roots = self.session_roots_for(cwd)?;
        let wanted = native_thread_id.to_owned();
        tokio::task::spawn_blocking(move || {
            !matches!(
                launch::resolve_resume(Some(&wanted), &roots, Some(&launch::acp_session_map())),
                launch::Resume::Missing
            )
        })
        .await
        .map_err(|e| HarnessError::Protocol(e.to_string()))
    }

    async fn fork_thread(&self, request: NativeForkRequest) -> Result<String, HarnessError> {
        let roots = self.session_roots_for(&request.cwd)?;
        let (source, cut) = Self::fork_plan(&request, &roots).map_err(HarnessError::Protocol)?;
        let exe = self.resolve_executable()?;
        self.check_version(&exe).await?;
        let mut args = launch::without_extensions_and_tools(&self.user_args()?);
        args.extend([
            "--fork".to_owned(),
            source.display().to_string(),
            "--no-extensions".to_owned(),
            "--no-tools".to_owned(),
        ]);
        let Spawned {
            mut child,
            rpc,
            mut events,
            stderr_tail,
        } = self.spawn(self.command(&exe, Some(&request.cwd), &args))?;
        // Nothing here may prompt; refuse any dialog and discard the rest.
        let canceller = rpc.clone();
        let drain = tokio::spawn(async move {
            while let Some(record) = events.recv().await {
                if record.get("type").and_then(Value::as_str) == Some("extension_ui_request")
                    && let Some(id) = record.get("id").and_then(Value::as_str)
                {
                    let _ = canceller.send(json!({
                        "type": "extension_ui_response", "id": id, "cancelled": true,
                    }));
                }
            }
        });
        let forked = async {
            let session_file = |state: &Value| {
                state
                    .get("sessionFile")
                    .and_then(Value::as_str)
                    .filter(|file| !file.is_empty())
                    .map(PathBuf::from)
            };
            let copied = session_file(
                &rpc.request(json!({"type": "get_state"}), FORK_TIMEOUT).await?,
            )
            .ok_or_else(|| HarnessError::Protocol("Pi fork reported no session file".into()))?;
            let Some(cut) = cut else {
                return Ok::<_, HarnessError>((copied.clone(), None));
            };
            let result = rpc
                .request(json!({"type": "fork", "entryId": cut}), FORK_TIMEOUT)
                .await?;
            if result.get("cancelled").and_then(Value::as_bool) == Some(true) {
                return Err(HarnessError::Protocol(
                    "A Pi extension cancelled the session fork".into(),
                ));
            }
            let cut_file = session_file(
                &rpc.request(json!({"type": "get_state"}), FORK_TIMEOUT).await?,
            )
            .ok_or_else(|| HarnessError::Protocol("Pi fork reported no session file".into()))?;
            Ok((cut_file, Some(copied)))
        };
        let outcome = tokio::time::timeout(FORK_TIMEOUT, forked).await;
        drain.abort();
        let died = child.try_wait().ok().flatten();
        child.shutdown(self.kill_grace).await;
        let (file, _copy) = match outcome {
            Ok(Ok(found)) => found,
            Ok(Err(error)) => {
                return Err(match died {
                    Some(_) => {
                        HarnessError::Protocol(crate::crash_message("pi", died, &stderr_tail))
                    }
                    None => error,
                });
            }
            Err(_) => {
                return Err(HarnessError::Transport(
                    "Pi did not finish the session fork in time".into(),
                ));
            }
        };
        if file == source {
            return Err(HarnessError::Protocol(
                "Pi fork did not create a distinct session file".into(),
            ));
        }
        Ok(file.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(dir: &Path) -> PathBuf {
        let file = dir.join("s.jsonl");
        std::fs::write(
            &file,
            [
                r#"{"type":"session","version":3,"id":"s","cwd":"/w"}"#,
                r#"{"type":"model_change","id":"m1","parentId":null}"#,
                r#"{"type":"message","id":"u1","parentId":"m1","message":{"role":"user","content":"hi"}}"#,
                r#"{"type":"message","id":"a1","parentId":"u1","message":{"role":"assistant","content":"yo"}}"#,
                r#"{"type":"message","id":"u2","parentId":"a1","message":{"role":"user","content":"again"}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        file
    }

    #[test]
    fn only_user_message_entries_are_cut_points() {
        let dir = tempfile::tempdir().unwrap();
        let file = session(dir.path());
        assert!(is_user_entry(&file, "u1"));
        assert!(is_user_entry(&file, "u2"));
        // Pi's fork rejects anything but a user message ("Invalid entry ID").
        assert!(!is_user_entry(&file, "a1"));
        assert!(!is_user_entry(&file, "m1"));
        assert!(!is_user_entry(&file, "missing"));
        assert!(!is_user_entry(&file, ""));
        assert!(!is_user_entry(&file, "u1\",\"x"));
        assert!(!is_user_entry(&dir.path().join("gone.jsonl"), "u1"));
    }

    #[test]
    fn the_leaf_is_the_last_complete_entry_read_from_the_tail() {
        let dir = tempfile::tempdir().unwrap();
        let file = session(dir.path());
        assert_eq!(session_leaf(&file), Leaf::Entry("u2".into()));
        // A torn trailing write is skipped.
        let torn = dir.path().join("torn.jsonl");
        let mut text = std::fs::read_to_string(&file).unwrap();
        text.push_str("\n{\"type\":\"message\",\"id\":\"u3\",\"mess");
        std::fs::write(&torn, text).unwrap();
        assert_eq!(session_leaf(&torn), Leaf::Entry("u2".into()));
        // A header alone, an empty file and a missing file are all empty sessions.
        let header = dir.path().join("header.jsonl");
        std::fs::write(&header, "{\"type\":\"session\",\"id\":\"s\"}\n").unwrap();
        assert_eq!(session_leaf(&header), Leaf::Empty);
        std::fs::write(dir.path().join("empty.jsonl"), "").unwrap();
        assert_eq!(session_leaf(&dir.path().join("empty.jsonl")), Leaf::Empty);
        assert_eq!(session_leaf(&dir.path().join("gone.jsonl")), Leaf::Empty);
    }

    #[test]
    fn the_leaf_of_a_huge_session_does_not_need_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.jsonl");
        let filler = "x".repeat(1 << 20);
        let mut text = String::from("{\"type\":\"session\",\"id\":\"s\"}\n");
        for n in 0..24 {
            text.push_str(&format!(
                "{{\"type\":\"message\",\"id\":\"m{n}\",\"message\":{{\"role\":\"user\",\"content\":\"{filler}\"}}}}\n"
            ));
        }
        std::fs::write(&big, &text).unwrap();
        assert_eq!(session_leaf(&big), Leaf::Entry("m23".into()));
        // The last entry alone is larger than the first window.
        let tail = dir.path().join("tail.jsonl");
        let single = format!(
            "{{\"type\":\"session\",\"id\":\"s\"}}\n{{\"type\":\"message\",\"id\":\"only\",\"message\":\"{}\"}}\n",
            "y".repeat(3 << 16)
        );
        std::fs::write(&tail, single).unwrap();
        assert_eq!(session_leaf(&tail), Leaf::Entry("only".into()));
        // Beyond the read cap the leaf is unknown, never guessed.
        let huge = dir.path().join("huge.jsonl");
        std::fs::write(
            &huge,
            format!(
                "{{\"type\":\"session\",\"id\":\"s\"}}\n{{\"id\":\"z\",\"pad\":\"{}\"}}\n",
                "z".repeat(TAIL_CAP as usize + 10)
            ),
        )
        .unwrap();
        assert_eq!(session_leaf(&huge), Leaf::Unknown);
    }

    fn request(source: &Path, turn: Option<&str>, next: Option<&str>) -> NativeForkRequest {
        NativeForkRequest {
            source_thread_id: source.display().to_string(),
            source_turn_id: turn.map(str::to_owned),
            source_next_turn_id: next.map(str::to_owned),
            rollback_turns: None,
            cwd: "dest".into(),
            model: String::new(),
            runtime_mode: Default::default(),
            interaction_mode: Default::default(),
            mcp: Default::default(),
        }
    }

    #[test]
    fn the_fork_boundary_must_be_provable_before_anything_runs() {
        let dir = tempfile::tempdir().unwrap();
        let file = session(dir.path());
        let roots = [dir.path().to_path_buf()];
        // Head fork of a known turn, and a cut before the next turn's entry.
        assert_eq!(PiHarness::fork_plan(&request(&file, Some("u1"), None), &roots).unwrap().1, None);
        assert_eq!(
            PiHarness::fork_plan(&request(&file, Some("u1"), Some("u2")), &roots).unwrap().1,
            Some("u2")
        );
        // Never fork a moving head with an unknown boundary.
        assert!(PiHarness::fork_plan(&request(&file, None, None), &roots)
            .unwrap_err()
            .contains("native turn reference"));
        // The next turn's entry must be a user message of THIS session.
        assert!(PiHarness::fork_plan(&request(&file, Some("u1"), Some("a1")), &roots).is_err());
        assert!(PiHarness::fork_plan(&request(&file, Some("u1"), Some("nope")), &roots).is_err());
        // Legacy ACP ids and vanished files fall back to portable context.
        assert!(PiHarness::fork_plan(&request(Path::new("01a11698-d036"), Some("u1"), None), &roots).is_err());
        let mut counted = request(&file, Some("u1"), None);
        counted.rollback_turns = Some(2);
        // A fork with no destination would re-home into the app's own cwd.
        let mut homeless = request(&file, Some("u1"), None);
        homeless.cwd = String::new();
        assert!(PiHarness::fork_plan(&homeless, &roots).unwrap_err().contains("working directory"));
        // Only Pi's own session files can be forked.
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(PiHarness::fork_plan(&request(&file, Some("u1"), None), &[elsewhere.path().to_path_buf()]).is_err());
        assert!(PiHarness::fork_plan(&counted, &roots).is_err());
    }
}
