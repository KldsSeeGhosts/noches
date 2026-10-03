//! Host-local, runtime-scoped MCP injection. Deliberately NOT a proto type:
//! neither entries nor contexts implement Serialize, so credentials cannot be
//! accidentally put in a RunRequest, CRDT document, or journal.
//!
//! ```compile_fail
//! let context = zeron_harness::mcp::SessionMcpContext::default();
//! serde_json::to_string(&context).unwrap(); // host-local bindings are not serializable
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::{CancellationToken, HarnessError};

pub mod bridge;
#[cfg(test)]
mod tests;

pub const ACP_EXECUTABLE_ENV: &str = "NOCHES_ACP_MCP_EXECUTABLE";
pub const ACP_ENDPOINT_ENV: &str = "NOCHES_ACP_MCP_ENDPOINT";
pub const ACP_AUTHORIZATION_ENV: &str = "NOCHES_ACP_MCP_AUTHORIZATION";
pub const MCP_ENTRIES_ENV: &str = "NOCHES_SESSION_MCP_ENTRIES";
pub const MCP_INSTRUCTIONS_ENV: &str = "NOCHES_SESSION_MCP_INSTRUCTIONS";
pub const MCP_ALLOWED_TOOLS_ENV: &str = "NOCHES_SESSION_MCP_ALLOWED_TOOLS";
pub const TOOL_TIMEOUT_MS: u64 = 65 * 60 * 1000;

#[derive(Clone)]
pub enum McpTransport {
    Stdio {
        command: PathBuf,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    },
    StreamableHttp {
        url: String,
        headers: BTreeMap<String, String>,
    },
}

#[derive(Clone)]
pub struct McpServerEntry {
    pub name: String,
    pub transport: McpTransport,
    /// Provider-qualified tool names (e.g. `mcp__t3-code__task_status`),
    /// or a server wildcard. Pre-approval is NOT an authorization boundary.
    pub allowed_tools: Vec<String>,
}

impl std::fmt::Debug for McpServerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServerEntry")
            .field("name", &crate::redact::redact_registered(&self.name))
            .field(
                "transport",
                &match self.transport {
                    McpTransport::Stdio { .. } => "stdio",
                    McpTransport::StreamableHttp { .. } => "streamable-http",
                },
            )
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stdio { .. } => "Stdio([host-local])",
            Self::StreamableHttp { .. } => "StreamableHttp([host-local])",
        })
    }
}

impl McpServerEntry {
    pub fn stdio(name: impl Into<String>, command: impl Into<PathBuf>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::Stdio {
                command: command.into(),
                args,
                env: BTreeMap::new(),
            },
            allowed_tools: Vec::new(),
        }
    }

    pub fn http(
        name: impl Into<String>,
        url: impl Into<String>,
        headers: BTreeMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::StreamableHttp {
                url: url.into(),
                headers,
            },
            allowed_tools: Vec::new(),
        }
    }

    /// Wire-only serialization, private to injection and subprocess plumbing.
    /// Never use this as a persisted model option or diagnostic.
    pub(crate) fn wire(&self) -> Value {
        match &self.transport {
            McpTransport::Stdio { command, args, env } => {
                json!({"command":command,"args":args,"env":env})
            }
            McpTransport::StreamableHttp { url, headers } => {
                json!({"type":"http","url":url,"headers":headers})
            }
        }
    }

    pub fn browser(connection: &zeron_browser::Connection) -> Self {
        let mut entry = Self::stdio("noches_browser", &connection.executable, connection.args());
        entry.allowed_tools.push("mcp__noches_browser__*".into());
        entry
    }
}

type Revoker = Box<dyn FnOnce() + Send + 'static>;

struct Context {
    entries: Vec<McpServerEntry>,
    instructions: String,
    allowed_tools: Vec<String>,
    executable: PathBuf,
    revoked: CancellationToken,
    revokers: Mutex<Vec<Revoker>>,
    // Retain redaction through teardown: queued provider output can arrive
    // after revocation. Removal happens only when the last clone is dropped.
    _secrets: crate::redact::RegisteredSecrets,
}

impl Drop for Context {
    fn drop(&mut self) {
        self.revoked.cancel();
        for revoke in self
            .revokers
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
        {
            revoke();
        }
    }
}

#[derive(Clone)]
pub struct SessionMcpContext(Arc<Context>);

impl Default for SessionMcpContext {
    fn default() -> Self {
        Self::new(Vec::new(), String::new(), Vec::new()).expect("empty MCP context")
    }
}

