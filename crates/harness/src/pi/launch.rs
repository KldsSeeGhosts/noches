//! Everything needed to start a Pi process correctly: user launch-argument
//! validation, version gating, Pi's own settings/session-store conventions, and
//! the resume target.
//!
//! Design intent (shared with T3's adapter): the process is spawned with no
//! `--no-*` flags, so the user's extensions, skills, prompt templates,
//! AGENTS.md context, settings.json, custom models and auth load exactly as in
//! the `pi` TUI. Sessions are stored by Pi itself and the session file path is
//! the durable native thread id, so a thread started in Noches can be resumed
//! from the TUI and vice versa.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Oldest Pi with the RPC surface this driver depends on (`get_entries`,
/// `agent_settled`, `streamingBehavior`, `fork`/`switch_session`).
pub(crate) const MINIMUM_VERSION: (u64, u64, u64) = (0, 80, 5);

pub(crate) fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    text.split_whitespace().find_map(|word| {
        let mut parts = word.trim_start_matches('v').split('.');
        let version = (
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
            parts
                .next()?
                .split(|c: char| !c.is_ascii_digit())
                .next()?
                .parse()
                .ok()?,
        );
        Some(version)
    })
}

pub(crate) fn version_error(found: Option<(u64, u64, u64)>) -> Option<String> {
    let (major, minor, patch) = MINIMUM_VERSION;
    match found {
        Some(version) if version >= MINIMUM_VERSION => None,
        Some((a, b, c)) => Some(format!(
            "Pi {a}.{b}.{c} is too old: Noches needs Pi {major}.{minor}.{patch} or newer (run `pi update self`)."
        )),
        None => Some(format!(
            "Noches could not determine the Pi version; Pi {major}.{minor}.{patch} or newer is required."
        )),
    }
}

// ---------------------------------------------------------------------------
// User launch arguments (instance `launchArgs`)
// ---------------------------------------------------------------------------

/// Arguments that select a different execution mode or native session. Noches
/// owns RPC mode and session identity.
const RESERVED: &[&str] = &[
    "--continue",
    "-c",
    "--export",
    "--fork",
    "--help",
    "-h",
    "--list-models",
    "--mode",
    "--no-session",
    "--print",
    "-p",
    "--resume",
    "-r",
    "--session",
    "--session-id",
    "--version",
    "-v",
];

const WITH_VALUE: &[&str] = &[
    "--api-key",
    "--append-system-prompt",
    "--exclude-tools",
    "-xt",
    "--extension",
    "-e",
    "--model",
    "--models",
    "--name",
    "-n",
    "--prompt-template",
    "--provider",
    "--session-dir",
    "--skill",
    "--system-prompt",
    "--theme",
    "--thinking",
    "--tools",
    "-t",
    "--tui-mode",
    "--use-theme",
];

const WITHOUT_VALUE: &[&str] = &[
    "--approve",
    "-a",
    "--no-approve",
    "-na",
    "--no-builtin-tools",
    "-nbt",
    "--no-context-files",
    "-nc",
    "--no-extensions",
    "-ne",
    "--no-prompt-templates",
    "-np",
    "--no-skills",
    "-ns",
    "--no-themes",
    "--no-tools",
    "-nt",
    "--offline",
    "--verbose",
];

fn reserved(arg: &str) -> Option<&'static str> {
    RESERVED.iter().copied().find(|reserved| {
        arg == *reserved || (reserved.starts_with("--") && arg.starts_with(&format!("{reserved}=")))
    })
}

/// Pi parses equals-form tokens only as extension flags, even when their name
/// matches a built-in option, so split known built-ins and leave arbitrary
/// extension flags in their native form.
fn normalize_equals(args: &[String]) -> Vec<String> {
    args.iter()
        .flat_map(|arg| match arg.split_once('=') {
            Some((option, value)) if !option.is_empty() && WITH_VALUE.contains(&option) => {
                vec![option.to_owned(), value.to_owned()]
            }
            _ => vec![arg.clone()],
        })
        .collect()
}

