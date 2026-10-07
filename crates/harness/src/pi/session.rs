//! One Pi run: setup, the event/response loop, steering, interrupt, settle.
//!
//! Turn lifecycle: `agent_settled` is the only terminal signal. `agent_end`
//! merely closes one low-level run - retries, compaction recovery and queued
//! steers may still follow - so a turn stays open until Pi reports the session
//! settled, and an idle probe confirms it before `Done` (an extension can start
//! detached compaction as that signal unwinds).
//!
//! Receipts: every `prompt`/`compact` is written with an id and its response
//! is re-injected into the ordered event stream (see `PiRpc::submit_ordered`),
//! so a steer's `disposition` is observed at exactly the position Pi wrote it
//! relative to the user message it starts. `InputAcceptedFor` is emitted only
//! from that native response.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use zeron_proto::{
    AgentEvent, ContextUsage, DoneStatus, HarnessId, PermissionDecision, PermissionRequest,
    RunRequest, RuntimeMode, SessionTokenTotals, SlashCommand, UserInputAnswer, UserInputQuestion,
};

use super::launch::{self, PiSettings, Resume};
use super::lifecycle::{self, Leaf};
use super::rpc::PiRpc;
use super::{Spawned, catalog, normalize};
use crate::mcp::SessionMcpContext;
use crate::process::Command;
use crate::{
    CancellationToken, HarnessError, PermissionGate, RequestPermission, RunControls, SteerMessage,
    permission_fingerprint, permission_summary,
};

const SETUP_TIMEOUT: Duration = Duration::from_secs(120);
const QUICK_TIMEOUT: Duration = Duration::from_secs(5);
const TREE_TIMEOUT: Duration = Duration::from_secs(10);
const SETTLE_PROBE_ATTEMPTS: u32 = 3;

type RequestInputFn = Box<
    dyn Fn(Vec<UserInputQuestion>) -> oneshot::Receiver<Vec<UserInputAnswer>> + Send + Sync,
>;
type Sender = mpsc::Sender<Result<AgentEvent, HarnessError>>;

// ---------------------------------------------------------------------------
// Private extensions and the child's environment
// ---------------------------------------------------------------------------

/// Noches' private Pi extensions for one run, written to a private directory
/// that lives as long as the run.
pub(super) struct Extensions {
    _dir: tempfile::TempDir,
    cua: PathBuf,
    policy: PathBuf,
    mcp: Option<PathBuf>,
    mode: &'static str,
}

fn write_private(path: &Path, contents: &str) -> Result<(), HarnessError> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(contents.as_bytes())?;
    Ok(())
}

fn mode_name(runtime: RuntimeMode) -> &'static str {
    match runtime {
        RuntimeMode::ApprovalRequired => "approval-required",
        RuntimeMode::AutoAcceptEdits => "auto-accept-edits",
        // `Auto` has no Pi classifier; policy::compile refuses it before here.
        RuntimeMode::Auto | RuntimeMode::FullAccess => "full-access",
    }
}

impl Extensions {
    pub(super) fn provision(
        controls: &RunControls,
        runtime: RuntimeMode,
    ) -> Result<Self, HarnessError> {
        let mut builder = tempfile::Builder::new();
        builder.prefix("noches-pi-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let dir = builder.tempdir().map_err(HarnessError::Io)?;
        let cua = dir.path().join("noches-cua.ts");
        write_private(&cua, super::CUA_EXTENSION)?;
        let policy = dir.path().join("noches-policy.ts");
        write_private(&policy, super::POLICY_EXTENSION)?;
        let mcp = if controls.mcp.entries().is_empty() && controls.mcp.instructions().is_empty() {
            None
        } else {
            Some(controls.mcp.pi_extension(dir.path())?)
        };
        Ok(Self {
            _dir: dir,
            cua,
            policy,
            mcp,
            mode: mode_name(runtime),
        })
    }

    /// `-e` flags. Explicit extension paths still load under a user's
    /// `--no-extensions`, so Noches' own cannot be disabled by launch args.
    pub(super) fn args(&self) -> Vec<String> {
        [Some(&self.cua), Some(&self.policy), self.mcp.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|path| ["-e".to_owned(), path.display().to_string()])
            .collect()
    }

    /// Bindings are per run: inherited values from a parent session are never
    /// reused, and server credentials exist only in this child's environment.
    pub(super) fn apply_env(&self, command: &mut Command, controls: &RunControls) {
        for key in [
            crate::mcp::ACP_EXECUTABLE_ENV,
            crate::mcp::MCP_ENTRIES_ENV,
            crate::mcp::MCP_INSTRUCTIONS_ENV,
            crate::mcp::MCP_ALLOWED_TOOLS_ENV,
            "NOCHES_CUA_SOCKET",
            super::RUNTIME_MODE_ENV,
        ] {
            command.env_remove(key);
        }
        command.env(super::RUNTIME_MODE_ENV, self.mode);
        if self.mcp.is_some() {
            for (key, value) in controls.mcp.process_environment() {
                command.env(key, value);
            }
        }
        if let Some(socket) = &controls.computer_use_socket {
            command.env("NOCHES_CUA_SOCKET", socket);
        }
    }
}

// ---------------------------------------------------------------------------
// Slash commands
// ---------------------------------------------------------------------------

/// `get_commands` entries as composer commands, behind the TUI built-ins that
/// Pi's RPC `get_commands` omits but this driver maps to RPC calls.
pub(super) fn commands_from(data: &Value) -> Vec<SlashCommand> {
    let mut commands = vec![SlashCommand {
        name: "compact".into(),
        description: "Summarize the conversation and reduce context usage".into(),
        input_hint: Some("Optional instructions".into()),
    }];
    commands.extend(launch::MAPPED_COMMANDS.iter().map(|(name, description, hint)| {
        SlashCommand {
            name: (*name).into(),
            description: (*description).into(),
            input_hint: hint.map(str::to_owned),
        }
    }));
    for entry in data
        .get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(name) = entry
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.is_empty() && !commands.iter().any(|c| c.name == *n))
        else {
            continue;
        };
        commands.push(SlashCommand {
            name: name.to_owned(),
            description: entry
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            input_hint: None,
        });
    }
    commands
}

/// Every command name Pi itself reported (skills, templates, extensions).
fn command_names(data: &Value) -> HashSet<String> {
    data.get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn skill_names(data: &Value) -> HashSet<String> {
    data.get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| entry.get("source").and_then(Value::as_str) == Some("skill"))
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .map(|name| name.strip_prefix("skill:").unwrap_or(name).to_owned())
        .collect()
}

// ---------------------------------------------------------------------------
// Extension dialogs
// ---------------------------------------------------------------------------

/// Answers `extension_ui_request` dialogs. Each dialog is answered by its own
/// task so a question left open never stalls the event loop.
#[derive(Clone)]
struct Ui {
    rpc: PiRpc,
    request_permission: Arc<RequestPermission>,
    request_input: Arc<RequestInputFn>,
    gate: PermissionGate,
    interrupt: CancellationToken,
}

/// The runtime-policy extension's confirmation: `{noches: "permission", tool,
/// input}` in the dialog message.
fn permission_payload(record: &Value) -> Option<(String, Value)> {
    let payload: Value = serde_json::from_str(record.get("message")?.as_str()?).ok()?;
    (payload.get("noches").and_then(Value::as_str) == Some("permission")).then(|| {
        (
            payload
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_owned(),
            payload.get("input").cloned().unwrap_or(Value::Null),
        )
    })
}

