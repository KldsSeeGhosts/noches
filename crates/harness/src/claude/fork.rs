//! Lazy native fork for the Claude CLI.
//!
//! The CLI has no standalone fork primitive: `--resume <parent> --fork-session`
//! writes nothing and prints no `init` until a user turn runs, and
//! `--resume-session-at <assistant uuid>` (hidden) cuts the resumed history at
//! a message. So a fork is a *deferred native id*: [`ForkToken`] is minted at
//! fork time (no process starts), travels through the host as the child's
//! native thread id, and the child's first run expands it into
//! `--resume <parent> --fork-session --session-id <child> [--resume-session-at <uuid>]`.
//! The child id is chosen up front, so a start that fails before `init` can be
//! retried without forking twice: once the child's transcript exists the token
//! resolves to a plain `--resume <child>`. After `init` the host records the
//! real session id and the token is never consulted again.

use std::path::{Path, PathBuf};

const PREFIX: &str = "claude-fork:v1:";

/// A fork of `parent` (cut after assistant message `at`) that becomes session
/// `child` when its first turn runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForkToken {
    pub child: String,
    pub parent: String,
    pub at: Option<String>,
}

fn is_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl ForkToken {
    pub fn mint(parent: &str, at: Option<&str>) -> Option<Self> {
        (is_id(parent) && at.is_none_or(is_id)).then(|| Self {
            child: uuid::Uuid::new_v4().to_string(),
            parent: parent.to_owned(),
            at: at.map(str::to_owned),
        })
    }

    pub fn encode(&self) -> String {
        format!(
            "{PREFIX}{}:{}:{}",
            self.child,
            self.parent,
            self.at.as_deref().unwrap_or("-")
        )
    }

    /// None for an ordinary session id (or anything malformed: never guess).
    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.strip_prefix(PREFIX)?.split(':');
        let (child, parent, at) = (parts.next()?, parts.next()?, parts.next()?);
        (parts.next().is_none() && is_id(child) && is_id(parent) && (at == "-" || is_id(at)))
            .then(|| Self {
                child: child.to_owned(),
                parent: parent.to_owned(),
                at: (at != "-").then(|| at.to_owned()),
            })
    }
}

/// Session ids are global across project directories (see
/// `Normalizer::restore_spawns`), so search rather than re-derive the CLI's
/// cwd encoding.
pub(crate) fn session_file(config_root: &Path, session_id: &str) -> Option<PathBuf> {
    if !is_id(session_id) {
        return None;
    }
    std::fs::read_dir(config_root.join("projects"))
        .ok()?
        .flatten()
        .map(|project| project.path().join(format!("{session_id}.jsonl")))
        .find(|path| path.is_file())
}

/// What a fork's first run is allowed to do, decided from the native
/// transcripts alone (no process, no network).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ForkPlan {
    /// The child already exists (an earlier start got far enough to write it).
    ResumeChild,
    /// Fork the parent now.
    Fork,
    /// Neither the child nor a usable parent exists: forking would invent a
    /// conversation, so the caller must use portable context instead.
    Unavailable,
}

pub(crate) fn plan(config_root: &Path, token: &ForkToken) -> ForkPlan {
    if session_file(config_root, &token.child).is_some() {
        ForkPlan::ResumeChild
    } else if session_file(config_root, &token.parent).is_some() {
        ForkPlan::Fork
    } else {
        ForkPlan::Unavailable
    }
}

