//! Native Pi driver: `pi --mode rpc` over LF-delimited JSON records.
//!
//! Pi (pi.dev) is driven the way its own RPC client and T3 Code's
//! `PiAdapterV2` drive it, not through an ACP adapter: exact `provider/id`
//! models, prompt/steer receipts from Pi's own command responses
//! (`disposition: started | queued | handled`), `agent_settled` as the only
//! terminal signal, session-tree entry ids as turn references, and native
//! `fork`/`switch_session` lifecycle. The session file path is the durable
//! native thread id, so a thread started in Noches resumes from the Pi TUI and
//! vice versa.
//!
//! The process is spawned with no `--no-*` flags: the user's extensions,
//! skills, prompt templates, AGENTS.md context, settings.json, custom models
//! and auth load exactly as in `pi`. Noches adds private extensions through
//! `-e` (computer use, session MCP tools, the runtime-policy hook).

mod catalog;
mod launch;
mod lifecycle;
mod normalize;
mod rpc;
mod session;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use serde_json::{Value, json};
use zeron_proto::{
    AgentEvent, HarnessId, Model, ReasoningLevel, RunRequest, SlashCommand, SteeringMode,
};

use crate::process::{Command, Stdio};
use crate::{Harness, HarnessError, RunControls};
use rpc::PiRpc;

/// Cold start of Pi plus the user's extensions before the first answer. The
/// widest discovery budget of any driver; callers bounding a `models()` round
/// trip (the picker's deadline, the engine's forward deadline) derive from it.
pub const MODEL_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(60);

const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Concurrent callers (picker + readiness) share one discovery.
const DISCOVERY_SHARE_WINDOW: Duration = Duration::from_secs(5);
/// A failed discovery (a hung extension, a missing provider) is not retried
/// for this long.
const DISCOVERY_FAILURE_WINDOW: Duration = Duration::from_secs(20);

const CUA_EXTENSION: &str = include_str!("noches-cua.ts");
const POLICY_EXTENSION: &str = include_str!("noches-policy.ts");

/// Env markers set for the child; the policy hook reads the mode from here.
pub(crate) const RUNTIME_MODE_ENV: &str = "NOCHES_PI_RUNTIME_MODE";

fn extra_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = crate::executable::home_dir() {
        paths.push(home.join(".local").join("bin").join("pi"));
        paths.push(home.join(".npm-global").join("bin").join("pi"));
    }
    paths.push(PathBuf::from("/opt/homebrew/bin/pi"));
    paths.push(PathBuf::from("/usr/local/bin/pi"));
    paths
}

/// What one discovery process learned.
#[derive(Clone)]
struct Discovery {
    models: Vec<Model>,
    commands: Vec<SlashCommand>,
}

/// The Pi harness. Construct with [`PiHarness::new`]; tests point it at a fake
/// `pi` with [`PiHarness::with_executable`].
pub struct PiHarness {
    launch: crate::instance::InstanceLaunch,
    executable: Option<PathBuf>,
    interrupt_grace: Duration,
    kill_grace: Duration,
    /// Declared context windows by `provider/id`, learned from discovery and
    /// from each run's model.
    windows: Arc<Mutex<std::collections::HashMap<String, u64>>>,
    /// The latest probe's outcome: a failure is remembered briefly so pickers
    /// and readiness checks do not each start a Pi that loads every extension.
    discovery: tokio::sync::Mutex<Option<(Instant, Result<Discovery, HarnessError>)>>,
    /// Versions that already passed the minimum-version gate, by executable
    /// and its modification time (an in-place downgrade is checked again).
    verified: Mutex<std::collections::HashSet<(PathBuf, Option<std::time::SystemTime>)>>,
}

impl Default for PiHarness {
    fn default() -> Self {
        Self {
            launch: Default::default(),
            executable: None,
            interrupt_grace: Duration::from_secs(2),
            kill_grace: Duration::from_secs(3),
            windows: Default::default(),
            discovery: tokio::sync::Mutex::new(None),
            verified: Mutex::default(),
        }
    }
}