fn dialog_question(record: &Value, method: &str) -> UserInputQuestion {
    let title = record
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = |key: &str| record.get(key).and_then(Value::as_str).filter(|t| !t.is_empty());
    let mut question = text("message")
        .or_else(|| text("placeholder"))
        .unwrap_or(title)
        .to_owned();
    // The question contract has no prefill field; an editor's content has to
    // ride the text or the user would edit blind.
    if let Some(prefill) = text("prefill").filter(|_| method == "editor") {
        question.push_str("\n\nCurrent value:\n");
        question.extend(prefill.chars().take(2000));
    }
    let options = match method {
        "select" => record
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        "confirm" => vec!["Yes".into(), "No".into()],
        // Free text: the composer's typed answer overrides picked labels.
        _ => Vec::new(),
    };
    UserInputQuestion {
        id: record
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        header: if title.is_empty() { "Pi".into() } else { title.to_owned() },
        question,
        options,
        multi_select: false,
    }
}

/// The `extension_ui_response` fields for a user's answer; `None` answers
/// (cancelled or never given) cancel the dialog.
fn dialog_response(method: &str, answer: Option<Vec<String>>) -> Value {
    let Some(labels) = answer else {
        return json!({"cancelled": true});
    };
    let first = labels.into_iter().next();
    match (method, first) {
        ("confirm", Some(label)) => json!({"confirmed": label == "Yes"}),
        (_, Some(value)) => json!({"value": value}),
        // A dialog submitted empty is an empty value, not a cancel.
        ("input" | "editor", None) => json!({"value": ""}),
        _ => json!({"cancelled": true}),
    }
}

impl Ui {
    fn handle(&self, record: &Value) {
        let Some(method) = record.get("method").and_then(Value::as_str) else {
            return;
        };
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            return;
        };
        if !matches!(method, "select" | "confirm" | "input" | "editor") {
            // notify/setStatus/setWidget/setTitle/set_editor_text are
            // terminal decoration with no Noches surface.
            return;
        }
        let ui = self.clone();
        let (id, method, record) = (id.to_owned(), method.to_owned(), record.clone());
        tokio::spawn(async move {
            let response = ui.answer(&method, &record).await;
            let mut message = json!({"type": "extension_ui_response", "id": id});
            if let (Some(target), Some(fields)) = (message.as_object_mut(), response.as_object()) {
                target.extend(fields.clone());
            }
            let _ = ui.rpc.send(message);
        });
    }

    async fn answer(&self, method: &str, record: &Value) -> Value {
        if method == "confirm"
            && let Some((tool, input)) = permission_payload(record)
        {
            let request = PermissionRequest::standard(&tool, permission_summary(&tool, &input), true);
            let option = self
                .gate
                .ask(
                    request,
                    permission_fingerprint(&input),
                    &self.request_permission,
                    &self.interrupt,
                )
                .await;
            return match option.decision {
                PermissionDecision::Accept
                | PermissionDecision::AcceptForSession
                | PermissionDecision::AcceptAlways => json!({"confirmed": true}),
                PermissionDecision::Decline => json!({"confirmed": false}),
                PermissionDecision::Cancel => json!({"cancelled": true}),
            };
        }
        let receiver = (self.request_input)(vec![dialog_question(record, method)]);
        let answer = tokio::select! {
            _ = self.interrupt.cancelled() => None,
            answer = receiver => answer.ok().and_then(|answers| {
                answers.into_iter().next().map(|answer| answer.labels)
            }),
        };
        dialog_response(method, answer)
    }
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

pub(super) struct RunnerInit {
    pub spawned: Spawned,
    pub event_tx: Sender,
    pub controls: RunControls,
    pub request: RunRequest,
    pub settings: PiSettings,
    pub resume: Resume,
    pub extensions: Extensions,
    pub windows: Arc<Mutex<HashMap<String, u64>>>,
    pub interrupt_grace: Duration,
    pub kill_grace: Duration,
}

/// What a response tag stands for.
enum Tag {
    /// The run's first input.
    Root,
    /// A steer submitted through the mailbox.
    Steer(Ticket),
    /// A manual compaction (root `/compact` or an idle steer).
    Compact { ticket: Option<Ticket> },
    /// A Pi terminal command mapped to one RPC call (`/name`, `/session`, ...).
    Builtin {
        command: launch::Builtin,
        ticket: Option<Ticket>,
    },
    /// `abort`: answered once Pi is idle.
    Abort,
    /// Responses nobody needs (`clear_queue`).
    Ignore,
}

struct Ticket {
    message_id: Option<String>,
    receipt: Option<oneshot::Sender<bool>>,
}

enum Flow {
    Continue,
    Stop,
}

/// Facts the setup phase learned.
struct Started {
    session_file: String,
    model: String,
    window: Option<u64>,
    skills: HashSet<String>,
    pi_commands: HashSet<String>,
    commands: Vec<SlashCommand>,
    leaf: Option<String>,
    leaf_trusted: bool,
}

pub(super) struct Runner {
    child: crate::acp::child::Child,
    rpc: PiRpc,
    events: mpsc::UnboundedReceiver<Value>,
    stderr_tail: crate::StderrTail,
    event_tx: Sender,
    request: RunRequest,
    settings: PiSettings,
    resume: Resume,
    _extensions: Extensions,
    windows: Arc<Mutex<HashMap<String, u64>>>,
    interrupt_grace: Duration,
    kill_grace: Duration,
    mcp: SessionMcpContext,
    ui: Ui,
    steering: mpsc::Receiver<SteerMessage>,
    interrupt: CancellationToken,

    // session
    session_file: String,
    model: String,
    window: Option<u64>,
    skills: HashSet<String>,
    /// Command names Pi reported; they reach Pi as typed.
    pi_commands: HashSet<String>,
    assistant_message_id: String,
    leaf_cursor: Option<String>,
    leaf_trusted: bool,

    // turn
    tags: HashMap<u64, Tag>,
    next_tag: u64,
    turn_open: bool,
    /// The root prompt's own user message has not been seen yet.
    expect_root_user: bool,
    /// Steers Pi accepted whose user message has not started yet.
    pending_deliveries: u32,
    turn_error: Option<String>,
    message_had_text: bool,
    /// Content blocks of the current assistant message that streamed deltas.
    streamed_blocks: HashSet<u64>,
    tool_calls: HashMap<String, (String, Value)>,
    compacting: bool,
    manual_compact: bool,
    settle_pending: bool,
    // mailbox / interrupt
    steering_open: bool,
    interrupted: bool,
    interrupt_sent: bool,
    done_after_interrupt: bool,
    escalation: Option<(tokio::time::Instant, bool)>,
}

fn new_message_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn rotate(id: &mut String) -> (String, String) {
    let next = new_message_id();
    (std::mem::replace(id, next.clone()), next)
}