/// Validate and normalize the user's launch arguments. They may configure
/// resources, models, tools, trust and storage; anything that would change the
/// execution mode, pick a native session, or smuggle in a prompt is rejected
/// before spawn so every launch site reports the reason, not a bare exit code.
pub(crate) fn resolve_launch_args(raw: &[String]) -> Result<Vec<String>, String> {
    let args = normalize_equals(raw);
    let (mut has_provider, mut has_model) = (false, false);
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        index += 1;
        if let Some(reserved) = reserved(arg) {
            return Err(format!(
                "Pi launch argument '{reserved}' is controlled by Noches and cannot be overridden."
            ));
        }
        if arg == "--" {
            return Err("Pi launch arguments cannot include positional prompts.".into());
        }
        if WITH_VALUE.contains(&arg) {
            if args.get(index).is_none() {
                return Err(format!("Pi launch argument '{arg}' requires a value."));
            }
            has_provider |= arg == "--provider";
            has_model |= arg == "--model";
            index += 1;
            continue;
        }
        if WITHOUT_VALUE.contains(&arg) || (arg.starts_with("--") && arg.contains('=')) {
            continue;
        }
        if arg.starts_with("--") {
            // Pi extensions may register arbitrary long flags. Treat one
            // following non-flag token as that extension flag's value.
            if let Some(next) = args.get(index)
                && !next.starts_with('-')
                && !next.starts_with('@')
            {
                index += 1;
            }
            continue;
        }
        if arg.starts_with('-') {
            return Err(format!(
                "Pi launch argument '{arg}' is not supported by Noches."
            ));
        }
        return Err(format!(
            "Pi launch arguments cannot include positional prompt '{arg}'."
        ));
    }
    // Pi 1.0 exits at startup on `--provider` without `--model`.
    if has_provider && !has_model {
        return Err("Pi launch argument '--provider' requires '--model'.".into());
    }
    Ok(args)
}

/// `--session-dir` from validated launch args.
pub(crate) fn session_dir_arg(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find(|pair| pair[0] == "--session-dir")
        .map(|pair| pair[1].as_str())
}

/// Drop `-e/--extension` pairs and tool selection, for the short-lived
/// fork process that must never run user code or tools.
pub(crate) fn without_extensions_and_tools(args: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--extension" | "-e" | "--tools" | "-t" | "--exclude-tools" | "-xt" => {
                iter.next();
            }
            "--no-tools" | "-nt" | "--no-builtin-tools" | "-nbt" => {}
            _ => kept.push(arg.clone()),
        }
    }
    kept
}

// ---------------------------------------------------------------------------
// Pi's configuration and session store
// ---------------------------------------------------------------------------

/// `PI_CODING_AGENT_DIR` as the child will see it: the instance's own value
/// first, then the engine's, else `~/.pi/agent`.
pub(crate) fn agent_dir(env: &BTreeMap<String, String>) -> PathBuf {
    let configured = env
        .get("PI_CODING_AGENT_DIR")
        .cloned()
        .or_else(|| std::env::var("PI_CODING_AGENT_DIR").ok())
        .filter(|dir| !dir.is_empty());
    match configured {
        Some(dir) => expand_home(&dir),
        None => crate::executable::home_or_current_dir()
            .join(".pi")
            .join("agent"),
    }
}

fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        Some(rest) => crate::executable::home_or_current_dir().join(rest),
        None if path == "~" => crate::executable::home_or_current_dir(),
        None => PathBuf::from(path),
    }
}

/// The settings Noches mirrors from Pi's own `settings.json` (global, then the
/// project's `.pi/settings.json`, key by key).
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct PiSettings {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_thinking: Option<String>,
    pub reserve_tokens: Option<u64>,
    /// `compaction.modelOverrides[provider/id].reserveTokens`.
    pub reserve_overrides: BTreeMap<String, u64>,
    pub session_dir: Option<String>,
}

