//! Claude Code harness: spawns the installed `claude` CLI and speaks its
//! stream-json protocol directly — no adapter process in between. Resurrected
//! from the pre-ACP driver (see docs/research/harness.md) and modernized
//! against CLI 2.1.228.
//!
//! - stdout JSONL frames are normalized into [`AgentEvent`]s (init dedupe,
//!   subagent tagging, typed tool decoding, error-code mapping).
//! - PERMISSIONS ride the stdio control channel: `--permission-prompt-tool
//!   stdio` (undocumented — absent from `claude --help`, but it is the same
//!   transport the Claude Agent SDK's `query()` drives, and was re-validated
//!   live against 2.1.228: `can_use_tool` control requests arrive and
//!   allow/deny responses are honored). The alternative channel — an MCP
//!   permission tool — needs a server process and was rejected. Native mode
//!   flags enforce policy; prompts use [`RunControls::request_permission`].
//!   `AskUserQuestion` separately uses [`RunControls::request_input`].
//! - DONE is the CLI's own `result` frame, eagerly: background work (a
//!   spawned subagent) never holds the turn. The CLI natively runs a second
//!   wake turn when a background task finishes — a fresh `init` (same
//!   session id, deduped) plus another `result` — and both are forwarded;
//!   the engine's parked-session resume path turns them into the
//!   done→Working→done wake.
//! - SUBAGENT frames arrive on the same stdout tagged with a top-level
//!   `parent_tool_use_id`; they are wrapped in [`AgentEvent::Subagent`] and
//!   NEVER folded into the parent feed (a background subagent interleaves
//!   with the parent's own stream — folding them in split contiguous text
//!   around phantom tool calls).
//! - Steering: queued [`SteerMessage`]s are written to stdin as user lines at
//!   any time; the CLI folds them into the running turn at its own step
//!   boundary.
//! - RECEIPTS: every stdin user line carries a host-chosen `uuid`, and the CLI
//!   runs with `--replay-user-messages`, which re-emits a user line on stdout
//!   (`isReplay`, same `uuid`) when it consumes it. That echo — not the pipe
//!   write/flush, which only proves the bytes left us — is the exact native
//!   receipt: it becomes [`AgentEvent::InputAcceptedFor`] for the mailbox
//!   message (or [`AgentEvent::InputAccepted`] for the initial prompt). The
//!   flush result still answers `notification_acceptance` (local delivery);
//!   an echo that never arrives stays unaccepted so the host's recovery owns
//!   it. Replay frames are never folded into the transcript.
//! - Interrupt: cancelling [`RunControls::interrupt`] sends the protocol-level
//!   interrupt control request, then escalates to SIGTERM and SIGKILL.

pub mod catalog;
mod fork;
mod normalize;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SlashCommand,
    SteeringMode, UserInputAnswer, UserInputQuestion,
};

use crate::process::{Child, ChildStdin, Command, Stdio};
use crate::{Harness, HarnessError, RunControls, Signal, send_signal, shutdown_child};
use catalog::{apply_ultrathink, static_models, to_effort};
use normalize::Normalizer;
use wire::{ControlRequestFrame, Frame, allow_response, control_response_line};

/// Locate the device's installed Claude Code CLI: our own PATH, then the
/// login-shell PATH snapshot (the user's shell init shapes PATH in ways a
/// GUI/service launch never sees — see [`crate::shell_env`]), then known
/// install locations as a last resort. The `CLAUDE_CODE_EXECUTABLE` override
/// is applied by [`ClaudeHarness::resolve_executable`], so availability and
/// launches agree on one resolution order. Resolved per call — cheap after
/// the snapshot is cached.
fn resolve_claude_executable() -> Option<PathBuf> {
    let mut extra = Vec::new();
    if let Some(home) = crate::executable::home_dir() {
        extra.push(home.join(".claude").join("local").join("claude"));
        extra.push(home.join(".local").join("bin").join("claude"));
    }
    extra.push(PathBuf::from("/opt/homebrew/bin/claude"));
    extra.push(PathBuf::from("/usr/local/bin/claude"));
    crate::executable::find_on_paths("claude", extra)
}

fn option_is_on(options: &serde_json::Map<String, Value>, key: &str) -> bool {
    match options.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "on" || s == "true",
        _ => false,
    }
}

/// The Claude Code harness. Construct with [`ClaudeHarness::new`]; tests point
/// it at a fake CLI with [`ClaudeHarness::with_executable`].
pub struct ClaudeHarness {
    launch: crate::instance::InstanceLaunch,
    executable: Option<PathBuf>,
    /// Grace between the interrupt control request and SIGTERM.
    interrupt_grace: Duration,
    /// Grace between SIGTERM and SIGKILL.
    kill_grace: Duration,
    /// Command discovery cache: only a successful probe is cached, so a
    /// broken CLI retries on the next picker open (ACP-harness parity).
    commands: tokio::sync::OnceCell<Vec<SlashCommand>>,
}

impl Default for ClaudeHarness {
    fn default() -> Self {
        Self {
            launch: Default::default(),
            executable: None,
            interrupt_grace: Duration::from_secs(2),
            kill_grace: Duration::from_secs(3),
            commands: tokio::sync::OnceCell::new(),
        }
    }
}