impl Runner {
    pub(super) fn new(init: RunnerInit) -> Self {
        let RunnerInit {
            spawned,
            event_tx,
            controls,
            request,
            settings,
            resume,
            extensions,
            windows,
            interrupt_grace,
            kill_grace,
        } = init;
        let Spawned {
            child,
            rpc,
            events,
            stderr_tail,
        } = spawned;
        let RunControls {
            mcp,
            request_permission,
            browser: _,
            computer_use_socket: _,
            request_input,
            steering,
            interrupt,
        } = controls;
        let ui = Ui {
            rpc: rpc.clone(),
            request_permission: Arc::new(request_permission),
            request_input: Arc::new(request_input),
            gate: PermissionGate::default(),
            interrupt: interrupt.clone(),
        };
        Self {
            child,
            rpc,
            events,
            stderr_tail,
            event_tx,
            request,
            settings,
            resume,
            _extensions: extensions,
            windows,
            interrupt_grace,
            kill_grace,
            mcp,
            ui,
            steering,
            interrupt,
            session_file: String::new(),
            model: String::new(),
            window: None,
            skills: HashSet::new(),
            pi_commands: HashSet::new(),
            assistant_message_id: new_message_id(),
            leaf_cursor: None,
            leaf_trusted: false,
            tags: HashMap::new(),
            next_tag: 0,
            turn_open: false,
            expect_root_user: false,
            pending_deliveries: 0,
            turn_error: None,
            message_had_text: false,
            streamed_blocks: HashSet::new(),
            tool_calls: HashMap::new(),
            compacting: false,
            manual_compact: false,
            settle_pending: false,
            steering_open: true,
            interrupted: false,
            interrupt_sent: false,
            done_after_interrupt: false,
            escalation: None,
        }
    }

    async fn emit(&self, event: AgentEvent) -> bool {
        self.event_tx.send(Ok(event)).await.is_ok()
    }

    async fn fail(&mut self, error: String, session_id: Option<String>) {
        let _ = self
            .emit(AgentEvent::Done {
                status: DoneStatus::Errored,
                result: None,
                error: Some(error),
                session_id,
            })
            .await;
        self.child.shutdown(self.kill_grace).await;
    }