impl PiHarness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_instance_launch(mut self, launch: crate::instance::InstanceLaunch) -> Self {
        self.launch = launch;
        self
    }

    /// Use a fixed `pi` binary instead of PATH/known-location resolution.
    pub fn with_executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Tune the abort→SIGTERM→SIGKILL escalation timing.
    pub fn with_graces(mut self, interrupt_grace: Duration, kill_grace: Duration) -> Self {
        self.interrupt_grace = interrupt_grace;
        self.kill_grace = kill_grace;
        self
    }

    /// Check a launch-argument string the way a run will, so settings can
    /// reject an unsupported value up front instead of at the first prompt.
    pub fn validate_launch_args(args: &[String]) -> Result<(), String> {
        launch::resolve_launch_args(args).map(|_| ())
    }

    fn resolve_executable(&self) -> Result<PathBuf, HarnessError> {
        if let Some(path) = &self.executable {
            return crate::executable::validate_native_override(path);
        }
        if let Some(path) = std::env::var_os("PI_EXECUTABLE").filter(|p| !p.is_empty()) {
            return crate::executable::validate_native_override(&PathBuf::from(path));
        }
        crate::executable::find_on_paths("pi", extra_paths()).ok_or_else(|| {
            HarnessError::NotInstalled(
                "pi (searched PATH, the login shell's PATH, ~/.local/bin, ~/.npm-global/bin, \
                 /opt/homebrew/bin, /usr/local/bin, and fnm/nvm/volta/pnpm/bun install dirs; \
                 install with `npm install -g --ignore-scripts @earendil-works/pi-coding-agent`, \
                 or set PI_EXECUTABLE to override)"
                    .into(),
            )
        })
    }

    /// The validated user launch arguments of this instance.
    fn user_args(&self) -> Result<Vec<String>, HarnessError> {
        launch::resolve_launch_args(&self.launch.args).map_err(HarnessError::Protocol)
    }

    /// Refuse a Pi too old for this driver, once per executable. A probe that
    /// cannot run is not a verdict: the real launch reports its own failure.
    async fn check_version(&self, exe: &Path) -> Result<(), HarnessError> {
        let stamp = (
            exe.to_path_buf(),
            std::fs::metadata(exe).and_then(|meta| meta.modified()).ok(),
        );
        if self
            .verified
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&stamp)
        {
            return Ok(());
        }
        let mut command = Command::new(exe);
        crate::compose_child_environment(&mut command, exe);
        self.launch.apply(&mut command);
        command
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let Ok(Ok(output)) = tokio::time::timeout(VERSION_PROBE_TIMEOUT, command.output()).await
        else {
            return Ok(());
        };
        // Older Pi printed the version on stderr; stdout wins when both do.
        let found = launch::parse_version(&String::from_utf8_lossy(&output.stdout))
            .or_else(|| launch::parse_version(&String::from_utf8_lossy(&output.stderr)));
        match found {
            Some(version) => {
                if let Some(error) = launch::version_error(Some(version)) {
                    return Err(HarnessError::Protocol(error));
                }
            }
            None if output.status.success() => {
                if let Some(error) = launch::version_error(None) {
                    return Err(HarnessError::Protocol(error));
                }
            }
            None => return Ok(()),
        }
        self.verified
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(stamp);
        Ok(())
    }

    /// Every directory this instance keeps sessions in for runs in `cwd`.
    fn session_roots_for(&self, cwd: &str) -> Result<Vec<PathBuf>, HarnessError> {
        let user_args = self.user_args()?;
        let cwd = PathBuf::from(cwd);
        let env = &self.launch.environment;
        let agent_dir = launch::agent_dir(env);
        let settings = launch::PiSettings::read(&agent_dir, &cwd);
        Ok(launch::session_roots(&user_args, env, &settings, &agent_dir, &cwd))
    }

    /// A configured `pi --mode rpc` child. `cwd` is the working directory the
    /// session runs in; `extra` are Noches `-e` extensions and session args.
    fn command(&self, exe: &Path, cwd: Option<&str>, args: &[String]) -> Command {
        let mut command = Command::new(exe);
        crate::compose_child_environment(&mut command, exe);
        self.launch.apply(&mut command);
        crate::acp::child::configure(&mut command);
        command
            .arg("--mode")
            .arg("rpc")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = cwd.filter(|cwd| !cwd.is_empty()) {
            command.current_dir(cwd);
        }
        command
    }

    /// Spawn and attach the transport.
    fn spawn(&self, mut command: Command) -> Result<Spawned, HarnessError> {
        let mut child = command.spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(HarnessError::Protocol("pi child has no stdio".into()));
        };
        let stderr_tail = crate::StderrTail::default();
        if let Some(stderr) = child.stderr.take() {
            let tail = stderr_tail.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tail.push(&line);
                }
                tail.close();
            });
        }
        let (rpc, events) = PiRpc::new(stdin, stdout);
        Ok(Spawned {
            child: crate::acp::child::Child::new(child),
            rpc,
            events,
            stderr_tail,
        })
    }

    /// One short-lived `pi --mode rpc --no-session` probe: models, commands
    /// and the configured default, with the user's extensions loaded (custom
    /// providers register their models there).
    async fn discover(&self) -> Result<Discovery, HarnessError> {
        let mut shared = self.discovery.lock().await;
        if let Some((at, outcome)) = shared.as_ref() {
            match outcome {
                Ok(found) if at.elapsed() < DISCOVERY_SHARE_WINDOW => return Ok(found.clone()),
                Err(error) if at.elapsed() < DISCOVERY_FAILURE_WINDOW => {
                    return Err(replayed(error));
                }
                _ => {}
            }
        }
        // Not installed or too old are answered cheaply and never remembered.
        let exe = self.resolve_executable()?;
        self.check_version(&exe).await?;
        let outcome = self.probe(&exe).await;
        *shared = Some((
            Instant::now(),
            match &outcome {
                Ok(found) => Ok(found.clone()),
                Err(error) => Err(replayed(error)),
            },
        ));
        outcome
    }

    async fn probe(&self, exe: &Path) -> Result<Discovery, HarnessError> {
        let mut args = self.user_args()?;
        args.insert(0, "--no-session".into());
        let cwd = crate::executable::home_or_current_dir();
        let Spawned {
            mut child,
            rpc,
            mut events,
            stderr_tail,
        } = self.spawn(self.command(exe, cwd.to_str(), &args))?;
        // Startup dialogs (project trust, login) cannot be answered here:
        // cancel them so an extension never blocks the probe.
        let canceller = rpc.clone();
        let drain = tokio::spawn(async move {
            while let Some(record) = events.recv().await {
                if record.get("type").and_then(Value::as_str) == Some("extension_ui_request")
                    && let Some(id) = record.get("id").and_then(Value::as_str)
                    && matches!(
                        record.get("method").and_then(Value::as_str),
                        Some("select" | "confirm" | "input" | "editor")
                    )
                {
                    let _ = canceller.send(json!({
                        "type": "extension_ui_response",
                        "id": id,
                        "cancelled": true,
                    }));
                }
            }
        });
        let probe = async {
            let models = rpc
                .request(json!({"type":"get_available_models"}), MODEL_DISCOVERY_TIMEOUT)
                .await?;
            let commands = rpc
                .request(json!({"type":"get_commands"}), Duration::from_secs(5))
                .await
                .unwrap_or(Value::Null);
            let state = rpc
                .request(json!({"type":"get_state"}), Duration::from_secs(5))
                .await
                .unwrap_or(Value::Null);
            Ok::<_, HarnessError>((models, commands, state))
        };
        let result = tokio::time::timeout(MODEL_DISCOVERY_TIMEOUT, probe).await;
        drain.abort();
        let died = child.try_wait().ok().flatten();
        child.shutdown(self.kill_grace).await;
        let (models, commands, state) = match result {
            Ok(Ok(found)) => found,
            Ok(Err(error)) => {
                return Err(match died {
                    Some(_) => HarnessError::Protocol(crate::crash_message(
                        "pi",
                        died,
                        &stderr_tail,
                    )),
                    None => error,
                });
            }
            Err(_) => {
                return Err(HarnessError::Protocol(format!(
                    "Pi model discovery did not complete within {}s",
                    MODEL_DISCOVERY_TIMEOUT.as_secs()
                )));
            }
        };
        let (catalog, windows) = catalog::parse_models(&models);
        {
            let mut known = self
                .windows
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            known.extend(windows);
            if let Some(default) = state.get("model").and_then(catalog::slug)
                && let Some(window) = state
                    .get("model")
                    .and_then(catalog::context_window)
            {
                known.insert(catalog::DEFAULT_MODEL.into(), window);
                known.insert(default, window);
            }
        }
        let found = Discovery {
            models: catalog,
            commands: session::commands_from(&commands),
        };
        Ok(found)
    }
}