impl ClaudeHarness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_instance_launch(mut self, launch: crate::instance::InstanceLaunch) -> Self {
        self.launch = launch;
        self
    }

    /// Use a fixed CLI binary instead of PATH/known-location resolution.
    pub fn with_executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Tune the interrupt→SIGTERM→SIGKILL escalation timing.
    pub fn with_graces(mut self, interrupt_grace: Duration, kill_grace: Duration) -> Self {
        self.interrupt_grace = interrupt_grace;
        self.kill_grace = kill_grace;
        self
    }

    fn resolve_executable(&self) -> Result<PathBuf, HarnessError> {
        if let Some(p) = &self.executable {
            return crate::executable::validate_native_override(p);
        }
        if let Some(p) = std::env::var_os("CLAUDE_CODE_EXECUTABLE")
            && !p.is_empty()
        {
            return crate::executable::validate_native_override(&PathBuf::from(p));
        }
        resolve_claude_executable().ok_or_else(|| {
            HarnessError::NotInstalled(
                "claude (searched PATH, the login shell's PATH, ~/.claude/local, \
                 ~/.local/bin, /opt/homebrew/bin, /usr/local/bin, and \
                 fnm/nvm/volta/pnpm/bun install dirs; Windows also checks USERPROFILE \
                 and explicit NVM_SYMLINK/VOLTA_HOME/PNPM_HOME; set \
                 CLAUDE_CODE_EXECUTABLE to override)"
                    .into(),
            )
        })
    }

    /// The CLI's config root as the child will see it, including login-shell
    /// values in packaged GUI builds, not only the engine's environment.
    fn config_root(&self, exe: &PathBuf) -> PathBuf {
        let mut cmd = Command::new(exe);
        crate::compose_child_environment(&mut cmd, exe);
        self.launch.apply_launch(&mut cmd);
        #[cfg(not(windows))]
        let command_env = cmd.as_std().get_envs();
        #[cfg(windows)]
        let command_env = cmd.as_std_mut().get_envs();
        command_env
            .filter(|(key, _)| *key == "CLAUDE_CONFIG_DIR")
            .find_map(|(_, value)| value.filter(|value| !value.is_empty()))
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("CLAUDE_CONFIG_DIR")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| crate::executable::home_or_current_dir().join(".claude"))
    }

    fn build_command(&self, exe: &PathBuf, request: &RunRequest, resume: &ResumeLaunch) -> Command {
        let mut cmd = Command::new(exe);
        crate::compose_child_environment(&mut cmd, exe);
        self.launch.apply_launch(&mut cmd);
        cmd.args([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            // Required by the CLI alongside `-p --output-format stream-json`.
            "--verbose",
            "--include-partial-messages",
            // Echo each consumed stdin user line (same `uuid`) as the native
            // input receipt.
            "--replay-user-messages",
            // Newer Claude models emit no readable thinking text unless a
            // summary is asked for (raw reasoning stays provider-private).
            "--thinking-display",
            "summarized",
            // Route permission prompts to the stdio control channel so
            // `can_use_tool` (and AskUserQuestion in particular) reaches us.
            // Undocumented flag; validated live against 2.1.228.
            "--permission-prompt-tool",
            "stdio",
        ]);
        // The 1M context window is selected via a model-id suffix
        // (`sonnet[1m]`), exactly how the CLI itself does it; fast mode and
        // always-on thinking are settings overrides.
        if let Some(model) = &request.model {
            let one_m = request
                .model_options
                .get("contextWindow")
                .and_then(Value::as_str)
                == Some("1m");
            cmd.arg("--model");
            cmd.arg(if one_m {
                format!("{model}[1m]")
            } else {
                model.clone()
            });
        }
        if let Some(effort) = to_effort(request.reasoning, request.model.as_deref()) {
            cmd.args(["--effort", effort]);
        }
        let policy = crate::policy::compile(
            HarnessId::ClaudeCode,
            request.runtime_mode,
            request.interaction_mode,
        )
        .expect("Claude supports all runtime/interaction modes");
        cmd.args(["--permission-mode", policy.claude_permission_mode]);
        if policy.claude_permission_mode == "bypassPermissions" {
            cmd.arg("--dangerously-skip-permissions");
        }
        match resume {
            ResumeLaunch::None => {}
            ResumeLaunch::Session(session) => {
                cmd.arg(format!("--resume={session}"));
            }
            ResumeLaunch::Fork(token) => {
                // The child id is ours, so a start that dies before `init`
                // resolves to the same session instead of forking again.
                cmd.arg(format!("--resume={}", token.parent));
                cmd.args(["--fork-session", "--session-id", &token.child]);
                if let Some(at) = &token.at {
                    cmd.args(["--resume-session-at", at]);
                }
            }
        }
        let mut settings = serde_json::Map::new();
        if option_is_on(&request.model_options, "fastMode") {
            settings.insert("fastMode".into(), Value::Bool(true));
        }
        if option_is_on(&request.model_options, "thinking") {
            settings.insert("alwaysThinkingEnabled".into(), Value::Bool(true));
        }
        if request.reasoning == Some(ReasoningLevel::Ultracode) {
            settings.insert("ultracode".into(), Value::Bool(true));
        }
        if !settings.is_empty() {
            cmd.arg("--settings");
            cmd.arg(Value::Object(settings).to_string());
        }
        if !request.cwd.is_empty() {
            cmd.current_dir(&request.cwd);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd
    }

    /// Short-lived discovery probe: spawn the CLI in stream-json mode, send
    /// the `initialize` control request, and read the commands out of its
    /// control_response. No user message is ever written, so no turn (and no
    /// API call) happens; the child is torn down as soon as the response
    /// lands.
    async fn discover_commands(&self) -> Result<Vec<SlashCommand>, HarnessError> {
        let exe = self.resolve_executable()?;
        let mut cmd = Command::new(&exe);
        crate::compose_child_environment(&mut cmd, &exe);
        self.launch.apply_launch(&mut cmd);
        cmd.args([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            // Mandatory with --print + stream-json output; without it the
            // CLI exits immediately with a usage error.
            "--verbose",
        ]);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                HarnessError::NotInstalled(exe.display().to_string())
            } else {
                HarnessError::Io(e)
            }
        })?;
        let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            shutdown_child(&mut child, self.kill_grace).await;
            return Err(HarnessError::Protocol("claude child has no stdio".into()));
        };
        const PROBE_ID: &str = "zeron-command-probe";
        let discovery = async {
            let request = serde_json::json!({
                "type": "control_request",
                "request_id": PROBE_ID,
                "request": { "subtype": "initialize" },
            });
            stdin
                .write_all(format!("{request}\n").as_bytes())
                .await
                .map_err(HarnessError::Io)?;
            stdin.flush().await.map_err(HarnessError::Io)?;
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await.map_err(HarnessError::Io)? {
                let Ok(frame) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if frame.get("type").and_then(Value::as_str) != Some("control_response") {
                    continue;
                }
                let response = frame.get("response").cloned().unwrap_or(Value::Null);
                if response.get("request_id").and_then(Value::as_str) != Some(PROBE_ID) {
                    continue;
                }
                if response.get("subtype").and_then(Value::as_str) == Some("error") {
                    let msg = response
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("initialize control request failed");
                    return Err(HarnessError::Protocol(msg.into()));
                }
                return Ok(parse_initialize_commands(&response));
            }
            Err(HarnessError::Protocol(
                "claude exited before answering the initialize control request".into(),
            ))
        };
        let result = tokio::time::timeout(Duration::from_secs(10), discovery).await;
        shutdown_child(&mut child, self.kill_grace).await;
        match result {
            Ok(inner) => inner,
            Err(_) => Err(HarnessError::Protocol("command discovery timed out".into())),
        }
    }
}