    pub(super) async fn run(mut self) {
        let _mcp_guard = self.mcp.run_guard();
        let _permission_lifetime = self.interrupt.clone().drop_guard();

        // Pi may raise dialogs while it starts (project trust, login); keep
        // answering them while setup waits on its own requests.
        let setup = Self::setup(self.rpc.clone(), self.request.clone(), self.settings.clone());
        tokio::pin!(setup);
        let started = loop {
            tokio::select! {
                biased;
                result = &mut setup => break result,
                Some(record) = self.events.recv() => {
                    if record.get("type").and_then(Value::as_str) == Some("extension_ui_request") {
                        self.ui.handle(&record);
                    }
                }
                _ = self.interrupt.cancelled() => {
                    let _ = self.emit(AgentEvent::Done {
                        status: DoneStatus::Interrupted,
                        result: None,
                        error: None,
                        session_id: None,
                    }).await;
                    self.child.shutdown(self.kill_grace).await;
                    return;
                }
            }
        };
        let started = match started {
            Ok(started) => started,
            Err(error) => {
                // Let the stderr reader drain the dying child's last words.
                let status = match self.child.try_wait().ok().flatten() {
                    Some(status) => Some(status),
                    None if matches!(error, HarnessError::Transport(_)) => {
                        tokio::time::timeout(Duration::from_millis(500), self.child.wait())
                            .await
                            .ok()
                            .and_then(Result::ok)
                    }
                    None => None,
                };
                if status.is_some() {
                    self.stderr_tail.wait_closed().await;
                }
                let message = match (&error, status) {
                    // A child that died before answering is better explained
                    // by its exit and stderr than by the transport symptom.
                    (HarnessError::Transport(_), Some(_)) => {
                        crate::crash_message("pi", status, &self.stderr_tail)
                    }
                    _ => error.to_string(),
                };
                self.fail(message, None).await;
                return;
            }
        };
        let commands = self.adopt(started);
        let lost_session = matches!(self.resume, Resume::Missing);
        if lost_session {
            tracing::warn!(
                target: "zeron_harness::pi",
                "stored Pi session is gone; starting a new session"
            );
        }
        if !self
            .emit(AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Pi,
                model: self.model.clone(),
                tools: Vec::new(),
                cwd: self.request.cwd.clone(),
                session_id: self.session_file.clone(),
                assistant_message_id: self.assistant_message_id.clone(),
            })
            .await
        {
            self.child.shutdown(self.kill_grace).await;
            return;
        }
        if lost_session
            && !self
                .emit(AgentEvent::Error {
                    message: "The earlier Pi session is no longer available on this device, so \
                              this chat continues in a new Pi session without its earlier context."
                        .into(),
                })
                .await
        {
            self.child.shutdown(self.kill_grace).await;
            return;
        }
        if !commands.is_empty() && !self.emit(AgentEvent::AvailableCommands { commands }).await {
            self.child.shutdown(self.kill_grace).await;
            return;
        }
        let (prompt, attachments) = (self.request.prompt.clone(), self.request.attachments.clone());
        if self.send_root(&prompt, &attachments).await {
            self.main_loop().await;
        }
        self.finish().await;
    }

    /// Select the model/thinking level the request names and read the session
    /// identity. Every request here is correlated, so dialogs raised while Pi
    /// processes them are answered by the caller's pump.
    async fn setup(
        rpc: PiRpc,
        request: RunRequest,
        settings: PiSettings,
    ) -> Result<Started, HarnessError> {
        let state = rpc
            .request(json!({"type": "get_state"}), SETUP_TIMEOUT)
            .await?;
        let session_file = state
            .get("sessionFile")
            .and_then(Value::as_str)
            .filter(|file| !file.is_empty())
            .ok_or_else(|| HarnessError::Protocol("Pi reported no session file".into()))?
            .to_owned();
        let mut model = state.get("model").cloned().filter(Value::is_object);
        let current_level = state
            .get("thinkingLevel")
            .and_then(Value::as_str)
            .map(str::to_owned);

        let explicit = request
            .model
            .as_deref()
            .filter(|m| !m.is_empty() && *m != catalog::DEFAULT_MODEL);
        // "Pi default" restores the configured model, which a resumed session
        // would otherwise replace with the one it last used.
        let wanted = match explicit {
            Some(slug) => Some(slug.to_owned()),
            None => settings
                .default_provider
                .as_ref()
                .zip(settings.default_model.as_ref())
                .map(|(provider, id)| format!("{provider}/{id}")),
        };
        if let Some(slug) = wanted
            && model.as_ref().and_then(catalog::slug).as_deref() != Some(slug.as_str())
        {
            match catalog::split_slug(&slug) {
                Some((provider, id)) => {
                    let selected = rpc
                        .request(
                            json!({"type": "set_model", "provider": provider, "modelId": id}),
                            QUICK_TIMEOUT * 6,
                        )
                        .await;
                    match selected {
                        Ok(selected) => model = Some(selected),
                        Err(error) if explicit.is_some() => {
                            return Err(HarnessError::Protocol(format!(
                                "Pi could not select model '{slug}': {error}"
                            )));
                        }
                        // The configured default may name a model that is no
                        // longer available; keep what Pi itself chose.
                        Err(_) => {}
                    }
                }
                None if explicit.is_some() => {
                    return Err(HarnessError::Protocol(format!(
                        "Pi model '{slug}' must use provider/model format"
                    )));
                }
                None => {}
            }
        }

        if let Some(level) = request.reasoning {
            let offered = model.as_ref().map(catalog::supported_levels).unwrap_or_default();
            if let Some(target) = catalog::clamp_level(level, &offered).and_then(catalog::level_name)
                && current_level.as_deref() != Some(target)
                && let Err(error) = rpc
                    .request(
                        json!({"type": "set_thinking_level", "level": target}),
                        QUICK_TIMEOUT,
                    )
                    .await
            {
                tracing::debug!(target: "zeron_harness::pi", "set_thinking_level failed: {error}");
            }
        }

        // Discovery can run extension code; a slow or failing lookup must not
        // block the session (skills simply stay unexpanded).
        let commands = rpc
            .request(json!({"type": "get_commands"}), QUICK_TIMEOUT)
            .await
            .unwrap_or(Value::Null);
        // The baseline cursor comes from the session file: a listing from Pi
        // would serialise the whole (possibly huge) session before the prompt.
        let baseline = leaf_of(&session_file).await;
        let slug = model.as_ref().and_then(catalog::slug);
        Ok(Started {
            session_file,
            model: slug
                .clone()
                .or(request.model.clone())
                .unwrap_or_else(|| catalog::DEFAULT_MODEL.into()),
            window: model.as_ref().and_then(catalog::context_window),
            skills: skill_names(&commands),
            pi_commands: command_names(&commands),
            commands: commands_from(&commands),
            leaf: match &baseline {
                Leaf::Entry(id) => Some(id.clone()),
                Leaf::Empty | Leaf::Unknown => None,
            },
            leaf_trusted: baseline != Leaf::Unknown,
        })
    }

    fn adopt(&mut self, started: Started) -> Vec<SlashCommand> {
        self.session_file = started.session_file;
        self.model = started.model;
        self.window = started.window;
        if let Some(window) = started.window {
            let mut windows = self
                .windows
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            windows.insert(self.model.clone(), window);
            if self.request.model.as_deref().is_none_or(|m| m == catalog::DEFAULT_MODEL) {
                windows.insert(catalog::DEFAULT_MODEL.into(), window);
            }
        }
        self.skills = started.skills;
        self.pi_commands = started.pi_commands;
        self.leaf_cursor = started.leaf;
        self.leaf_trusted = started.leaf_trusted;
        started.commands
    }

    // -- sending ------------------------------------------------------------

    /// Write a command whose response arrives, ordered, on the event stream.
    fn fire(&mut self, record: Value, tag: Tag) -> Result<(), HarnessError> {
        let number = self.next_tag;
        self.next_tag += 1;
        self.tags.insert(number, tag);
        self.rpc.submit_ordered(record, number).inspect_err(|_| {
            self.tags.remove(&number);
        })
    }

    /// A correlated request made from the main loop. Until an interrupt has
    /// been acted on it gives way to one (`None`), so a slow Pi cannot hold a
    /// Stop behind its probe timeouts; afterwards it is bounded tightly.
    async fn request_unless_interrupted(
        &self,
        record: Value,
        timeout: Duration,
    ) -> Option<Result<Value, HarnessError>> {
        let timeout = if self.interrupt_sent { timeout.min(QUICK_TIMEOUT) } else { timeout };
        tokio::select! {
            biased;
            _ = self.interrupt.cancelled(), if !self.interrupt_sent => None,
            result = self.rpc.request(record, timeout) => Some(result),
        }
    }

    /// Expand `$skill` references and inline image attachments.
    async fn payload(&self, text: &str, attachments: &[String]) -> (String, Vec<Value>) {
        let message = if text.contains('$') && !self.skills.is_empty() {
            launch::expand_skill_references(text, &self.skills)
        } else {
            text.to_owned()
        };
        (message, load_images(attachments).await)
    }

    async fn send_root(&mut self, prompt: &str, attachments: &[String]) -> bool {
        self.turn_open = true;
        let builtin = match launch::parse_builtin(prompt, &self.pi_commands) {
            Ok(builtin) => builtin,
            Err(refusal) => {
                self.fail_turn(refusal).await;
                return false;
            }
        };
        let sent = if let Some(instructions) = launch::parse_compact_command(prompt) {
            self.manual_compact = true;
            self.fire(compact_record(instructions), Tag::Compact { ticket: None })
        } else if let Some(command) = builtin {
            let record = command.record();
            self.fire(record, Tag::Builtin { command, ticket: None })
        } else {
            let (message, images) = self.payload(prompt, attachments).await;
            self.expect_root_user = true;
            let mut record = json!({"type": "prompt", "message": message});
            if !images.is_empty() {
                record["images"] = Value::Array(images);
            }
            self.fire(record, Tag::Root)
        };
        match sent {
            Ok(()) => true,
            Err(error) => {
                self.fail(error.to_string(), Some(self.session_file.clone()))
                    .await;
                false
            }
        }
    }

    async fn on_steer(&mut self, message: SteerMessage) -> Flow {
        let SteerMessage {
            prompt,
            message_id,
            attachments,
            notification_acceptance,
        } = message;
        let ticket = Ticket {
            message_id,
            receipt: notification_acceptance,
        };
        if let Some(instructions) = launch::parse_compact_command(&prompt) {
            // `compact` aborts the agent first; a running turn would be lost.
            if self.turn_open {
                ticket.reject();
                return if self
                    .emit(AgentEvent::Error {
                        message: "Pi cannot compact while it is working; send /compact when the \
                                  turn has finished."
                            .into(),
                    })
                    .await
                {
                    Flow::Continue
                } else {
                    Flow::Stop
                };
            }
            self.turn_open = true;
            self.manual_compact = true;
            let _ = self.fire(
                compact_record(instructions),
                Tag::Compact {
                    ticket: Some(ticket),
                },
            );
            return Flow::Continue;
        }
        match launch::parse_builtin(&prompt, &self.pi_commands) {
            Ok(None) => {}
            Ok(Some(command)) => {
                if self.turn_open {
                    ticket.reject();
                    return self
                        .emit_flow(AgentEvent::Error {
                            message: "Pi terminal commands run between turns; send it again when \
                                      the turn has finished."
                                .into(),
                        })
                        .await;
                }
                self.turn_open = true;
                let record = command.record();
                let _ = self.fire(
                    record,
                    Tag::Builtin {
                        command,
                        ticket: Some(ticket),
                    },
                );
                return Flow::Continue;
            }
            Err(refusal) => {
                ticket.reject();
                return self
                    .emit_flow(AgentEvent::Error { message: refusal })
                    .await;
            }
        }
        // `streamingBehavior: steer` is atomic on Pi's side: it queues during
        // a run and starts a new run if the turn settled first. A bare `steer`
        // sent after Pi went idle would stay queued forever.
        let (text, images) = self.payload(&prompt, &attachments).await;
        let mut record = json!({
            "type": "prompt",
            "message": text,
            "streamingBehavior": "steer",
        });
        if !images.is_empty() {
            record["images"] = Value::Array(images);
        }
        // A failed write drops the ticket: Pi may or may not have seen it, so
        // no receipt is claimed either way (the transport error follows).
        let _ = self.fire(record, Tag::Steer(ticket));
        Flow::Continue
    }

    // -- the loop -----------------------------------------------------------

    async fn main_loop(&mut self) {
        loop {
            let escalate_at = self
                .escalation
                .map(|(at, _)| at)
                .unwrap_or_else(|| tokio::time::Instant::now() + Duration::from_secs(3600));
            let flow = tokio::select! {
                record = self.events.recv() => match record {
                    Some(record) => self.on_record(record).await,
                    // stdout closed: Pi exited.
                    None => Flow::Stop,
                },
                steer = self.steering.recv(), if self.steering_open && !self.interrupted => {
                    match steer {
                        Some(message) => self.on_steer(message).await,
                        // The caller's graceful idle-reap: finish once nothing
                        // is in flight.
                        None => {
                            self.steering_open = false;
                            if self.turn_open { Flow::Continue } else { Flow::Stop }
                        }
                    }
                }
                _ = self.interrupt.cancelled(), if !self.interrupt_sent => self.begin_interrupt(),
                _ = tokio::time::sleep_until(escalate_at), if self.escalation.is_some() => {
                    self.escalate();
                    Flow::Continue
                }
                _ = self.event_tx.closed() => Flow::Stop,
            };
            if matches!(flow, Flow::Stop) {
                break;
            }
        }
    }

    fn begin_interrupt(&mut self) -> Flow {
        self.interrupt_sent = true;
        self.interrupted = true;
        if !self.turn_open {
            return Flow::Stop;
        }
        let now = tokio::time::Instant::now();
        if self.manual_compact || self.compacting {
            // Pi's abort does not cancel compaction: terminate.
            self.escalation = Some((now, false));
        } else {
            // `abort` continues queued messages that remain in the session, so
            // drop them first or a Stop would immediately start the next one.
            let _ = self.fire(json!({"type": "clear_queue"}), Tag::Ignore);
            let _ = self.fire(json!({"type": "abort"}), Tag::Abort);
            self.escalation = Some((now + self.interrupt_grace, false));
        }
        Flow::Continue
    }

    fn escalate(&mut self) {
        match self.escalation.take() {
            Some((_, false)) => {
                self.child.request_group_shutdown();
                self.escalation =
                    Some((tokio::time::Instant::now() + self.kill_grace, true));
            }
            Some((_, true)) => {
                self.child.terminate_group();
                let _ = self.child.start_kill();
            }
            None => {}
        }
    }

    async fn finish(&mut self) {
        if !self.event_tx.is_closed() {
            if self.interrupted && !self.done_after_interrupt {
                let _ = self
                    .emit(AgentEvent::Done {
                        status: DoneStatus::Interrupted,
                        result: None,
                        error: None,
                        session_id: Some(self.session_file.clone()),
                    })
                    .await;
            } else if !self.interrupted && self.turn_open {
                // A child killed mid-turn must not read as a silent success.
                self.stderr_tail.wait_closed().await;
                let status = tokio::time::timeout(Duration::from_millis(500), self.child.wait())
                    .await
                    .ok()
                    .and_then(Result::ok);
                let _ = self
                    .emit(AgentEvent::Done {
                        status: DoneStatus::Errored,
                        result: None,
                        error: Some(crate::crash_message("pi", status, &self.stderr_tail)),
                        session_id: Some(self.session_file.clone()),
                    })
                    .await;
            }
        }
        self.child.shutdown(self.kill_grace).await;
    }

    // -- events -------------------------------------------------------------

    async fn on_record(&mut self, record: Value) -> Flow {
        let kind = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        match kind.as_str() {
            "response" => match record.get("tag").and_then(Value::as_u64) {
                Some(tag) => self.on_response(tag, record).await,
                None => Flow::Continue,
            },
            "agent_start" => {
                self.turn_open = true;
                self.settle_pending = false;
                Flow::Continue
            }
            "message_start" => self.on_message_start(&record).await,
            "message_update" => self.on_message_update(&record).await,
            "message_end" => self.on_message_end(&record).await,
            "tool_execution_start" => self.on_tool_start(&record).await,
            "tool_execution_end" => self.on_tool_end(&record).await,
            "compaction_start" => {
                self.compacting = true;
                Flow::Continue
            }
            "compaction_end" => self.on_compaction_end(&record).await,
            "auto_retry_start" => {
                let number = |key: &str| record.get(key).and_then(Value::as_u64).unwrap_or(0);
                let reason = record
                    .get("errorMessage")
                    .and_then(Value::as_str)
                    .unwrap_or("a provider error");
                self.emit_flow(AgentEvent::Error {
                    message: format!(
                        "Pi is retrying after an error (attempt {} of {}, in {}s): {reason}",
                        number("attempt"),
                        number("maxAttempts"),
                        number("delayMs").div_ceil(1000),
                    ),
                })
                .await
            }
            "auto_retry_end" => {
                if record.get("success").and_then(Value::as_bool) == Some(true) {
                    // Pi emits the erroring message_end before retrying, so a
                    // recovered retry must clear it or the turn would settle
                    // as failed.
                    self.turn_error = None;
                } else if !self.interrupted {
                    self.turn_error = Some(
                        record
                            .get("finalError")
                            .and_then(Value::as_str)
                            .unwrap_or("Pi auto-retry failed.")
                            .to_owned(),
                    );
                }
                Flow::Continue
            }
            "agent_settled" => self.on_settled().await,
            "extension_ui_request" => {
                self.ui.handle(&record);
                Flow::Continue
            }
            "extension_error" => {
                let name = record
                    .get("extensionPath")
                    .and_then(Value::as_str)
                    .and_then(|path| Path::new(path).file_stem())
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("extension");
                let during = record
                    .get("event")
                    .and_then(Value::as_str)
                    .map(|event| format!(" during {event}"))
                    .unwrap_or_default();
                let detail: String = record
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .chars()
                    .take(500)
                    .collect();
                if self
                    .emit(AgentEvent::Error {
                        message: format!("Pi extension {name} failed{during}. {detail}")
                            .trim()
                            .to_owned(),
                    })
                    .await
                {
                    Flow::Continue
                } else {
                    Flow::Stop
                }
            }
            // turn_start/turn_end, queue_update, entry_appended,
            // session_info_changed, thinking_level_changed, retries' scheduling,
            // bash updates: nothing to render.
            _ => Flow::Continue,
        }
    }

    async fn on_response(&mut self, tag: u64, record: Value) -> Flow {
        let ok = record.get("success").and_then(Value::as_bool) == Some(true);
        let data = record.get("data").cloned().unwrap_or(Value::Null);
        let error = record
            .get("error")
            .map(|e| match e {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| "Pi rejected the command".into());
        let disposition = data
            .get("disposition")
            .and_then(Value::as_str)
            .unwrap_or("started")
            .to_owned();
        let Some(kind) = self.tags.remove(&tag) else {
            return Flow::Continue;
        };
        match kind {
            Tag::Root => {
                if !ok {
                    return self.fail_turn(error).await;
                }
                if !self.emit(AgentEvent::InputAccepted).await {
                    return Flow::Stop;
                }
                if disposition == "handled" {
                    // An extension command consumed the prompt: no agent run
                    // starts, so no `agent_settled` is coming unless the
                    // extension began one.
                    self.settle_pending = true;
                    return self.probe_idle().await;
                }
                Flow::Continue
            }
            Tag::Steer(ticket) => {
                if !ok {
                    ticket.reject();
                    return self
                        .emit_flow(AgentEvent::Error {
                            message: format!("Steering failed: {error}"),
                        })
                        .await;
                }
                if disposition != "handled" {
                    // Pi will inject the message at its step boundary (or has
                    // just started a run with it); the user message that
                    // follows is the transcript boundary.
                    self.pending_deliveries += 1;
                }
                if disposition == "started" {
                    self.turn_open = true;
                    self.expect_root_user = false;
                }
                let message_id = ticket.message_id.clone();
                ticket.accept();
                self.emit_flow(crate::input_accepted_event(message_id)).await
            }
            Tag::Compact { ticket } => {
                self.manual_compact = false;
                if !ok {
                    if let Some(ticket) = ticket {
                        ticket.reject();
                    }
                    return self.fail_turn(error).await;
                }
                if !self.accept_local_input(ticket).await {
                    return Flow::Stop;
                }
                if !self
                    .emit(AgentEvent::TextDelta {
                        text: compaction_summary(&data),
                    })
                    .await
                {
                    return Flow::Stop;
                }
                self.finish_turn().await
            }
            Tag::Builtin { command, ticket } => {
                // The message is consumed either way; a failure is the reply.
                if !self.accept_local_input(ticket).await {
                    return Flow::Stop;
                }
                let text = if ok {
                    command.render(&data)
                } else {
                    self.turn_error = Some(error.clone());
                    error
                };
                if !self.emit(AgentEvent::TextDelta { text }).await {
                    return Flow::Stop;
                }
                self.finish_turn().await
            }
            Tag::Abort => {
                if !ok {
                    tracing::debug!(target: "zeron_harness::pi", "abort rejected: {error}");
                }
                Flow::Continue
            }
            Tag::Ignore => Flow::Continue,
        }
    }

    /// Acknowledge an input Pi answered without starting an agent run (a
    /// compaction or a mapped terminal command): the root prompt is accepted,
    /// a steer is accepted at a fresh assistant-message boundary.
    async fn accept_local_input(&mut self, ticket: Option<Ticket>) -> bool {
        match ticket {
            Some(ticket) => {
                let (prev, next) = rotate(&mut self.assistant_message_id);
                let message_id = ticket.message_id.clone();
                ticket.accept();
                for event in [
                    AgentEvent::Steered {
                        assistant_message_id: Some(prev),
                        next_assistant_message_id: Some(next),
                    },
                    crate::input_accepted_event(message_id),
                ] {
                    if !self.emit(event).await {
                        return false;
                    }
                }
                true
            }
            None => self.emit(AgentEvent::InputAccepted).await,
        }
    }

    async fn emit_flow(&self, event: AgentEvent) -> Flow {
        if self.emit(event).await {
            Flow::Continue
        } else {
            Flow::Stop
        }
    }

    /// The run's input was refused or its compaction failed: report it as the
    /// turn's terminal error and end the run.
    async fn fail_turn(&mut self, error: String) -> Flow {
        self.turn_error = Some(error);
        self.finish_turn().await;
        Flow::Stop
    }

    async fn on_message_start(&mut self, record: &Value) -> Flow {
        let message = record.get("message").unwrap_or(&Value::Null);
        match message.get("role").and_then(Value::as_str) {
            Some("user") => {
                if self.expect_root_user {
                    self.expect_root_user = false;
                } else if self.pending_deliveries > 0 {
                    self.pending_deliveries -= 1;
                    let (prev, next) = rotate(&mut self.assistant_message_id);
                    return self
                        .emit_flow(AgentEvent::Steered {
                            assistant_message_id: Some(prev),
                            next_assistant_message_id: Some(next),
                        })
                        .await;
                }
            }
            Some("assistant") => {
                self.message_had_text = false;
                self.streamed_blocks.clear();
            }
            _ => {}
        }
        Flow::Continue
    }

    async fn on_message_update(&mut self, record: &Value) -> Flow {
        let event = record.get("assistantMessageEvent").unwrap_or(&Value::Null);
        let kind = event.get("type").and_then(Value::as_str).unwrap_or_default();
        let index = event.get("contentIndex").and_then(Value::as_u64).unwrap_or(0);
        let delta = event
            .get("delta")
            .and_then(Value::as_str)
            .filter(|d| !d.is_empty());
        // `*_end` carries the authoritative block; it only renders when the
        // provider streamed nothing for that block.
        let end_content = event
            .get("content")
            .and_then(Value::as_str)
            .filter(|c| !c.is_empty());
        let thinking_block = index | (1 << 32);
        let out = match kind {
            "text_delta" => delta.map(|text| {
                self.streamed_blocks.insert(index);
                self.message_had_text = true;
                AgentEvent::TextDelta { text: text.to_owned() }
            }),
            "thinking_delta" => delta.map(|text| {
                self.streamed_blocks.insert(thinking_block);
                AgentEvent::ReasoningDelta { text: text.to_owned() }
            }),
            "text_end" if !self.streamed_blocks.contains(&index) => {
                end_content.map(|text| {
                    self.message_had_text = true;
                    AgentEvent::TextDelta { text: text.to_owned() }
                })
            }
            "thinking_end" if !self.streamed_blocks.contains(&thinking_block) => {
                end_content.map(|text| AgentEvent::ReasoningDelta { text: text.to_owned() })
            }
            _ => None,
        };
        match out {
            Some(event) => self.emit_flow(event).await,
            None => Flow::Continue,
        }
    }

    async fn on_message_end(&mut self, record: &Value) -> Flow {
        let message = record.get("message").unwrap_or(&Value::Null);
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            return Flow::Continue;
        }
        if self.message_had_text {
            self.message_had_text = false;
            // Consecutive assistant messages carry no separator in their
            // deltas; close each as a paragraph and mark the boundary.
            let (prev, _) = rotate(&mut self.assistant_message_id);
            for event in [
                AgentEvent::TextDelta { text: "\n\n".into() },
                AgentEvent::AssistantMessageCompleted {
                    assistant_message_id: prev,
                },
            ] {
                if !self.emit(event).await {
                    return Flow::Stop;
                }
            }
        }
        match message.get("stopReason").and_then(Value::as_str) {
            // Our own abort surfaces as an errored message; it is the
            // requested outcome, not a failure.
            Some("error") if !self.interrupted && self.turn_error.is_none() => {
                self.turn_error = Some(
                    message
                        .get("errorMessage")
                        .and_then(Value::as_str)
                        .unwrap_or("Pi reported a model error.")
                        .to_owned(),
                );
            }
            _ => {}
        }
        let used = message
            .pointer("/usage/totalTokens")
            .and_then(Value::as_u64)
            .filter(|tokens| *tokens > 0);
        if let (Some(tokens), Some(window)) = (used, self.window) {
            return self
                .emit_flow(AgentEvent::ContextUsage {
                    tokens: Some(tokens),
                    window: Some(window),
                })
                .await;
        }
        Flow::Continue
    }

    async fn on_tool_start(&mut self, record: &Value) -> Flow {
        let Some(id) = record.get("toolCallId").and_then(Value::as_str) else {
            return Flow::Continue;
        };
        let name = record
            .get("toolName")
            .and_then(Value::as_str)
            .unwrap_or("tool");
        let args = record.get("args").cloned().unwrap_or(Value::Null);
        let call = normalize::tool_call(name, &args);
        self.tool_calls
            .insert(id.to_owned(), (name.to_owned(), args));
        self.emit_flow(AgentEvent::ToolCall {
            id: id.to_owned(),
            call,
        })
        .await
    }

    async fn on_tool_end(&mut self, record: &Value) -> Flow {
        let Some(id) = record.get("toolCallId").and_then(Value::as_str) else {
            return Flow::Continue;
        };
        let is_error = record.get("isError").and_then(Value::as_bool) == Some(true);
        let call = self.tool_calls.remove(id);
        let diff = match (&call, is_error) {
            (Some((name, args)), false) => normalize::tool_diff(name, args),
            _ => None,
        };
        self.emit_flow(AgentEvent::ToolResult {
            id: id.to_owned(),
            is_error,
            output: record.get("result").and_then(normalize::result_output),
            diff,
        })
        .await
    }

    async fn on_compaction_end(&mut self, record: &Value) -> Flow {
        self.compacting = false;
        // A manual compaction reports its own outcome; an automatic one that
        // failed would otherwise look like a stall.
        if record.get("reason").and_then(Value::as_str) != Some("manual")
            && record.get("aborted").and_then(Value::as_bool) != Some(true)
            && let Some(reason) = record
                .get("errorMessage")
                .and_then(Value::as_str)
                .filter(|reason| !reason.is_empty())
            && !self
                .emit(AgentEvent::Error {
                    message: format!("Pi could not compact the conversation: {reason}"),
                })
                .await
        {
            return Flow::Stop;
        }
        // A successful overflow recovery retries the prompt: the model error
        // that triggered it is no longer the turn's outcome.
        if record.get("willRetry").and_then(Value::as_bool) == Some(true) {
            self.turn_error = None;
        }
        // Context is unknown until the next assistant response; the snapshot
        // replaces the stale pre-compaction number.
        self.emit_usage().await;
        if self.settle_pending {
            return self.probe_idle().await;
        }
        Flow::Continue
    }

    // -- settling -----------------------------------------------------------

    async fn on_settled(&mut self) -> Flow {
        if !self.turn_open {
            return Flow::Continue;
        }
        if self.interrupted {
            return self.finish_turn().await;
        }
        self.settle_pending = true;
        self.probe_idle().await
    }

    /// Confirm Pi is idle before terminalizing: an extension can start
    /// compaction or a queued run as `agent_settled` unwinds.
    async fn probe_idle(&mut self) -> Flow {
        for attempt in 1..=SETTLE_PROBE_ATTEMPTS {
            let Some(reply) = self
                .request_unless_interrupted(json!({"type": "get_state"}), QUICK_TIMEOUT)
                .await
            else {
                // Stopped while checking: the turn did settle, so it ends as
                // it was; the interrupt is handled by the loop right after.
                return self.finish_turn().await;
            };
            match reply {
                Ok(state) => {
                    let busy = ["isStreaming", "isCompacting"]
                        .iter()
                        .any(|key| state.get(*key).and_then(Value::as_bool) == Some(true))
                        || state
                            .get("pendingMessageCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                        || self.compacting
                        || self.manual_compact;
                    // Still busy: the run's own next `agent_settled` (or
                    // `compaction_end`) re-probes.
                    return if busy {
                        Flow::Continue
                    } else {
                        self.finish_turn().await
                    };
                }
                Err(_) if attempt < SETTLE_PROBE_ATTEMPTS => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => break,
            }
        }
        // Pi stopped answering: report the settled turn rather than hang.
        self.finish_turn().await
    }

    async fn finish_turn(&mut self) -> Flow {
        if !self.turn_open {
            return Flow::Continue;
        }
        self.settle_pending = false;
        self.emit_usage().await;
        self.emit_native_reference().await;
        let status = if self.interrupted {
            DoneStatus::Interrupted
        } else if self.turn_error.is_some() {
            DoneStatus::Errored
        } else {
            DoneStatus::Completed
        };
        let error = (status == DoneStatus::Errored)
            .then(|| self.turn_error.clone())
            .flatten();
        self.turn_open = false;
        self.turn_error = None;
        self.expect_root_user = false;
        self.pending_deliveries = 0;
        self.tool_calls.clear();
        self.done_after_interrupt = self.interrupted;
        if !self
            .emit(AgentEvent::Done {
                status,
                result: None,
                error,
                session_id: Some(self.session_file.clone()),
            })
            .await
        {
            return Flow::Stop;
        }
        if self.interrupted || !self.steering_open {
            Flow::Stop
        } else {
            Flow::Continue
        }
    }

    /// Context occupancy from `get_session_stats`. Pi reports `tokens: null`
    /// right after compaction until a fresh response; that is a real state
    /// ("waiting"), not an absent one.
    async fn emit_usage(&mut self) {
        let Some(Ok(stats)) = self
            .request_unless_interrupted(json!({"type": "get_session_stats"}), QUICK_TIMEOUT)
            .await
        else {
            return;
        };
        let Some(window) = stats
            .pointer("/contextUsage/contextWindow")
            .and_then(Value::as_u64)
            .filter(|window| *window > 0)
        else {
            return;
        };
        self.window = Some(window);
        let totals = stats.get("tokens");
        let count = |key: &str| totals.and_then(|t| t.get(key)).and_then(Value::as_u64);
        let session = count("input").map(|input| SessionTokenTotals {
            input,
            output: count("output").unwrap_or(0),
            cache_read: count("cacheRead").unwrap_or(0),
        });
        let _ = self
            .emit(AgentEvent::ContextUsageSnapshot {
                usage: ContextUsage {
                    tokens: stats.pointer("/contextUsage/tokens").and_then(Value::as_u64),
                    window: Some(window),
                    compact_at: self.settings.compact_at(&self.model, window),
                    session,
                },
            })
            .await;
    }

    /// The session-tree entry of this turn's first user message becomes the
    /// provider turn's native ref: the point `fork` re-roots before.
    async fn emit_native_reference(&mut self) {
        let was_trusted = self.leaf_trusted;
        let mut turn_id = None;
        let mut synced = false;
        if was_trusted {
            let mut record = json!({"type": "get_entries"});
            if let Some(cursor) = &self.leaf_cursor {
                record["since"] = Value::String(cursor.clone());
            }
            // Bounded by this turn: the cursor is where the previous one ended
            // (and an unset cursor means the session was empty).
            if let Some(Ok(data)) = self.request_unless_interrupted(record, TREE_TIMEOUT).await {
                self.leaf_cursor = data
                    .get("leafId")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                turn_id = first_user_entry(&data);
                synced = true;
            }
        }
        if !synced {
            // The cursor was unknown or no longer matches (a Pi extension
            // rewrote the tree): resync from the file, never from a full
            // listing, and skip this turn's ref rather than point a fork too
            // far back.
            let leaf = leaf_of(&self.session_file).await;
            self.leaf_trusted = leaf != Leaf::Unknown;
            self.leaf_cursor = match leaf {
                Leaf::Entry(id) => Some(id),
                Leaf::Empty | Leaf::Unknown => None,
            };
        }
        let _ = self
            .emit(AgentEvent::NativeReference {
                thread_id: self.session_file.clone(),
                turn_id,
            })
            .await;
    }
}