/// A copy of a remembered discovery failure (errors are not `Clone`).
fn replayed(error: &HarnessError) -> HarnessError {
    match error {
        HarnessError::Transport(message) => HarnessError::Transport(message.clone()),
        HarnessError::Protocol(message) => HarnessError::Protocol(message.clone()),
        other => HarnessError::Protocol(other.to_string()),
    }
}

pub(crate) struct Spawned {
    pub child: crate::acp::child::Child,
    pub rpc: PiRpc,
    pub events: tokio::sync::mpsc::UnboundedReceiver<Value>,
    pub stderr_tail: crate::StderrTail,
}

#[async_trait]
impl Harness for PiHarness {
    fn session_lifecycle(&self) -> Option<&dyn crate::session_lifecycle::SessionLifecycle> {
        Some(self)
    }
    fn id(&self) -> HarnessId {
        HarnessId::Pi
    }
    fn display_name(&self) -> &str {
        "Pi"
    }
    fn supports_steering(&self) -> bool {
        true
    }
    /// A steer is delivered after the current assistant turn finishes its tool
    /// calls, before the next model call.
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::StepBoundary
    }
    /// Receipts are Pi's own `prompt` responses (`queued`/`started`/`handled`),
    /// so the local `Steered` boundary cannot retire a pending input.
    fn confirms_steered_inputs(&self) -> bool {
        true
    }
    fn model_context_window(
        &self,
        model: &str,
        _options: &serde_json::Map<String, Value>,
    ) -> Option<u64> {
        self.windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(model)
            .copied()
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[
            ReasoningLevel::Minimal,
            ReasoningLevel::Low,
            ReasoningLevel::Medium,
            ReasoningLevel::High,
            ReasoningLevel::XHigh,
            ReasoningLevel::Max,
        ]
    }
    fn installed(&self) -> bool {
        self.resolve_executable().is_ok()
    }
    /// `agent_settled` is Pi's own terminal signal, for self-started runs too.
    fn deterministic_turn_end(&self) -> bool {
        true
    }

    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(self.discover().await?.models)
    }

    /// Pi has no credential-free login probe; a Pi that lists at least one
    /// model has a usable provider configured.
    async fn authenticated(&self) -> Result<Option<bool>, HarnessError> {
        let found = self.discover().await?;
        Ok(Some(found.models.iter().any(|m| m.id != catalog::DEFAULT_MODEL)))
    }

    async fn commands(&self) -> Result<Vec<SlashCommand>, HarnessError> {
        Ok(self.discover().await?.commands)
    }

    async fn run(
        &self,
        request: RunRequest,
        mut controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let mcp_guard = controls.mcp.run_guard();
        controls.bind_browser()?;
        let policy = crate::policy::compile(
            HarnessId::Pi,
            request.runtime_mode,
            request.interaction_mode,
        )?;
        let exe = self.resolve_executable()?;
        self.check_version(&exe).await?;
        let user_args = self.user_args()?;
        let cwd = PathBuf::from(&request.cwd);
        let env = &self.launch.environment;
        let agent_dir = launch::agent_dir(env);
        let settings = launch::PiSettings::read(&agent_dir, &cwd);
        let roots = launch::session_roots(&user_args, env, &settings, &agent_dir, &cwd);
        let wanted = request.resume.clone();
        // The session store walk is blocking file I/O.
        let resume = tokio::task::spawn_blocking(move || {
            launch::resolve_resume(wanted.as_deref(), &roots, Some(&launch::acp_session_map()))
        })
        .await
        .map_err(|error| HarnessError::Protocol(error.to_string()))?;

        let extensions = session::Extensions::provision(&controls, policy.runtime)?;
        let mut args = Vec::new();
        if let launch::Resume::File(path) = &resume {
            args.extend(["--session".to_owned(), path.display().to_string()]);
        }
        args.extend(user_args);
        args.extend(extensions.args());

        let mut command = self.command(&exe, Some(&request.cwd), &args);
        extensions.apply_env(&mut command, &controls);
        let spawned = self.spawn(command)?;
        spawned.stderr_tail.retain_mcp(&controls.mcp);
        let (event_tx, event_rx) = crate::session_event_channel(&controls.mcp);
        let runner = session::Runner::new(session::RunnerInit {
            spawned,
            event_tx,
            controls,
            request,
            settings,
            resume,
            extensions,
            windows: self.windows.clone(),
            interrupt_grace: self.interrupt_grace,
            kill_grace: self.kill_grace,
        });
        tokio::spawn(async move {
            let _mcp_start_guard = mcp_guard;
            runner.run().await;
        });
        Ok(futures::stream::unfold(event_rx, |mut rx| async move {
            rx.recv().await.map(|ev| (ev, rx))
        })
        .boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_harness_reports_its_native_contract() {
        let harness = PiHarness::new();
        assert_eq!(harness.id(), HarnessId::Pi);
        assert!(harness.supports_steering());
        assert_eq!(harness.steering_mode(), SteeringMode::StepBoundary);
        assert!(harness.confirms_steered_inputs());
        assert!(harness.deterministic_turn_end());
        assert!(harness.session_lifecycle().is_some());
    }

    #[test]
    fn invalid_launch_args_are_reported_before_spawn() {
        assert!(PiHarness::validate_launch_args(&["--model".into(), "a/b".into()]).is_ok());
        assert!(PiHarness::validate_launch_args(&["--session".into(), "x".into()]).is_err());
    }
}