impl PiSettings {
    pub(crate) fn read(agent_dir: &Path, cwd: &Path) -> Self {
        let mut settings = Self::default();
        for path in [
            agent_dir.join("settings.json"),
            cwd.join(".pi").join("settings.json"),
        ] {
            // A missing or malformed file leaves the previous layer.
            if let Ok(text) = std::fs::read_to_string(path)
                && let Ok(value) = serde_json::from_str::<Value>(&text)
            {
                settings.layer(&value);
            }
        }
        settings
    }

    fn layer(&mut self, value: &Value) {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        };
        self.default_provider = text("defaultProvider").or(self.default_provider.take());
        self.default_model = text("defaultModel").or(self.default_model.take());
        self.default_thinking = text("defaultThinkingLevel").or(self.default_thinking.take());
        self.session_dir = text("sessionDir").or(self.session_dir.take());
        let tokens = |value: Option<&Value>| {
            value
                .and_then(Value::as_u64)
                .filter(|v| *v <= (1u64 << 53) - 1)
        };
        if let Some(compaction) = value.get("compaction") {
            self.reserve_tokens = tokens(compaction.get("reserveTokens")).or(self.reserve_tokens);
            for (slug, entry) in compaction
                .get("modelOverrides")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                if let Some(reserve) = tokens(entry.get("reserveTokens")) {
                    self.reserve_overrides.insert(slug.clone(), reserve);
                }
            }
        }
    }

    /// Where auto-compaction triggers for `model`: `contextWindow -
    /// reserveTokens`. Never guessed: an invalid reserve leaves it unknown.
    pub(crate) fn compact_at(&self, model: &str, window: u64) -> Option<u64> {
        let reserve = self
            .reserve_overrides
            .get(model)
            .copied()
            .or(self.reserve_tokens)
            .unwrap_or(DEFAULT_RESERVE_TOKENS);
        let threshold = window.checked_sub(reserve)?;
        (threshold > 0 && threshold < window).then_some(threshold)
    }
}

pub(crate) const DEFAULT_RESERVE_TOKENS: u64 = 16_384;

/// Every directory Pi may keep this run's sessions in, most specific first:
/// the launch `--session-dir`, the env override, the `sessionDir` setting,
/// then `<agent dir>/sessions`.
pub(crate) fn session_roots(
    args: &[String],
    env: &BTreeMap<String, String>,
    settings: &PiSettings,
    agent_dir: &Path,
    cwd: &Path,
) -> Vec<PathBuf> {
    let resolve = |dir: &str| {
        let path = expand_home(dir);
        if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        }
    };
    let mut roots = Vec::new();
    if let Some(dir) = session_dir_arg(args) {
        roots.push(resolve(dir));
    }
    if let Some(dir) = env
        .get("PI_CODING_AGENT_SESSION_DIR")
        .cloned()
        .or_else(|| std::env::var("PI_CODING_AGENT_SESSION_DIR").ok())
        .filter(|dir| !dir.is_empty())
    {
        roots.push(resolve(&dir));
    }
    if let Some(dir) = &settings.session_dir {
        roots.push(resolve(dir));
    }
    roots.push(agent_dir.join("sessions"));
    roots.dedup();
    roots
}

fn is_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Find a session file by Pi's session id. Files are named
/// `<timestamp>_<id>.jsonl`, either directly under a root (`--session-dir`) or
/// one level down, grouped by working directory.
pub(crate) fn find_session_file(roots: &[PathBuf], id: &str) -> Option<PathBuf> {
    if !is_session_id(id) {
        return None;
    }
    let suffix = format!("_{id}.jsonl");
    let matches = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(&suffix))
    };
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && matches(&path) {
                return Some(path);
            }
            if path.is_dir()
                && let Ok(children) = std::fs::read_dir(&path)
                && let Some(found) = children.flatten().map(|c| c.path()).find(|p| matches(p))
            {
                return Some(found);
            }
        }
    }
    None
}