/// The session file's current leaf, read off the async workers.
async fn leaf_of(session_file: &str) -> Leaf {
    let path = PathBuf::from(session_file);
    tokio::task::spawn_blocking(move || lifecycle::session_leaf(&path))
        .await
        .unwrap_or(Leaf::Unknown)
}

fn first_user_entry(data: &Value) -> Option<String> {
    data.get("entries")?
        .as_array()?
        .iter()
        .find(|entry| {
            entry.get("type").and_then(Value::as_str) == Some("message")
                && entry.pointer("/message/role").and_then(Value::as_str) == Some("user")
        })?
        .get("id")?
        .as_str()
        .map(str::to_owned)
}

fn compact_record(instructions: Option<String>) -> Value {
    match instructions {
        Some(instructions) => json!({"type": "compact", "customInstructions": instructions}),
        None => json!({"type": "compact"}),
    }
}

fn thousands(tokens: u64) -> String {
    if tokens >= 10_000 {
        format!("{}K", (tokens + 500) / 1000)
    } else {
        tokens.to_string()
    }
}

/// The visible result of a manual compaction.
fn compaction_summary(data: &Value) -> String {
    match (
        data.get("tokensBefore").and_then(Value::as_u64),
        data.get("estimatedTokensAfter").and_then(Value::as_u64),
    ) {
        (Some(before), Some(after)) => format!(
            "Compacted the conversation: {} → ~{} tokens.",
            thousands(before),
            thousands(after)
        ),
        _ => "Compacted the conversation.".into(),
    }
}