/// `commands` out of an `initialize` control_response payload
/// (`response.response.commands`: name / description / argumentHint).
fn parse_initialize_commands(response: &Value) -> Vec<SlashCommand> {
    response
        .get("response")
        .and_then(|r| r.get("commands"))
        .and_then(Value::as_array)
        .map(|a| a.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|c| {
            let name = c.get("name").and_then(Value::as_str)?.trim();
            if name.is_empty() {
                return None;
            }
            Some(SlashCommand {
                name: name.to_owned(),
                description: c
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                input_hint: c
                    .get("argumentHint")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|h| !h.is_empty())
                    .map(str::to_owned),
            })
        })
        .collect()
}

#[async_trait]
impl Harness for ClaudeHarness {
    fn session_lifecycle(&self) -> Option<&dyn crate::session_lifecycle::SessionLifecycle> {
        Some(self)
    }
    fn id(&self) -> HarnessId {
        HarnessId::ClaudeCode
    }
    fn display_name(&self) -> &str {
        "Claude Code"
    }
    fn supports_steering(&self) -> bool {
        true
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::StepBoundary
    }
    /// Receipts are the CLI's `--replay-user-messages` echo of the submitted
    /// `uuid`, so the local `Steered` boundary cannot retire a pending input.
    fn confirms_steered_inputs(&self) -> bool {
        true
    }
    fn model_context_window(&self, model: &str, options: &serde_json::Map<String, Value>) -> Option<u64> {
        catalog::declared_context_window(model, options)
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[
            ReasoningLevel::Low,
            ReasoningLevel::Medium,
            ReasoningLevel::High,
            ReasoningLevel::XHigh,
            ReasoningLevel::Max,
        ]
    }
    fn installed(&self) -> bool {
        // The launch resolver, not bare discovery: a valid CLAUDE_CODE_EXECUTABLE
        // (or a test `executable`) must report installed, and an invalid one
        // must not — availability and launches share one resolution.
        self.resolve_executable().is_ok()
    }
    /// Done is the CLI's own terminal frame, for wake turns too.
    fn deterministic_turn_end(&self) -> bool {
        true
    }

    /// The curated static catalog (see [`catalog`]); requires an installed CLI
    /// so an absent binary surfaces as [`HarnessError::NotInstalled`] here,
    /// like the discovery call would.
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        self.resolve_executable()?;
        Ok(static_models())
    }

    async fn authenticated(&self) -> Result<Option<bool>, HarnessError> {
        let executable = self.resolve_executable()?;
        let mut command = Command::new(&executable);
        crate::compose_child_environment(&mut command, &executable);
        self.launch.apply(&mut command);
        command
            .args(["auth", "status", "--json"])
            .kill_on_drop(true);
        let output = match tokio::time::timeout(Duration::from_secs(3), command.output()).await {
            Ok(Ok(output)) => output,
            _ => return Ok(None),
        };
        // Consume just the public readiness bit, never persist/log the output.
        Ok(serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .and_then(|status| status["loggedIn"].as_bool()))
    }

    /// Slash commands from the CLI's `initialize` control-request handshake —
    /// the same channel the Claude Agent SDK's `query()` opens. The response
    /// carries every command with description + argument hint and involves no
    /// model turn (verified live, 2.1.228: the control_response is the first
    /// stdout line, well before any API traffic). Cached on success.
    async fn commands(&self) -> Result<Vec<SlashCommand>, HarnessError> {
        self.commands
            .get_or_try_init(|| self.discover_commands())
            .await
            .cloned()
    }

    async fn run(
        &self,
        request: RunRequest,
        controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        self.run_with_mode(request, controls, false).await
    }

    async fn run_title(
        &self,
        mut request: RunRequest,
        controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        request.resume = None;
        request.worktree = None;
        request.attachments.clear();
        request.model_options.clear();
        request.auto_approve = false;
        request.runtime_mode = zeron_proto::RuntimeMode::ApprovalRequired;
        request.interaction_mode = zeron_proto::InteractionMode::Default;
        self.run_with_mode(request, controls, true).await
    }

    async fn run_source_control(
        &self,
        mut request: RunRequest,
        controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        request.resume = None;
        request.worktree = None;
        request.attachments.clear();
        request.model_options.clear();
        request
            .model_options
            .insert("_noches_source_control".into(), true.into());
        request.auto_approve = false;
        request.runtime_mode = zeron_proto::RuntimeMode::ApprovalRequired;
        request.interaction_mode = zeron_proto::InteractionMode::Default;
        self.run_with_mode(request, controls, true).await
    }
}