/// The transcript can be forked at `at` from `cwd`: the parent was recorded in
/// that same working directory (the CLI resolves `--resume` per project) and
/// `at` is one of its top-level assistant messages. A moving head is never
/// forked: callers always pin a message.
pub(crate) fn can_fork_at(
    config_root: &Path,
    parent: &str,
    at: &str,
    cwd: &str,
) -> bool {
    use std::io::BufRead as _;
    let Some(path) = session_file(config_root, parent) else {
        return false;
    };
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let wanted = format!("\"uuid\":\"{at}\"");
    let mut same_cwd = cwd.is_empty();
    let mut found = false;
    let mut reader = std::io::BufReader::with_capacity(1 << 16, file);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        // Only lines that can matter are parsed: transcripts run to hundreds
        // of MB.
        let is_target = contains(&line, wanted.as_bytes());
        let needs_cwd = !same_cwd && contains(&line, b"\"cwd\":\"");
        if !is_target && !needs_cwd {
            continue;
        }
        let Ok(record) = serde_json::from_slice::<serde_json::Value>(&line) else {
            continue;
        };
        if needs_cwd
            && record
                .get("cwd")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|recorded| same_dir(recorded, cwd))
        {
            same_cwd = true;
        }
        if is_target
            && record.get("uuid").and_then(serde_json::Value::as_str) == Some(at)
            && record.get("type").and_then(serde_json::Value::as_str) == Some("assistant")
            && record.get("isSidechain").and_then(serde_json::Value::as_bool) != Some(true)
        {
            found = true;
        }
        if found && same_cwd {
            return true;
        }
    }
    false
}

fn same_dir(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(sessions: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("projects").join("-tmp-p");
        std::fs::create_dir_all(&project).unwrap();
        for (id, body) in sessions {
            std::fs::write(project.join(format!("{id}.jsonl")), body).unwrap();
        }
        root
    }

    #[test]
    fn tokens_round_trip_and_ordinary_ids_are_not_tokens() {
        let token = ForkToken::mint("parent-1", Some("asst-9")).unwrap();
        assert_eq!(ForkToken::parse(&token.encode()), Some(token.clone()));
        let head = ForkToken::mint("parent-1", None).unwrap();
        assert_eq!(ForkToken::parse(&head.encode()).unwrap().at, None);
        assert_eq!(ForkToken::parse("sess-1"), None);
        assert_eq!(ForkToken::parse("claude-fork:v1:a:b"), None);
        assert_eq!(ForkToken::parse("claude-fork:v1:a:b:c:d"), None);
        assert_eq!(ForkToken::parse("claude-fork:v1:a:../b:-"), None);
        assert!(ForkToken::mint("../etc", None).is_none());
    }

    #[test]
    fn plan_prefers_an_existing_child_and_never_invents_a_parent() {
        let token = ForkToken {
            child: "child".into(),
            parent: "parent".into(),
            at: None,
        };
        assert_eq!(
            plan(config_with(&[("parent", "")]).path(), &token),
            ForkPlan::Fork
        );
        assert_eq!(
            plan(config_with(&[("parent", ""), ("child", "")]).path(), &token),
            ForkPlan::ResumeChild
        );
        assert_eq!(plan(config_with(&[]).path(), &token), ForkPlan::Unavailable);
    }

    #[test]
    fn only_a_top_level_assistant_message_in_the_same_cwd_is_a_fork_point() {
        let cwd = tempfile::tempdir().unwrap();
        let cwd_str = cwd.path().to_str().unwrap();
        let body = format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":{cwd:?}}}\n\
             {{\"type\":\"assistant\",\"uuid\":\"a1\",\"cwd\":{cwd:?}}}\n\
             {{\"type\":\"assistant\",\"uuid\":\"side\",\"isSidechain\":true}}\n",
            cwd = cwd_str
        );
        let root = config_with(&[("parent", &body)]);
        assert!(can_fork_at(root.path(), "parent", "a1", cwd_str));
        assert!(!can_fork_at(root.path(), "parent", "u1", cwd_str));
        assert!(!can_fork_at(root.path(), "parent", "side", cwd_str));
        assert!(!can_fork_at(root.path(), "parent", "missing", cwd_str));
        assert!(!can_fork_at(root.path(), "gone", "a1", cwd_str));
        let other = tempfile::tempdir().unwrap();
        assert!(!can_fork_at(
            root.path(),
            "parent",
            "a1",
            other.path().to_str().unwrap()
        ));
    }
}