impl Ticket {
    /// Pi accepted the message natively.
    fn accept(self) {
        if let Some(receipt) = self.receipt {
            let _ = receipt.send(true);
        }
    }

    /// Pi refused it; the durable queue still owns delivery.
    fn reject(self) {
        if let Some(receipt) = self.receipt {
            let _ = receipt.send(false);
        }
    }
}

/// Load image attachments as Pi `ImageContent`, best-effort: an unreadable,
/// oversized or unsupported file is skipped (its path ref still rides the
/// prompt text), never fatal to the run.
async fn load_images(paths: &[String]) -> Vec<Value> {
    use base64::Engine as _;
    let mut images = Vec::new();
    for path in paths {
        let Ok(bytes) = tokio::fs::read(path).await else {
            tracing::warn!(target: "zeron_harness::pi", %path, "attachment unreadable; path ref only");
            continue;
        };
        if bytes.len() as u64 > crate::claude::MAX_INLINE_IMAGE_BYTES {
            continue;
        }
        let Some(mime) = crate::claude::image_media_type(Path::new(path), &bytes) else {
            continue;
        };
        images.push(json!({
            "type": "image",
            "mimeType": mime,
            "data": base64::engine::general_purpose::STANDARD.encode(&bytes),
        }));
    }
    images
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_confirmations_are_recognized_by_their_marker_only() {
        let record = json!({"method":"confirm","title":"Allow bash?",
            "message": json!({"noches":"permission","tool":"bash","input":{"command":"ls"}}).to_string()});
        let (tool, input) = permission_payload(&record).unwrap();
        assert_eq!(tool, "bash");
        assert_eq!(input["command"], "ls");
        // A user extension's own confirm stays a question, even with JSON.
        assert!(permission_payload(&json!({"message":"{\"a\":1}"})).is_none());
        assert!(permission_payload(&json!({"message":"plain text"})).is_none());
        assert!(permission_payload(&json!({"title":"no message"})).is_none());
    }

    #[test]
    fn dialogs_map_to_questions_and_answers_back() {
        let select = json!({"id":"d1","title":"Pick","options":["a","b"],"message":"Choose one"});
        let question = dialog_question(&select, "select");
        assert_eq!(question.options, ["a", "b"]);
        assert_eq!(question.question, "Choose one");
        assert_eq!(question.header, "Pick");
        let editor = json!({"id":"d2","title":"Edit","prefill":"line 1"});
        let question = dialog_question(&editor, "editor");
        assert!(question.options.is_empty());
        assert!(question.question.contains("Current value:\nline 1"));
        assert_eq!(dialog_question(&json!({"id":"d3"}), "confirm").options, ["Yes", "No"]);

        assert_eq!(dialog_response("select", Some(vec!["b".into()])), json!({"value":"b"}));
        assert_eq!(dialog_response("confirm", Some(vec!["Yes".into()])), json!({"confirmed":true}));
        assert_eq!(dialog_response("confirm", Some(vec!["No".into()])), json!({"confirmed":false}));
        assert_eq!(dialog_response("confirm", None), json!({"cancelled":true}));
        // An empty submission is a value for free-text dialogs, a cancel for select.
        assert_eq!(dialog_response("input", Some(vec![])), json!({"value":""}));
        assert_eq!(dialog_response("select", Some(vec![])), json!({"cancelled":true}));
    }

    #[test]
    fn commands_lead_with_the_compact_builtin_and_skip_duplicates() {
        let data = json!({"commands":[
            {"name":"compact","description":"ext shadow"},
            {"name":"skill:review","description":"Review","source":"skill"},
            {"name":"cc-theme","description":"Toggle"},
            {"description":"nameless"}
        ]});
        let commands = commands_from(&data);
        let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "compact", "name", "session", "autocompact", "steering", "follow-up", "export",
                "skill:review", "cc-theme"
            ]
        );
        assert_eq!(commands[0].input_hint.as_deref(), Some("Optional instructions"));
        assert_eq!(skill_names(&data), HashSet::from(["review".to_owned()]));
        assert!(command_names(&data).contains("skill:review"));
        assert_eq!(commands_from(&Value::Null).len(), 1 + launch::MAPPED_COMMANDS.len());
    }

    #[test]
    fn the_turn_ref_is_the_first_user_entry_in_the_window() {
        let data = json!({"entries":[
            {"type":"message","id":"sys","message":{"role":"system"}},
            {"type":"message","id":"u1","message":{"role":"user"}},
            {"type":"message","id":"a1","message":{"role":"assistant"}},
            {"type":"message","id":"u2","message":{"role":"user"}}
        ],"leafId":"u2"});
        assert_eq!(first_user_entry(&data).as_deref(), Some("u1"));
        assert_eq!(first_user_entry(&json!({"entries":[]})), None);
        assert_eq!(first_user_entry(&Value::Null), None);
    }

    #[test]
    fn compaction_results_read_as_one_line() {
        assert_eq!(
            compaction_summary(&json!({"tokensBefore":150000,"estimatedTokensAfter":32000})),
            "Compacted the conversation: 150K → ~32K tokens."
        );
        assert_eq!(
            compaction_summary(&json!({"tokensBefore":900,"estimatedTokensAfter":300})),
            "Compacted the conversation: 900 → ~300 tokens."
        );
        assert_eq!(compaction_summary(&Value::Null), "Compacted the conversation.");
        assert_eq!(compact_record(None), json!({"type":"compact"}));
        assert_eq!(
            compact_record(Some("keep it".into())),
            json!({"type":"compact","customInstructions":"keep it"})
        );
    }

    #[test]
    fn runtime_modes_map_to_the_policy_extensions_vocabulary() {
        assert_eq!(mode_name(RuntimeMode::ApprovalRequired), "approval-required");
        assert_eq!(mode_name(RuntimeMode::AutoAcceptEdits), "auto-accept-edits");
        assert_eq!(mode_name(RuntimeMode::FullAccess), "full-access");
    }

    #[test]
    fn message_ids_rotate_returning_the_closed_one() {
        let mut id = "first".to_owned();
        let (prev, next) = rotate(&mut id);
        assert_eq!(prev, "first");
        assert_eq!(id, next);
        assert_ne!(id, "first");
    }
}