impl ClaudeHarness {
    async fn run_with_mode(
        &self,
        request: RunRequest,
        controls: RunControls,
        title_only: bool,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let mut controls = controls;
        if title_only {
            controls.mcp = Default::default();
            controls.browser = None;
        }
        let mcp_guard = controls.mcp.run_guard();
        controls.bind_browser()?;
        crate::policy::compile(
            HarnessId::ClaudeCode,
            request.runtime_mode,
            request.interaction_mode,
        )?;
        let exe = self.resolve_executable()?;
        let config_root = self.config_root(&exe);
        let resume = ResumeLaunch::resolve(&config_root, request.resume.as_deref())?;
        let mut cmd = self.build_command(&exe, &request, &resume);
        let mcp_config = controls.mcp.claude_config()?;
        if let Some(config) = &mcp_config {
            cmd.arg("--mcp-config").arg(config.path());
        }
        if !title_only {
            if !controls.mcp.allowed_tools().is_empty() {
                cmd.arg("--allowedTools")
                    .arg(controls.mcp.allowed_tools().join(","));
            }
            if !controls.mcp.instructions().is_empty() {
                cmd.arg("--append-system-prompt")
                    .arg(controls.mcp.instructions());
            }
        }
        if title_only {
            cmd.args([
                "--system-prompt",
                crate::restricted_text_instructions(&request),
                "--tools",
                "",
                "--strict-mcp-config",
                "--mcp-config",
                "{\"mcpServers\":{}}",
                "--setting-sources",
                "",
            ]);
        }
        // Spawn identity comes from the history the child continues: the
        // parent's, for a first fork run.
        let normalizer = match resume.history_session() {
            Some(session_id) => Normalizer::for_resume(&config_root, session_id).await,
            None => Normalizer::new(),
        };
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                HarnessError::NotInstalled(exe.display().to_string())
            } else {
                HarnessError::Io(e)
            }
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HarnessError::Protocol("claude child has no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HarnessError::Protocol("claude child has no stdout".into()))?;
        let stderr_tail = crate::StderrTail::default();
        stderr_tail.retain_mcp(&controls.mcp);
        if let Some(stderr) = child.stderr.take() {
            let tail = stderr_tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "zeron_harness::claude", stderr = %crate::redact::redact_output(&line));
                    tail.push(&line);
                }
            });
        }

        let (stdin_tx, stdin_rx) = mpsc::unbounded_channel::<StdinMsg>();
        tokio::spawn(stdin_writer(stdin, stdin_rx));

        // The initial prompt as the first stdin user line (streaming-input
        // mode). Ultrathink rides every user message — steers included.
        // Staged image attachments are inlined as base64 image content blocks
        // ahead of the text (verified against the real CLI); their path refs
        // also ride the prompt text, so a skipped/unreadable file degrades to
        // the old-app behavior (the agent opens the path with its Read tool).
        let images = load_image_blocks(&request.attachments).await;
        let first_uuid = new_input_uuid();
        let first = wire::user_message_line_with_images(
            &apply_ultrathink(request.reasoning, &request.prompt),
            &images,
            &first_uuid,
        );
        let _ = stdin_tx.send(StdinMsg::Line(first));
        // The prompt has no mailbox identity: its echo is the uncorrelated
        // root-input receipt.
        let mut pending_inputs = PendingInputs::default();
        pending_inputs.register(first_uuid, None);

        let (event_tx, event_rx) = crate::session_event_channel(&controls.mcp);
        let session = Session {
            normalizer,
            title_only,
            child,
            stdout_lines: BufReader::new(stdout).lines(),
            stdin_tx,
            event_tx,
            pending_inputs,
            controls,
            reasoning: request.reasoning,
            interrupt_grace: self.interrupt_grace,
            kill_grace: self.kill_grace,
            stderr_tail,
            _mcp_config: mcp_config,
        };
        tokio::spawn(async move {
            let _mcp_start_guard = mcp_guard;
            run_session(session).await;
        });

        Ok(futures::stream::unfold(event_rx, |mut rx| async move {
            rx.recv().await.map(|ev| (ev, rx))
        })
        .boxed())
    }
}

/// How a run attaches to native history.
enum ResumeLaunch {
    None,
    Session(String),
    /// First run of a lazy fork (see [`fork`]).
    Fork(fork::ForkToken),
}

impl ResumeLaunch {
    fn resolve(config_root: &std::path::Path, resume: Option<&str>) -> Result<Self, HarnessError> {
        let Some(resume) = resume else {
            return Ok(Self::None);
        };
        let Some(token) = fork::ForkToken::parse(resume) else {
            return Ok(Self::Session(resume.to_owned()));
        };
        match fork::plan(config_root, &token) {
            fork::ForkPlan::ResumeChild => Ok(Self::Session(token.child)),
            fork::ForkPlan::Fork => Ok(Self::Fork(token)),
            // Definite: nothing has been written for this fork, so the host
            // may rebuild from portable context rather than guess.
            fork::ForkPlan::Unavailable => Err(HarnessError::Protocol(
                "The Claude conversation this thread was forked from is no longer on this device."
                    .into(),
            )),
        }
    }

    /// The session whose history the process continues.
    fn history_session(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Session(id) => Some(id),
            Self::Fork(token) => Some(&token.parent),
        }
    }
}

#[async_trait]
impl crate::session_lifecycle::SessionLifecycle for ClaudeHarness {
    /// Turn refs are the CLI's assistant message uuids, which
    /// `--resume-session-at` cuts at.
    fn can_fork_from_turn(&self) -> bool {
        true
    }

    /// Only a recorded assistant message of a parent transcript in the child's
    /// own working directory can be forked: a moving head, a legacy turn with
    /// no uuid, or a parent that is gone all fall back to portable context.
    async fn can_fork_now(
        &self,
        request: &crate::session_lifecycle::NativeForkRequest,
    ) -> Result<bool, HarnessError> {
        let (Some(at), true) = (request.source_turn_id.clone(), request.rollback_turns.is_none())
        else {
            return Ok(false);
        };
        let config_root = self.config_root(&self.resolve_executable()?);
        let (parent, cwd) = (request.source_thread_id.clone(), request.cwd.clone());
        Ok(tokio::task::spawn_blocking(move || {
            fork::can_fork_at(&config_root, &parent, &at, &cwd)
        })
        .await
        .unwrap_or(false))
    }