/// What a run's `resume` value resolves to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resume {
    /// No resume requested.
    Fresh,
    /// An existing session file.
    File(PathBuf),
    /// A resume was requested but its session no longer exists. `--session`
    /// on a missing path would silently create a NEW session at that path, so
    /// the caller starts fresh explicitly instead.
    Missing,
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.ends_with(".jsonl")
}

/// Native ids are session file paths; legacy ids stored by the old ACP adapter
/// are Pi session UUIDs and are resolved through the session store.
pub(crate) fn resolve_resume(resume: Option<&str>, roots: &[PathBuf]) -> Resume {
    let Some(resume) = resume.filter(|value| !value.is_empty()) else {
        return Resume::Fresh;
    };
    let found = if looks_like_path(resume) {
        let path = PathBuf::from(resume);
        path.is_file().then_some(path)
    } else {
        find_session_file(roots, resume)
    };
    found.map_or(Resume::Missing, Resume::File)
}

/// `/compact [instructions]` maps to the RPC `compact` command: Pi's built-in
/// TUI commands are not prompt-addressable in RPC mode.
pub(crate) fn parse_compact_command(text: &str) -> Option<Option<String>> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix("/compact")?;
    if rest.is_empty() {
        return Some(None);
    }
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let instructions = rest.trim();
    Some((!instructions.is_empty()).then(|| instructions.to_owned()))
}