impl std::fmt::Debug for SessionMcpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionMcpContext")
            .field("entries", &self.entries())
            .field("revoked", &self.is_revoked())
            .finish_non_exhaustive()
    }
}

impl SessionMcpContext {
    pub fn new(
        entries: Vec<McpServerEntry>,
        instructions: String,
        mut allowed_tools: Vec<String>,
    ) -> Result<Self, HarnessError> {
        let mut names = std::collections::HashSet::new();
        for entry in &entries {
            if entry.name.is_empty()
                || !entry
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                || !names.insert(entry.name.clone())
            {
                return Err(HarnessError::Protocol(
                    "MCP server names must be unique ASCII identifiers".into(),
                ));
            }
            if let McpTransport::StreamableHttp { url, .. } = &entry.transport {
                let url = reqwest::Url::parse(url)
                    .map_err(|_| HarnessError::Protocol("invalid MCP endpoint".into()))?;
                if !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(HarnessError::Protocol(
                        "MCP requires an HTTP(S) endpoint without userinfo".into(),
                    ));
                }
            }
            allowed_tools.extend(entry.allowed_tools.iter().cloned());
        }
        allowed_tools.sort();
        allowed_tools.dedup();
        let mut secrets: Vec<String> = entries
            .iter()
            .flat_map(|entry| match &entry.transport {
                McpTransport::Stdio { env, .. } => env.values().cloned().collect::<Vec<_>>(),
                McpTransport::StreamableHttp { headers, .. } => headers
                    .values()
                    .flat_map(|v| {
                        [
                            Some(v.clone()),
                            v.split_once(' ')
                                .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
                                .map(|(_, token)| token.to_owned()),
                        ]
                        .into_iter()
                        .flatten()
                    })
                    .collect(),
            })
            .collect();
        for entry in &entries {
            if let McpTransport::StreamableHttp { url, .. } = &entry.transport {
                if let Ok(url) = reqwest::Url::parse(url) {
                    secrets.extend(url.query_pairs().map(|(_, v)| v.into_owned()));
                    secrets.extend(url.query().into_iter().flat_map(|query| {
                        query.split('&').filter_map(|pair| {
                            pair.split_once('=').map(|(_, value)| value.to_owned())
                        })
                    }));
                }
            }
        }
        let secrets = crate::redact::register_secrets(secrets);
        let instructions = crate::redact::redact_registered(&instructions);
        Ok(Self(Arc::new(Context {
            entries,
            instructions,
            allowed_tools,
            executable: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("noches")),
            revoked: CancellationToken::new(),
            revokers: Mutex::new(Vec::new()),
            _secrets: secrets,
        })))
    }

    /// Embedders/headless tests can use the standalone `noches-mcp` binary.
    pub fn with_executable(mut self, executable: PathBuf) -> Self {
        Arc::get_mut(&mut self.0)
            .expect("set MCP executable before sharing")
            .executable = executable;
        self
    }

    pub fn entries(&self) -> &[McpServerEntry] {
        if self.is_revoked() {
            &[]
        } else {
            &self.0.entries
        }
    }

    pub fn instructions(&self) -> &str {
        if self.is_revoked() {
            ""
        } else {
            &self.0.instructions
        }
    }

    pub fn allowed_tools(&self) -> &[String] {
        if self.is_revoked() {
            &[]
        } else {
            &self.0.allowed_tools
        }
    }

    pub fn is_revoked(&self) -> bool {
        self.0.revoked.is_cancelled()
    }
    pub fn same_registration(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// The later engine MCP server attaches its credential revocation here.
    /// Registration racing teardown executes immediately, never loses a revoke.
    pub fn on_revoke(&self, revoke: impl FnOnce() + Send + 'static) {
        let mut callbacks = self
            .0
            .revokers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.is_revoked() {
            drop(callbacks);
            revoke();
        } else {
            callbacks.push(Box::new(revoke));
        }
    }

    pub fn revoke(&self) {
        let callbacks = {
            let mut callbacks = self
                .0
                .revokers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.0.revoked.cancel();
            std::mem::take(&mut *callbacks)
        };
        for revoke in callbacks {
            revoke();
        }
    }

    pub(crate) fn run_guard(&self) -> McpRunGuard {
        McpRunGuard(self.clone())
    }

    pub fn with_browser(&self, browser: &zeron_browser::Connection) -> Result<Self, HarnessError> {
        let mut entries = self.entries().to_vec();
        entries.push(McpServerEntry::browser(browser));
        let instructions = [self.instructions(), &browser.instructions()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let context = Self::new(entries, instructions, self.allowed_tools().to_vec())?
            .with_executable(self.0.executable.clone());
        let parent = self.clone();
        context.on_revoke(move || parent.revoke());
        Ok(context)
    }

    /// Additive Claude CLI config: no `--strict-mcp-config`, leaving the
    /// user's global/project MCP configuration intact. Config is in a private
    /// tempfile, NOT argv (where process listings would expose credentials).
    pub(crate) fn claude_config(&self) -> Result<Option<tempfile::NamedTempFile>, HarnessError> {
        if self.entries().is_empty() {
            return Ok(None);
        }
        let servers: serde_json::Map<String, Value> = self
            .entries()
            .iter()
            .map(|e| {
                let mut wire = e.wire();
                if matches!(e.transport, McpTransport::StreamableHttp { .. }) {
                    wire["timeout"] = TOOL_TIMEOUT_MS.into();
                }
                (e.name.clone(), wire)
            })
            .collect();
        let mut file = tempfile::Builder::new()
            .prefix("noches-mcp-")
            .suffix(".json")
            .tempfile()?;
        use std::io::Write;
        file.write_all(json!({"mcpServers":servers}).to_string().as_bytes())?;
        Ok(Some(file))
    }

    /// Shared by start/resume/fork. Dotted overrides augment user config
    /// rather than replacing its whole `mcp_servers` table.
    pub fn codex_thread_overrides(&self) -> Value {
        let mut config = serde_json::Map::new();
        for entry in self.entries() {
            let wire = match &entry.transport {
                McpTransport::Stdio { command, args, env } => {
                    json!({"command":command,"args":args,"env":env})
                }
                McpTransport::StreamableHttp { url, headers } => {
                    json!({"url":url,"http_headers":headers,"tool_timeout_sec":TOOL_TIMEOUT_MS/1000})
                }
            };
            config.insert(format!("mcp_servers.{}", entry.name), wire);
        }
        Value::Object(config)
    }

    pub fn cursor_servers(&self) -> Value {
        Value::Object(
            self.entries()
                .iter()
                .map(|e| (e.name.clone(), e.wire()))
                .collect(),
        )
    }

    /// ACP stdio is the required baseline even if a provider advertises
    /// HTTP: several adapters accept but drop native HTTP injection.
    pub fn acp_servers(&self) -> Vec<Value> {
        self.entries().iter().map(|entry| match &entry.transport {
            McpTransport::Stdio { command, args, env } => json!({
                "name":entry.name,"command":command,"args":args,
                "env":env.iter().map(|(k,v)| json!({"name":k,"value":v})).collect::<Vec<_>>()
            }),
            McpTransport::StreamableHttp { url, headers } => json!({
                "name":entry.name,"command":self.0.executable,"args":["acp-mcp-bridge"],
                "env":[{"name":MCP_ENTRIES_ENV,"value":json!([{"name":entry.name,"config":{"type":"http","url":url,"headers":headers}}]).to_string()}]
            }),
        }).collect()
    }

    pub(crate) fn process_environment(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                ACP_EXECUTABLE_ENV,
                self.0.executable.to_string_lossy().into(),
            ),
            (
                MCP_ENTRIES_ENV,
                Value::Array(
                    self.entries()
                        .iter()
                        .map(|e| json!({"name":e.name,"config":e.wire()}))
                        .collect(),
                )
                .to_string(),
            ),
            (MCP_INSTRUCTIONS_ENV, self.instructions().into()),
            (
                MCP_ALLOWED_TOOLS_ENV,
                json!(self.allowed_tools()).to_string(),
            ),
        ]
    }

    pub(crate) fn acp_instructions(&self) -> String {
        if self.entries().is_empty() {
            return self.instructions().into();
        }
        format!(
            "{}\n\nNoches MCP fallback: if injected MCP tools are not exposed, use the private launcher from NOCHES_ACP_MCP_EXECUTABLE: \"$NOCHES_ACP_MCP_EXECUTABLE\" acp-mcp-call <tool> '<json>'. Use mcp__<server>__<tool> to disambiguate duplicate tool names. This calls the same session-scoped MCP server; never print the MCP environment or credentials.",
            self.instructions()
        )
    }

    pub(crate) fn pi_extension(&self, dir: &Path) -> Result<PathBuf, HarnessError> {
        let path = dir.join("noches-mcp.ts");
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&path)?
            .write_all(include_str!("pi/noches-mcp.ts").as_bytes())?;
        Ok(path)
    }
}

pub(crate) struct McpRunGuard(SessionMcpContext);
impl Drop for McpRunGuard {
    fn drop(&mut self) {
        self.0.revoke();
    }
}