    /// Forking only mints the child's identity: the CLI writes the child when
    /// its first turn runs, and that run expands the token. No process starts
    /// here, so there is no acceptance to lose; a retry reuses the token.
    async fn fork_thread(
        &self,
        request: crate::session_lifecycle::NativeForkRequest,
    ) -> Result<String, HarnessError> {
        if !self.can_fork_now(&request).await? {
            return Err(HarnessError::Protocol(
                "Cannot fork Claude here: the source turn is not in a local transcript for this directory.".into(),
            ));
        }
        fork::ForkToken::mint(&request.source_thread_id, request.source_turn_id.as_deref())
            .map(|token| token.encode())
            .ok_or_else(|| HarnessError::Protocol("Claude fork source id is malformed.".into()))
    }
}

#[derive(Debug)]
enum StdinMsg {
    Line(String),
    /// Adapter delivery acknowledgement belongs to the write/flush boundary,
    /// not the local writer mailbox. This is not a CLI prompt-echo receipt.
    Steer {
        line: String,
        receipt: tokio::sync::oneshot::Sender<bool>,
    },
    /// Close stdin (end of steering input): the CLI finishes the current turn
    /// and exits, which ends the run stream at stdout EOF.
    Close,
}

/// Anthropic's API caps inline images at 5MB of raw bytes; larger files stay
/// path refs only.
const MAX_INLINE_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

/// Media type for an inline image block — extension first, magic bytes as the
/// fallback (pasted screenshots may carry odd names). Only the API-supported
/// inline types map; anything else (svg/bmp/tiff/…) returns `None`.
fn image_media_type(path: &std::path::Path, bytes: &[u8]) -> Option<&'static str> {
    let by_ext = match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        _ => None,
    };
    by_ext.or(match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("image/webp"),
        _ => None,
    })
}

/// Load `RunRequest::attachments` into inline image blocks, best-effort: an
/// unreadable, oversized, or unsupported file is skipped — its path ref still
/// rides the prompt text — never fatal to the run.
async fn load_image_blocks(paths: &[String]) -> Vec<wire::ImageBlock> {
    use base64::Engine as _;
    let mut blocks = Vec::new();
    for path in paths {
        let bytes = match tokio::fs::read(path).await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!(target: "zeron_harness::claude", %path, error = %err, "attachment unreadable; path ref only");
                continue;
            }
        };
        if bytes.len() as u64 > MAX_INLINE_IMAGE_BYTES {
            tracing::debug!(target: "zeron_harness::claude", %path, "attachment over inline cap; path ref only");
            continue;
        }
        let Some(media_type) = image_media_type(std::path::Path::new(path), &bytes) else {
            tracing::debug!(target: "zeron_harness::claude", %path, "attachment not an inline-supported image; path ref only");
            continue;
        };
        blocks.push(wire::ImageBlock {
            media_type: media_type.to_string(),
            data: base64::engine::general_purpose::STANDARD.encode(&bytes),
        });
    }
    blocks
}

fn new_input_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Submitted stdin user lines awaiting their `--replay-user-messages` echo,
/// keyed by the host-chosen line `uuid`. A line's echo is consumed once: a
/// duplicate or unknown echo yields nothing, so it can never accept another
/// message.
#[derive(Default)]
struct PendingInputs(std::collections::HashMap<String, Option<String>>);

impl PendingInputs {
    fn register(&mut self, uuid: String, message_id: Option<String>) {
        self.0.insert(uuid, message_id);
    }

    fn accept(&mut self, uuid: &str) -> Option<AgentEvent> {
        self.0.remove(uuid).map(crate::input_accepted_event)
    }
}