/// Hoist `$skill` references to Pi's native leading `/skill:name` position,
/// preserving the rest of the prompt. Only skills Pi reported are rewritten.
pub(crate) fn expand_skill_references(
    text: &str,
    skills: &std::collections::HashSet<String>,
) -> String {
    let mut ordered: Vec<&str> = Vec::new();
    let mut body = String::new();
    let mut previous_blank = true;
    let mut chars = text.char_indices().peekable();
    let mut last = 0;
    while let Some((index, ch)) = chars.next() {
        if ch == '$' && previous_blank {
            let rest = &text[index + 1..];
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let name = &rest[..end];
            if !name.is_empty() && skills.contains(name) {
                body.push_str(&text[last..index]);
                last = index + 1 + end;
                if !ordered.contains(&name) {
                    ordered.push(name);
                }
                while chars.peek().is_some_and(|(i, _)| *i < last) {
                    chars.next();
                }
                previous_blank = false;
                continue;
            }
        }
        previous_blank = ch.is_whitespace();
    }
    if ordered.is_empty() {
        return text.to_owned();
    }
    body.push_str(&text[last..]);
    let body = body.trim();
    let prefix = ordered
        .iter()
        .map(|name| format!("/skill:{name}"))
        .collect::<Vec<_>>()
        .join(" ");
    if body.is_empty() {
        prefix
    } else {
        format!("{prefix} {body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: &str) -> Vec<String> {
        value.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn versions_parse_from_bare_and_prefixed_output() {
        assert_eq!(parse_version("1.0.4"), Some((1, 0, 4)));
        assert_eq!(parse_version("pi v0.80.5\n"), Some((0, 80, 5)));
        assert_eq!(parse_version("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("no version"), None);
        assert!(version_error(Some((1, 0, 0))).is_none());
        assert!(version_error(Some((0, 80, 5))).is_none());
        assert!(version_error(Some((0, 79, 9))).unwrap().contains("pi update self"));
        assert!(version_error(None).unwrap().contains("could not determine"));
    }

    #[test]
    fn user_args_may_configure_pi_but_not_its_mode_or_session() {
        let resolved = resolve_launch_args(&[
            "--extension=/home/user/.pi/agent/extensions/demo.ts".into(),
            "--session-dir=/tmp/pi-sessions".into(),
            "--provider=anthropic".into(),
            "--model=claude-sonnet".into(),
            "--tools=".into(),
            "--name=-review".into(),
            "--extension-flag=kept".into(),
        ])
        .unwrap();
        assert_eq!(
            resolved,
            [
                "--extension",
                "/home/user/.pi/agent/extensions/demo.ts",
                "--session-dir",
                "/tmp/pi-sessions",
                "--provider",
                "anthropic",
                "--model",
                "claude-sonnet",
                "--tools",
                "",
                "--name",
                "-review",
                "--extension-flag=kept",
            ]
        );
        assert_eq!(session_dir_arg(&resolved), Some("/tmp/pi-sessions"));
        for (rejected, needle) in [
            ("--mode text", "'--mode' is controlled by Noches"),
            ("--session old.jsonl", "'--session' is controlled"),
            ("--fork=x", "'--fork' is controlled"),
            ("--continue", "'--continue' is controlled"),
            ("prompt pi immediately", "positional prompt 'prompt'"),
            ("--plan @instructions.md", "positional prompt '@instructions.md'"),
            ("-z", "'-z' is not supported"),
            ("--model", "requires a value"),
            ("-- hello", "positional prompts"),
        ] {
            let error = resolve_launch_args(&args(rejected)).unwrap_err();
            assert!(error.contains(needle), "{rejected}: {error}");
        }
    }

    #[test]
    fn provider_requires_model_which_pi_1_refuses_at_startup() {
        for rejected in ["--provider openrouter", "--provider=openrouter --models gpt-6"] {
            assert_eq!(
                resolve_launch_args(&args(rejected)).unwrap_err(),
                "Pi launch argument '--provider' requires '--model'."
            );
        }
        assert!(resolve_launch_args(&args("--provider openrouter --model=deepseek/v4")).is_ok());
        assert!(resolve_launch_args(&args("--model deepseek/v4")).is_ok());
    }

    #[test]
    fn the_fork_process_never_runs_user_extensions_or_tools() {
        let launch = args("--provider anthropic --model m --extension a.ts -e b.ts --tools read -nt --offline");
        assert_eq!(
            without_extensions_and_tools(&launch),
            args("--provider anthropic --model m --offline")
        );
    }

    #[test]
    fn settings_layer_project_over_global_key_by_key() {
        let dir = tempfile::tempdir().unwrap();
        let (agent, project) = (dir.path().join("agent"), dir.path().join("project"));
        std::fs::create_dir_all(&agent).unwrap();
        std::fs::create_dir_all(project.join(".pi")).unwrap();
        std::fs::write(
            agent.join("settings.json"),
            r#"{"defaultProvider":"cpa","defaultModel":"gemini-3.8-flash","defaultThinkingLevel":"medium",
                "compaction":{"reserveTokens":27000,"modelOverrides":{"cpa/big":{"reserveTokens":400000}}}}"#,
        )
        .unwrap();
        std::fs::write(
            project.join(".pi").join("settings.json"),
            r#"{"defaultModel":"other","compaction":{"reserveTokens":"junk"}}"#,
        )
        .unwrap();
        let settings = PiSettings::read(&agent, &project);
        assert_eq!(settings.default_provider.as_deref(), Some("cpa"));
        assert_eq!(settings.default_model.as_deref(), Some("other"));
        assert_eq!(settings.default_thinking.as_deref(), Some("medium"));
        // An invalid project value leaves the global one in place.
        assert_eq!(settings.reserve_tokens, Some(27_000));
        assert_eq!(settings.compact_at("cpa/x", 1_048_576), Some(1_048_576 - 27_000));
        assert_eq!(settings.compact_at("cpa/big", 1_000_000), Some(600_000));
        // A reserve that swallows the whole window is unknown, not guessed.
        assert_eq!(settings.compact_at("cpa/big", 400_000), None);
        assert_eq!(PiSettings::default().compact_at("m", 200_000), Some(200_000 - 16_384));
        // Missing files are not an error.
        assert_eq!(
            PiSettings::read(&dir.path().join("nope"), &dir.path().join("nope2")),
            PiSettings::default()
        );
    }

    #[test]
    fn sessions_resolve_by_path_or_legacy_id_and_missing_is_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sessions");
        let grouped = root.join("--work--");
        std::fs::create_dir_all(&grouped).unwrap();
        let id = "01a11698-d036-75cf-ad2b-434fcca3e719";
        let file = grouped.join(format!("2026-10-07T13-41-15-958Z_{id}.jsonl"));
        std::fs::write(&file, "{}\n").unwrap();
        let flat = root.join("2026-10-07T13-41-16-000Z_aaaa-bbbb.jsonl");
        std::fs::write(&flat, "{}\n").unwrap();
        let roots = vec![root];

        assert_eq!(resolve_resume(None, &roots), Resume::Fresh);
        assert_eq!(resolve_resume(Some(""), &roots), Resume::Fresh);
        assert_eq!(
            resolve_resume(Some(file.to_str().unwrap()), &roots),
            Resume::File(file.clone())
        );
        assert_eq!(resolve_resume(Some(id), &roots), Resume::File(file));
        assert_eq!(
            resolve_resume(Some("aaaa-bbbb"), &roots),
            Resume::File(flat)
        );
        // A vanished file must not be handed to `--session` (it would create
        // a new session at that path).
        let gone = dir.path().join("gone.jsonl");
        assert_eq!(resolve_resume(Some(gone.to_str().unwrap()), &roots), Resume::Missing);
        assert_eq!(resolve_resume(Some("deadbeef"), &roots), Resume::Missing);
        // Ids that are not hex cannot walk the store.
        assert_eq!(find_session_file(&roots, "../../etc"), None);
    }

    #[test]
    fn session_roots_follow_pi_precedence() {
        let env = BTreeMap::from([("PI_CODING_AGENT_SESSION_DIR".to_owned(), "/env".to_owned())]);
        let settings = PiSettings {
            session_dir: Some("rel-sessions".into()),
            ..Default::default()
        };
        let cwd = Path::new("/work");
        let roots = session_roots(
            &args("--session-dir /flag"),
            &env,
            &settings,
            Path::new("/agent"),
            cwd,
        );
        assert_eq!(
            roots,
            [
                PathBuf::from("/flag"),
                PathBuf::from("/env"),
                cwd.join("rel-sessions"),
                PathBuf::from("/agent/sessions"),
            ]
        );
    }

    #[test]
    fn compact_commands_map_to_the_rpc_command() {
        assert_eq!(parse_compact_command("/compact"), Some(None));
        assert_eq!(parse_compact_command("  /compact  "), Some(None));
        assert_eq!(
            parse_compact_command("/compact keep the marker"),
            Some(Some("keep the marker".into()))
        );
        assert_eq!(parse_compact_command("/compactor"), None);
        assert_eq!(parse_compact_command("please /compact"), None);
        assert_eq!(parse_compact_command("hello"), None);
    }

    #[test]
    fn skill_references_move_to_pis_native_command_position() {
        let skills: std::collections::HashSet<String> =
            ["review".to_owned(), "plan".to_owned()].into();
        assert_eq!(
            expand_skill_references("check $review this $plan please", &skills),
            "/skill:review /skill:plan check this please"
        );
        assert_eq!(expand_skill_references("$review", &skills), "/skill:review");
        // Unknown skills, prices and mid-word dollars are left alone.
        assert_eq!(
            expand_skill_references("costs $5 or a$review $unknown", &skills),
            "costs $5 or a$review $unknown"
        );
        assert_eq!(
            expand_skill_references("$review $review x", &skills),
            "/skill:review x"
        );
    }
}