/// Owns the child's stdin; a write failure (EPIPE after the child died) is
/// tolerated and logged.
async fn stdin_writer(mut stdin: ChildStdin, mut rx: mpsc::UnboundedReceiver<StdinMsg>) {
    while let Some(msg) = rx.recv().await {
        let (line, receipt) = match msg {
            StdinMsg::Line(line) => (line, None),
            StdinMsg::Steer { line, receipt } => (line, Some(receipt)),
            StdinMsg::Close => {
                let _ = stdin.shutdown().await;
                return;
            }
        };
        let write = async {
            stdin.write_all(line.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await
        }
        .await;
        if let Some(receipt) = receipt {
            let _ = receipt.send(write.is_ok());
        }
        if let Err(e) = write {
            tracing::debug!(target: "zeron_harness::claude", "stdin write failed (tolerated): {e}");
            return;
        }
    }
}

struct Session {
    normalizer: Normalizer,
    title_only: bool,
    child: Child,
    stdout_lines: tokio::io::Lines<BufReader<crate::process::ChildStdout>>,
    stdin_tx: mpsc::UnboundedSender<StdinMsg>,
    event_tx: mpsc::Sender<Result<AgentEvent, HarnessError>>,
    /// Submitted stdin user lines still waiting for their native echo.
    pending_inputs: PendingInputs,
    controls: RunControls,
    reasoning: Option<ReasoningLevel>,
    interrupt_grace: Duration,
    kill_grace: Duration,
    /// Rolling stderr tail for the crash message on an unexpected exit.
    stderr_tail: crate::StderrTail,
    _mcp_config: Option<tempfile::NamedTempFile>,
}

/// The per-run event loop: one task multiplexing stdout frames, the steering
/// mailbox, the interrupt token, and consumer liveness.
async fn run_session(session: Session) {
    let Session {
        normalizer: mut norm,
        title_only,
        mut child,
        mut stdout_lines,
        stdin_tx,
        event_tx,
        mut pending_inputs,
        controls,
        reasoning,
        interrupt_grace,
        kill_grace,
        stderr_tail,
        _mcp_config,
    } = session;
    let RunControls {
        mcp,
        request_permission,
        browser: _,
        request_input,
        mut steering,
        interrupt,
        computer_use_socket: _,
    } = controls;
    let _mcp_guard = mcp.run_guard();
    let request_input = Arc::new(request_input);
    let request_permission = Arc::new(request_permission);
    let permission_gate = crate::PermissionGate::default();
    let _permission_lifetime = interrupt.clone().drop_guard();

    let mut steering_open = true;
    let mut interrupted = false;
    let mut interrupt_sent = false;
    let mut any_done = false;
    let mut done_after_interrupt = false;
    let mut escalation: Option<tokio::task::JoinHandle<()>> = None;

    'main: loop {
        tokio::select! {
            line = stdout_lines.next_line() => match line {
                Ok(Some(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let frame = match wire::parse_frame(line) {
                        Ok(frame) => frame,
                        Err(e) => {
                            tracing::debug!(target: "zeron_harness::claude", "unparseable frame (skipped): {e}");
                            continue;
                        }
                    };
                    if let Frame::ControlRequest(req) = frame {
                        if title_only {
                            let line = control_response_line(&req.request_id, serde_json::json!({
                                "behavior": "deny", "message": "Tools are disabled for title generation"
                            }));
                            let _ = stdin_tx.send(StdinMsg::Line(line));
                        } else {
                            handle_control_request(req, &request_input, &request_permission, &permission_gate, &interrupt, &stdin_tx);
                        }
                        continue;
                    }
                    // A replayed stdin line is the CLI's consumption receipt for
                    // that exact submission, never conversation traffic.
                    if let Frame::User(user) = &frame
                        && user.is_replay
                    {
                        if let Some(receipt) = user
                            .uuid
                            .as_deref()
                            .and_then(|uuid| pending_inputs.accept(uuid))
                            && event_tx.send(Ok(receipt)).await.is_err()
                        {
                            break 'main;
                        }
                        continue;
                    }
                    for ev in norm.normalize(frame, interrupted) {
                        let is_done = matches!(ev, AgentEvent::Done { .. });
                        if event_tx.send(Ok(ev)).await.is_err() {
                            break 'main; // consumer gone — reap below
                        }
                        if is_done {
                            any_done = true;
                            if interrupted {
                                done_after_interrupt = true;
                                break 'main;
                            }
                        }
                    }
                }
                Ok(None) => break 'main, // stdout EOF: the CLI exited
                Err(e) => {
                    let _ = event_tx.send(Err(HarnessError::Io(e))).await;
                    break 'main;
                }
            },

            steer = steering.recv(), if steering_open && !interrupted => match steer {
                Some(msg) => {
                    let uuid = new_input_uuid();
                    // Same best-effort inlining as the first prompt: an
                    // unreadable image keeps its path ref in the text.
                    let images = load_image_blocks(&msg.attachments).await;
                    let line = wire::user_message_line_with_images(
                        &apply_ultrathink(reasoning, &msg.prompt),
                        &images,
                        &uuid,
                    );
                    let queued = if let Some(receipt) = msg.notification_acceptance {
                        match stdin_tx.send(StdinMsg::Steer { line, receipt }) {
                            Ok(()) => true,
                            Err(error) => {
                                if let StdinMsg::Steer { receipt, .. } = error.0 {
                                    let _ = receipt.send(false);
                                }
                                false
                            }
                        }
                    } else {
                        stdin_tx.send(StdinMsg::Line(line)).is_ok()
                    };
                    if !queued {continue 'main;}
                    // Only an identified mailbox message can be retired by its
                    // echo; an anonymous steer has nothing to acknowledge.
                    if let Some(message_id) = msg.message_id {
                        pending_inputs.register(uuid, Some(message_id));
                    }
                    // The CLI consumes the queued line at its own step
                    // boundary; rotate the assistant message id so post-steer
                    // output folds into a fresh message.
                    let (prev, next) = norm.rotate_for_steer();
                    let ev = AgentEvent::Steered {
                        assistant_message_id: Some(prev),
                        next_assistant_message_id: Some(next),
                    };
                    if event_tx.send(Ok(ev)).await.is_err() {
                        break 'main;
                    }
                }
                None => {
                    // Mailbox closed: end the input so the run can finish
                    // after the current turn.
                    steering_open = false;
                    let _ = stdin_tx.send(StdinMsg::Close);
                }
            },

            _ = interrupt.cancelled(), if !interrupt_sent => {
                interrupt_sent = true;
                interrupted = true;
                let _ = stdin_tx.send(StdinMsg::Line(wire::interrupt_request_line("int_1")));
                // Escalate if the CLI doesn't wind down within the grace
                // periods: SIGTERM (kills bash trees, runs SessionEnd hooks),
                // then SIGKILL. Aborted once the child is reaped.
                if let Some(pid) = crate::process::signal_target(&child) {
                    escalation = Some(tokio::spawn(async move {
                        tokio::time::sleep(interrupt_grace).await;
                        send_signal(&pid, Signal::Term);
                        tokio::time::sleep(kill_grace).await;
                        send_signal(&pid, Signal::Kill);
                    }));
                }
            },

            _ = event_tx.closed() => break 'main,
        }
    }

    // Terminal bookkeeping: never end the stream without a Done unless the
    // consumer already hung up.
    if !event_tx.is_closed() {
        if interrupted && !done_after_interrupt {
            let _ = event_tx
                .send(Ok(AgentEvent::Done {
                    status: DoneStatus::Interrupted,
                    result: None,
                    error: None,
                    session_id: norm.session_id.clone(),
                }))
                .await;
        } else if !interrupted && !any_done {
            let status = child.try_wait().ok().flatten();
            let _ = event_tx
                .send(Ok(AgentEvent::Done {
                    status: DoneStatus::Errored,
                    result: None,
                    error: Some(crate::crash_message("claude", status, &stderr_tail)),
                    session_id: norm.session_id.clone(),
                }))
                .await;
        }
    }

    shutdown_child(&mut child, kill_grace).await;
    if let Some(handle) = escalation {
        handle.abort();
    }
}

type RequestInputFn = Box<
    dyn Fn(Vec<UserInputQuestion>) -> tokio::sync::oneshot::Receiver<Vec<UserInputAnswer>>
        + Send
        + Sync,
>;

/// Serve one control request through the permission bridge, except for
/// `AskUserQuestion`, which uses the separate content-input bridge. Native
/// permission modes filter requests before this callback; anything reaching
/// it requires explicit consent. Subtasks keep the frame loop flowing, and
/// unsupported requests are denied rather than left unanswered.
fn handle_control_request(
    req: ControlRequestFrame,
    request_input: &Arc<RequestInputFn>,
    request_permission: &Arc<crate::RequestPermission>,
    permission_gate: &crate::PermissionGate,
    interrupt: &crate::CancellationToken,
    stdin_tx: &mpsc::UnboundedSender<StdinMsg>,
) {
    if req.request.subtype != "can_use_tool" {
        tracing::debug!(
            target: "zeron_harness::claude",
            subtype = %crate::redact::redact_registered(&req.request.subtype),
            "unhandled control_request subtype"
        );
        let _ = stdin_tx.send(StdinMsg::Line(control_response_line(
            &req.request_id,
            serde_json::json!({
                "behavior": "deny", "message": "Unsupported control request"
            }),
        )));
        return;
    }
    if req.request.tool_name == "ExitPlanMode" {
        // T3 captures the plan but never lets a permission answer switch the
        // interaction mode. Implementing it requires a later explicit turn.
        let _ = stdin_tx.send(StdinMsg::Line(control_response_line(
            &req.request_id,
            serde_json::json!({
                "behavior": "deny",
                "message": "Stay in plan mode and wait for an explicit implementation turn."
            }),
        )));
        return;
    }
    if req.request.tool_name != "AskUserQuestion" {
        // Native acceptEdits/Auto filters run before this callback. A prompt
        // that reaches us must ask, even in Full access (unexpected escalation).
        let request_permission = request_permission.clone();
        let permission_gate = permission_gate.clone();
        let stdin_tx = stdin_tx.clone();
        let interrupt = interrupt.clone();
        tokio::spawn(async move {
            let request = zeron_proto::PermissionRequest::standard(
                &req.request.tool_name,
                crate::permission_summary(&req.request.tool_name, &req.request.input),
                true,
            );
            let fingerprint = crate::permission_fingerprint(&req.request.input);
            let answer = permission_gate
                .ask(request, fingerprint, &request_permission, &interrupt)
                .await;
            let response = if matches!(
                answer.decision,
                zeron_proto::PermissionDecision::Accept
                    | zeron_proto::PermissionDecision::AcceptForSession
            ) {
                let mut response = allow_response(req.request.input.clone());
                if answer.decision == zeron_proto::PermissionDecision::AcceptForSession {
                    // Never apply a suggested global setMode or fabricate a
                    // wildcard tool rule. Without scoped native rules, the
                    // runtime-local exact-input grant above handles repeats.
                    let mut updates: Vec<Value> = req
                        .request
                        .permission_suggestions
                        .into_iter()
                        .filter(|update| {
                            update["type"] == "addRules"
                                && update["behavior"] == "allow"
                                && update["rules"].as_array().is_some_and(|rules| {
                                    !rules.is_empty()
                                        && rules.iter().all(|rule| {
                                            rule["toolName"].as_str()
                                                == Some(req.request.tool_name.as_str())
                                                && exact_session_rule(
                                                    &req.request.input,
                                                    &rule["ruleContent"],
                                                )
                                        })
                                })
                        })
                        .collect();
                    for update in &mut updates {
                        if let Some(update) = update.as_object_mut() {
                            update.insert("destination".into(), "session".into());
                        }
                    }
                    if !updates.is_empty() {
                        response["updatedPermissions"] = serde_json::json!(updates);
                    }
                }
                response
            } else {
                serde_json::json!({"behavior":"deny","message":"Permission declined"})
            };
            let _ = stdin_tx.send(StdinMsg::Line(control_response_line(
                &req.request_id,
                response,
            )));
        });
        return;
    }
    let request_input = Arc::clone(request_input);
    let stdin_tx = stdin_tx.clone();
    let interrupt = interrupt.clone();
    tokio::spawn(async move {
        let request_id = req.request_id;
        let input = req.request.input;
        let questions = parse_questions(&input);
        // The engine's input bridge is the SOLE emitter of
        // `InputRequested`/`InputResolved`: it mints the request id, parks the
        // resolver for `respond_input`, and surfaces both events. Emitting our
        // own copy here (keyed by Claude's control-request id) folded a SECOND
        // input part into the doc whose id no resolver knew — the QuestionPanel
        // answered that unanswerable twin and the run never resumed.
        //
        // A dropped sender (caller went away) degrades to empty answers so the
        // agent is unblocked rather than wedged.
        let answers = tokio::select! {
            answer = (request_input)(questions.clone()) => answer.unwrap_or_default(),
            _ = interrupt.cancelled() => Vec::new(),
        };
        if answers.is_empty() {
            let _ = stdin_tx.send(StdinMsg::Line(control_response_line(
                &request_id,
                serde_json::json!({"behavior":"deny","message":"Question cancelled"}),
            )));
            return;
        }
        let updated = updated_input_with_answers(&input, &questions, &answers);
        let line = control_response_line(&request_id, allow_response(updated));
        let _ = stdin_tx.send(StdinMsg::Line(line));
    });
}

/// Native suggestions are patterns, not necessarily this request's scope.
/// Forward only a literal exact command/path; cache everything else locally.
fn exact_session_rule(input: &Value, rule: &Value) -> bool {
    rule.as_str().is_some_and(|rule| {
        !rule.is_empty()
            && !rule.contains(['*', '?', '[', ']'])
            && ["command", "file_path", "path"]
                .into_iter()
                .any(|key| input.get(key).and_then(Value::as_str) == Some(rule))
    })
}

/// Parse Claude's `AskUserQuestion` tool input into [`UserInputQuestion`]s
/// (tolerant of `header`/`title`, `question`/`prompt`, string or object
/// options — option descriptions are dropped, the wire type carries labels).
fn parse_questions(input: &Value) -> Vec<UserInputQuestion> {
    let raw = input.get("questions").and_then(Value::as_array);
    raw.map(|a| a.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|q| {
            let field =
                |keys: [&str; 2]| keys.iter().find_map(|k| q.get(*k).and_then(Value::as_str));
            UserInputQuestion {
                id: uuid::Uuid::new_v4().to_string(),
                header: field(["header", "title"]).unwrap_or("Question").into(),
                question: field(["question", "prompt"]).unwrap_or("").into(),
                multi_select: ["multiSelect", "multi_select"]
                    .iter()
                    .find_map(|k| q.get(*k).and_then(Value::as_bool))
                    .unwrap_or(false),
                options: q
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| a.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .map(|op| match op {
                        Value::String(s) => s.clone(),
                        other => other
                            .get("label")
                            .or_else(|| other.get("value"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Merge the user's answers back into the tool input, keyed by question text
/// (single-select ⇒ a string, multi-select ⇒ an array), as the tool expects.
fn updated_input_with_answers(
    input: &Value,
    questions: &[UserInputQuestion],
    answers: &[UserInputAnswer],
) -> Value {
    let mut updated = match input {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    let mut by_question = serde_json::Map::new();
    for q in questions {
        let labels: Vec<String> = answers
            .iter()
            .find(|a| a.question_id == q.id)
            .map(|a| a.labels.clone())
            .unwrap_or_default();
        let value = if q.multi_select {
            Value::Array(labels.into_iter().map(Value::String).collect())
        } else {
            Value::String(labels.into_iter().next().unwrap_or_default())
        };
        by_question.insert(q.question.clone(), value);
    }
    updated.insert("answers".into(), Value::Object(by_question));
    Value::Object(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(unix)]
    #[tokio::test]
    async fn steering_writer_acknowledges_a_flushed_line_not_its_mailbox() {
        let mut child = tokio::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("read -r line; test \"$line\" = user-line")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        let (receipt, mut accepted) = tokio::sync::oneshot::channel();
        tx.send(StdinMsg::Steer {
            line: "user-line".into(),
            receipt,
        })
        .unwrap();
        assert!(matches!(
            accepted.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        let writer = tokio::spawn(stdin_writer(child.stdin.take().unwrap(), rx));
        assert!(
            tokio::time::timeout(Duration::from_secs(2), accepted)
                .await
                .unwrap()
                .unwrap()
        );
        assert!(child.wait().await.unwrap().success());
        drop(tx);
        writer.await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn steering_writer_refuses_a_closed_provider_pipe() {
        let mut child = tokio::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        child.wait().await.unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        let (receipt, accepted) = tokio::sync::oneshot::channel();
        tx.send(StdinMsg::Steer {
            line: "not-delivered".into(),
            receipt,
        })
        .unwrap();
        let writer = tokio::spawn(stdin_writer(stdin, rx));
        assert!(
            !tokio::time::timeout(Duration::from_secs(2), accepted)
                .await
                .unwrap()
                .unwrap()
        );
        writer.await.unwrap();
    }

    #[test]
    fn declared_context_window_follows_the_catalog_option() {
        let harness = ClaudeHarness::new();
        let mut options = serde_json::Map::new();
        assert_eq!(
            harness.model_context_window("claude-opus-5-5", &options),
            Some(200_000)
        );
        options.insert("contextWindow".into(), "1m".into());
        assert_eq!(
            harness.model_context_window("claude-opus-5-5", &options),
            Some(1_000_000)
        );
        // No selectable window: the option cannot widen it. Custom IDs are
        // never guessed.
        assert_eq!(
            harness.model_context_window("claude-opus-4-8", &options),
            Some(200_000)
        );
        assert_eq!(harness.model_context_window("my-proxy-model", &options), None);
    }

    #[test]
    fn session_rules_never_expand_exact_consent_to_a_pattern() {
        let input = json!({"command":"echo safe"});
        assert!(exact_session_rule(&input, &json!("echo safe")));
        for rule in ["*", "echo *", "echo safe:*", "other command", ""] {
            assert!(!exact_session_rule(&input, &json!(rule)));
        }
        assert!(!exact_session_rule(
            &json!({"command":"echo *"}),
            &json!("echo *")
        ));
        assert!(exact_session_rule(
            &json!({"file_path":"/tmp/a.rs"}),
            &json!("/tmp/a.rs")
        ));
    }

    #[test]
    fn parses_questions_tolerantly() {
        let input = json!({
            "questions": [
                {
                    "header": "Choice",
                    "question": "Pick one",
                    "options": ["A", {"label": "B", "description": "second"}],
                    "multiSelect": false
                },
                { "title": "Alt", "prompt": "Pick many", "multi_select": true }
            ]
        });
        let qs = parse_questions(&input);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].header, "Choice");
        assert_eq!(qs[0].options, vec!["A".to_string(), "B".to_string()]);
        assert!(!qs[0].multi_select);
        assert_eq!(qs[1].header, "Alt");
        assert_eq!(qs[1].question, "Pick many");
        assert!(qs[1].multi_select);
    }

    #[test]
    fn answers_key_by_question_text() {
        let input =
            json!({"questions": [{"header": "H", "question": "Pick one", "options": ["A", "B"]}]});
        let qs = parse_questions(&input);
        let answers = vec![UserInputAnswer {
            question_id: qs[0].id.clone(),
            labels: vec!["B".into()],
        }];
        let updated = updated_input_with_answers(&input, &qs, &answers);
        assert_eq!(updated["answers"]["Pick one"], json!("B"));
        // Original input is preserved alongside the answers.
        assert!(updated["questions"].is_array());
    }
}
