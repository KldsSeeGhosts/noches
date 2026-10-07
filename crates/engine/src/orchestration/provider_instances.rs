//! Persisted host-local instance configuration and runtime materialization.
//! Included as a child of the canonical catalog to keep its lock boundary local.
use super::*;
use std::path::Path;
use std::sync::Arc;
use zeron_harness::{Harness, HarnessError, instance::InstanceLaunch};
use zeron_proto::provider_instance::{ProviderInstanceConfig, ProviderInstanceConfigMap};
use zeron_proto::provider_settings::ProviderInstanceSettings;

fn error(message: impl Into<String>) -> HarnessError {
    HarnessError::Protocol(message.into())
}

fn slug(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_alphabetic()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

fn validate(id: &ProviderInstanceId, config: &ProviderInstanceConfig) -> Result<(), String> {
    if !slug(id.as_ref()) || !slug(config.driver.as_ref()) {
        return Err("invalid or duplicate provider instance id".into());
    }
    for variable in config.environment.as_ref().into_iter().flatten() {
        let name = &variable.name;
        if name.is_empty()
            || name.len() > 128
            || !(name.as_bytes()[0].is_ascii_alphabetic() || name.starts_with('_'))
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || name.starts_with("NOCHES_ORCHESTRATOR_")
            || name.starts_with("NOCHES_MCP_")
            || name.starts_with("NOCHES_SESSION_MCP_")
            || name.starts_with("T3_ACP_MCP_")
            || name.starts_with("NOCHES_ACP_MCP_")
        {
            return Err("Invalid provider environment variable name.".into());
        }
        if variable.value.contains('\0') {
            return Err("Provider environment values cannot contain NUL.".into());
        }
    }
    Ok(())
}

fn normalize(
    id: ProviderInstanceId,
    mut config: ProviderInstanceConfig,
) -> Result<(ProviderInstanceId, ProviderInstanceConfig), String> {
    // Generated envelope types retain opaque config but their string aliases
    // cannot express Effect's trim transforms. Mirror those at this boundary.
    let id = ProviderInstanceId::from(id.as_ref().trim());
    config.driver = ProviderDriverKind::from(config.driver.as_ref().trim());
    for field in [&mut config.display_name, &mut config.accent_color] {
        if let Optional::Present(value) = field {
            *value = value.trim().to_owned();
            if value.is_empty() {
                return Err("Provider display name and accent color must be nonempty.".into());
            }
        }
    }
    if let Optional::Present(environment) = &mut config.environment {
        for variable in environment {
            variable.name = variable.name.trim().to_owned();
        }
    }
    validate(&id, &config)?;
    Ok((id, config))
}

fn enabled(config: &ProviderInstanceConfig) -> bool {
    config.enabled.as_ref().copied().unwrap_or(true)
        && !config
            .config
            .as_ref()
            .is_some_and(|c| c["enabled"] == false)
}

fn harness_for_driver(driver: &ProviderDriverKind) -> Option<HarnessId> {
    if driver.as_ref() == "claudeAgent" {
        return Some(HarnessId::ClaudeCode);
    }
    serde_json::from_value(json!(driver.as_ref())).ok()
}

fn config_string<'a>(config: &'a ProviderInstanceConfig, key: &str) -> Option<&'a str> {
    config
        .config
        .as_ref()?
        .get(key)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
}

fn custom_models(config: &ProviderInstanceConfig) -> Vec<CatalogModel> {
    config
        .config
        .as_ref()
        .and_then(|c| c["customModels"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry.as_str().or_else(|| entry["slug"].as_str())?;
            if id.trim().is_empty() {
                return None;
            }
            Some(CatalogModel {
                id: id.into(),
                label: entry["name"].as_str().map(str::to_owned),
                options: entry["capabilities"]["optionDescriptors"]
                    .as_array()
                    .and_then(|v| serde_json::from_value(json!(v)).ok()),
                is_custom: true,
                reasoning_levels: Vec::new(),
                description: None,
                legacy_reasoning_option: None,
            })
        })
        .collect()
}

fn row(id: &ProviderInstanceId, config: &ProviderInstanceConfig) -> ProviderInstance {
    let harness = harness_for_driver(&config.driver);
    let problem = config_problem(config);
    ProviderInstance {
        provider_instance_id: id.clone(),
        driver_kind: config.driver.clone(),
        harness_id: harness,
        display_name: config
            .display_name
            .as_ref()
            .cloned()
            .or_else(|| Some(config.driver.to_string())),
        enabled: enabled(config),
        installed: false,
        authentication: Authentication::Unknown,
        adapter_registered: harness.is_some() && problem.is_none(),
        unavailable_reason: if let Some(problem) = problem {
            Some(format!("Invalid config for instance '{id}': {problem}"))
        } else if harness.is_none() {
            Some(format!(
                "Driver '{}' is not registered in this build.",
                config.driver
            ))
        } else {
            None
        },
        status: None,
        message: None,
        models: custom_models(config),
        model_count: 0,
    }
}

fn config_problem(config: &ProviderInstanceConfig) -> Option<String> {
    // Open drivers retain their opaque payload unchanged across downgrades.
    harness_for_driver(&config.driver)?;
    let value = config.config.as_ref()?;
    if value.is_null() {
        return None;
    }
    let Some(object) = value.as_object() else {
        return Some("expected an object.".into());
    };
    for key in [
        "binaryPath",
        "homePath",
        "shadowHomePath",
        "launchArgs",
        "serverUrl",
        "serverPassword",
    ] {
        if object.get(key).is_some_and(|v| !v.is_string()) {
            return Some(format!("{key} must be a string."));
        }
    }
    if object.get("enabled").is_some_and(|v| !v.is_boolean()) {
        return Some("enabled must be a boolean.".into());
    }
    if object.get("customModels").is_some_and(|v| !v.is_array()) {
        return Some("customModels must be an array.".into());
    }
    // Pi owns RPC mode and the native session: launch arguments that would
    // change either are rejected here, not at the first prompt.
    if config.driver.as_ref() == "pi"
        && let Some(args) = config_string(config, "launchArgs")
        && let Err(error) = zeron_harness::PiHarness::validate_launch_args(&tokenize(args))
    {
        return Some(error);
    }
    if config_string(config, "shadowHomePath").is_some() {
        return Some("Codex auth-overlay shadow homes are not supported by this build; use a private homePath.".into());
    }
    None
}

fn legacy_configs(rows: Vec<ProviderInstance>) -> Result<ProviderInstanceConfigMap, String> {
    rows.into_iter().map(|row| {
        let customs: Vec<_> = row.models.iter()
            .map(|m| json!({"slug":m.id,"name":m.label,"capabilities":{"optionDescriptors":m.options}})).collect();
        let mut value = json!({"driver":row.driver_kind,"enabled":row.enabled,"config":{"customModels":customs}});
        if let Some(name) = row.display_name { value["displayName"] = json!(name); }
        let config = serde_json::from_value(value).map_err(|e| e.to_string())?;
        Ok((row.provider_instance_id, config))
    }).collect()
}

/// Same quote/escape behavior as packages/shared/src/cliArgs.ts, no shell.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut quoted = false;
    let mut chars = input.trim().chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
                quoted = true;
            } else if ch == '\\'
                && q == '"'
                && chars
                    .peek()
                    .is_some_and(|c| ['"', '\\', '$', '`'].contains(c))
            {
                current.push(chars.next().unwrap());
            } else {
                current.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
            quoted = true;
        } else if ch.is_whitespace() {
            if !current.is_empty() || quoted {
                tokens.push(std::mem::take(&mut current));
                quoted = false;
            }
        } else if ch == '\\' && chars.peek().is_some_and(|c| c.is_whitespace()) {
            current.push(chars.next().unwrap());
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() || quoted {
        tokens.push(current);
    }
    tokens
}

fn expand(value: &str) -> String {
    crate::repos::expand_home(value).unwrap_or_else(|_| value.to_owned())
}

fn launch(id: &ProviderInstanceId, config: &ProviderInstanceConfig, root: &Path) -> InstanceLaunch {
    let mut environment = std::collections::BTreeMap::new();
    // Compatibility identities retain the user's current login. Newly-created
    // routing identities start private; no credentials are copied on duplicate.
    if id.as_ref() != config.driver.as_ref() {
        let home = root.join("provider-instances").join(id.as_ref());
        match harness_for_driver(&config.driver) {
            Some(HarnessId::Codex) => {
                environment.insert(
                    "CODEX_HOME".into(),
                    home.join("codex").display().to_string(),
                );
            }
            Some(HarnessId::ClaudeCode) => {
                // Preserve macOS keychain HOME, as T3's ClaudeHome.ts does.
                environment.insert(
                    "CLAUDE_CONFIG_DIR".into(),
                    home.join("claude").display().to_string(),
                );
            }
            _ => {
                environment.insert("HOME".into(), home.display().to_string());
                environment.insert("USERPROFILE".into(), home.display().to_string());
                for (name, path) in [
                    ("XDG_CONFIG_HOME", "config"),
                    ("XDG_DATA_HOME", "data"),
                    ("XDG_CACHE_HOME", "cache"),
                    ("GEMINI_HOME", "gemini"),
                    ("PI_CODING_AGENT_DIR", "pi"),
                    ("ZERON_CURSOR_STATE_DIR", "cursor-state"),
                ] {
                    environment.insert(name.into(), home.join(path).display().to_string());
                }
            }
        }
    }
    let mut secrets = Vec::new();
    for key in ["apiKey", "serverPassword"] {
        if let Some(value) = config_string(config, key) {
            secrets.push(value.to_owned());
        }
    }
    for variable in config.environment.as_ref().into_iter().flatten() {
        let value = if ["CODEX_HOME", "CLAUDE_CONFIG_DIR"].contains(&variable.name.as_str()) {
            expand(&variable.value)
        } else {
            variable.value.clone()
        };
        environment.insert(variable.name.clone(), value.clone());
        if variable.sensitive {
            secrets.push(value);
        }
    }
    if let Some(home) = config_string(config, "homePath") {
        let key = if config.driver.as_ref() == "codex" {
            "CODEX_HOME"
        } else if config.driver.as_ref() == "claudeAgent" {
            "CLAUDE_CONFIG_DIR"
        } else {
            "HOME"
        };
        environment.insert(key.into(), expand(home));
    }
    let args = tokenize(config_string(config, "launchArgs").unwrap_or_default());
    InstanceLaunch::new(environment, args).with_secrets(secrets)
}

fn executable_key(harness: HarnessId) -> &'static str {
    match harness {
        HarnessId::Codex => "CODEX_EXECUTABLE",
        HarnessId::ClaudeCode => "CLAUDE_CODE_EXECUTABLE",
        HarnessId::Cursor => "CURSOR_SDK_SHIM_EXECUTABLE",
        HarnessId::Grok => "GROK_EXECUTABLE",
        HarnessId::Devin => "DEVIN_EXECUTABLE",
        HarnessId::Hermes => "HERMES_EXECUTABLE",
        HarnessId::Pi => "PI_EXECUTABLE",
        HarnessId::Antigravity => "ANTIGRAVITY_ACP_EXECUTABLE",
        HarnessId::Opencode => "OPENCODE_EXECUTABLE",
        HarnessId::Mock => "",
    }
}

fn build(
    harness: HarnessId,
    config: &ProviderInstanceConfig,
    launch: InstanceLaunch,
) -> Arc<dyn Harness> {
    let executable_key = executable_key(harness);
    // Cursor binaryPath remains CLI compatibility metadata, as in T3. Its
    // backend is the SDK shim; only the explicit shim env seam replaces it.
    let binary = (harness != HarnessId::Cursor)
        .then(|| config_string(config, "binaryPath"))
        .flatten()
        .map(expand)
        .or_else(|| launch.environment.get(executable_key).cloned())
        .and_then(|p| zeron_harness::instance::resolve_binary(&p));
    macro_rules! native {
        ($value:expr) => {{
            let mut value = $value.with_instance_launch(launch);
            if let Some(binary) = binary {
                value = value.with_executable(binary);
            }
            Arc::new(value) as Arc<dyn Harness>
        }};
    }
    match harness {
        HarnessId::Codex => native!(zeron_harness::CodexHarness::new()),
        HarnessId::ClaudeCode => native!(zeron_harness::ClaudeHarness::new()),
        HarnessId::Cursor => native!(zeron_harness::CursorHarness::new()),
        HarnessId::Opencode => {
            let mut value = zeron_harness::OpencodeHarness::new().with_instance_launch(launch);
            if let Some(binary) = binary {
                value = value.with_executable(binary);
            }
            if let Some(url) = config_string(config, "serverUrl") {
                value = value.with_base_url(url);
                if let Some(password) = config_string(config, "serverPassword") {
                    value = value.with_server_password(password);
                }
            }
            Arc::new(value)
        }
        HarnessId::Grok => native!(zeron_harness::AcpHarness::grok()),
        HarnessId::Devin => native!(zeron_harness::AcpHarness::devin()),
        HarnessId::Hermes => native!(zeron_harness::AcpHarness::hermes()),
        HarnessId::Pi => native!(zeron_harness::PiHarness::new()),
        HarnessId::Antigravity => native!(zeron_harness::AcpHarness::antigravity()),
        HarnessId::Mock => Arc::new(zeron_harness::mock::MockHarness { script: Vec::new() }),
    }
}

fn persist(root: &Path, configs: &ProviderInstanceConfigMap) -> Result<(), String> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    serde_json::to_writer_pretty(&mut file, configs).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(root.join("provider-instances.json"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

impl ProviderInstanceRegistry {
    /// Dispatch and settings replacement serialize per identity, not per
    /// driver. A mutation cannot retire a new process admitted mid-reconcile.
    pub async fn lifecycle(&self, id: &ProviderInstanceId) -> tokio::sync::OwnedMutexGuard<()> {
        let gate = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .lifecycle_gates
            .entry(id.clone())
            .or_default()
            .clone();
        gate.lock_owned().await
    }

    pub fn load(&self, harnesses: &HarnessRegistry, root: &Path) -> Result<(), String> {
        let configs: ProviderInstanceConfigMap =
            match std::fs::read(root.join("provider-instances.json")) {
                Ok(bytes) => {
                    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    if value.is_array() {
                        let rows: Vec<ProviderInstance> =
                            serde_json::from_value(value).map_err(|e| e.to_string())?;
                        legacy_configs(rows)?
                    } else {
                        serde_json::from_value(value).map_err(|e| e.to_string())?
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let configured = self
                        .state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .configured
                        .clone();
                    if let Some(configured) = configured {
                        legacy_configs(configured)?
                    } else {
                        let descriptors = harnesses.descriptors();
                        let mock_rig = descriptors.iter().all(|d| d.id == HarnessId::Mock)
                            || std::env::var("COMET_HARNESS").is_ok_and(|h| h == "mock");
                        descriptors
                            .into_iter()
                            .filter(|d| {
                                crate::registry::descriptor_enabled(d)
                                    || (mock_rig && d.id == HarnessId::Mock)
                            })
                            .map(|d| {
                                let config = serde_json::from_value(json!({
                                "driver":legacy_driver(d.id),"displayName":d.name,"enabled":true,
                                "config":{"legacyCatalogImport":true}
                            }))
                            .expect("migration envelope");
                                (legacy_instance_id(d.id), config)
                            })
                            .collect()
                    }
                }
                Err(e) => return Err(e.to_string()),
            };
        let original_count = configs.len();
        let configs: ProviderInstanceConfigMap = configs
            .into_iter()
            .map(|(id, config)| normalize(id, config))
            .collect::<Result<_, _>>()?;
        if configs.len() != original_count {
            return Err("invalid or duplicate provider instance id".into());
        }
        persist(root, &configs)?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.data_dir = Some(root.to_owned());
        reconcile(&mut state, configs);
        self.changes.send_modify(|v| *v += 1);
        Ok(())
    }

    pub fn config(&self, id: &ProviderInstanceId) -> Option<ProviderInstanceConfig> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .configs
            .as_ref()?
            .get(id)
            .cloned()
    }

    pub(super) fn configured_installed(
        &self,
        state: &State,
        row: &ProviderInstance,
        detected: bool,
    ) -> bool {
        if row.driver_kind.as_ref() == "opencode"
            && state
                .configs
                .as_ref()
                .and_then(|c| c.get(&row.provider_instance_id))
                .is_some_and(|c| config_string(c, "serverUrl").is_some())
        {
            return true;
        }
        if let Some(config) = state
            .configs
            .as_ref()
            .and_then(|c| c.get(&row.provider_instance_id))
            && let Some(binary) = (row.driver_kind.as_ref() != "cursor")
                .then(|| config_string(config, "binaryPath"))
                .flatten()
                .or_else(|| {
                    config
                        .environment
                        .as_ref()?
                        .iter()
                        .rev()
                        .find(|v| Some(v.name.as_str()) == row.harness_id.map(executable_key))
                        .map(|v| v.value.as_str())
                })
        {
            return zeron_harness::instance::resolve_binary(&expand(binary)).is_some();
        }
        detected
    }

    /// Readiness refuses before runtime materialization. Discovery may probe
    /// signed-out instances but never a disabled/missing/invalid driver.
    pub fn resolve_runtime(
        &self,
        harnesses: &HarnessRegistry,
        id: &ProviderInstanceId,
        require_auth: bool,
    ) -> Result<Arc<dyn Harness>, HarnessError> {
        let row = self
            .snapshot(harnesses)
            .into_iter()
            .find(|p| &p.provider_instance_id == id)
            .ok_or_else(|| error("Provider is absent from the live catalog."))?;
        let constraints: Vec<_> = row
            .constraints()
            .into_iter()
            .filter(|c| require_auth || c != "Provider is not authenticated.")
            .collect();
        if !constraints.is_empty() {
            return Err(error(constraints.join(" ")));
        }
        let harness = row.harness_id.ok_or_else(|| error("Missing adapter."))?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(runtime) = state.runtimes.get(id) {
            return Ok(runtime.clone());
        }
        let config = state.configs.as_ref().and_then(|c| c.get(id));
        let runtime = if let Some(config) = config {
            let root = state
                .data_dir
                .as_deref()
                .ok_or_else(|| error("Provider data directory missing."))?;
            let overrides = launch(id, config, root);
            // Only canonical compatibility slots without launch overrides may
            // reuse a registered fixture/legacy adapter.
            if harness == HarnessId::Mock
                || (overrides.environment.is_empty()
                    && overrides.args.is_empty()
                    && config_string(config, "binaryPath").is_none()
                    && config_string(config, "serverUrl").is_none())
            {
                harnesses.resolve(harness)?
            } else {
                for (name, value) in &overrides.environment {
                    if [
                        "CODEX_HOME",
                        "CLAUDE_CONFIG_DIR",
                        "HOME",
                        "USERPROFILE",
                        "PI_CODING_AGENT_DIR",
                        "GEMINI_HOME",
                        "ZERON_CURSOR_STATE_DIR",
                        "XDG_CONFIG_HOME",
                        "XDG_DATA_HOME",
                        "XDG_CACHE_HOME",
                    ]
                    .contains(&name.as_str())
                        && !value.is_empty()
                    {
                        std::fs::create_dir_all(value)?;
                    }
                }
                build(harness, config, overrides)
            }
        } else {
            harnesses.resolve(harness)?
        };
        state.runtimes.insert(id.clone(), runtime.clone());
        Ok(runtime)
    }

    /// Serialized, durable full-envelope update. Redacted sensitive values are
    /// restored by name (last-value-wins), not by row position.
    pub fn write_instance(
        &self,
        id: ProviderInstanceId,
        config: ProviderInstanceConfig,
        create: bool,
    ) -> Result<(), String> {
        let (id, mut config) = normalize(id, config)?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut configs = state
            .configs
            .clone()
            .ok_or("Provider settings are not initialized.")?;
        if create && configs.contains_key(&id) {
            return Err("Provider instance already exists.".into());
        }
        if !create && !configs.contains_key(&id) {
            return Err("Provider instance does not exist.".into());
        }
        if let Optional::Present(env) = &mut config.environment {
            for variable in env {
                if variable.sensitive && variable.value_redacted.as_ref() == Some(&true) {
                    variable.value = configs
                        .get(&id)
                        .and_then(|c| c.environment.as_ref())
                        .and_then(|env| {
                            env.iter()
                                .rev()
                                .find(|v| v.name == variable.name && v.sensitive)
                        })
                        .map(|v| v.value.clone())
                        .ok_or("Redacted environment value has no stored secret.")?;
                }
                variable.value_redacted = Optional::Absent;
            }
        }
        if let Optional::Present(Value::Object(object)) = &mut config.config {
            for key in ["apiKey", "serverPassword"] {
                if object.get(key).and_then(Value::as_str) == Some("[redacted]") {
                    let previous = configs
                        .get(&id)
                        .and_then(|c| c.config.as_ref())
                        .and_then(|c| c.get(key))
                        .cloned()
                        .ok_or("Redacted config value has no stored secret.")?;
                    object.insert(key.into(), previous);
                }
            }
        }
        configs.insert(id, config);
        persist(
            state
                .data_dir
                .as_deref()
                .ok_or("Provider data directory missing.")?,
            &configs,
        )?;
        reconcile(&mut state, configs);
        self.changes.send_modify(|v| *v += 1);
        Ok(())
    }

    pub fn delete_instance(&self, id: &ProviderInstanceId) -> Result<(), String> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut configs = state
            .configs
            .clone()
            .ok_or("Provider settings are not initialized.")?;
        if configs.remove(id).is_none() {
            return Err("Provider instance does not exist.".into());
        }
        persist(
            state
                .data_dir
                .as_deref()
                .ok_or("Provider data directory missing.")?,
            &configs,
        )?;
        reconcile(&mut state, configs);
        self.changes.send_modify(|v| *v += 1);
        Ok(())
    }

    pub fn duplicate_instance(
        &self,
        source: &ProviderInstanceId,
        target: ProviderInstanceId,
        name: Option<String>,
    ) -> Result<(), String> {
        let mut config = self
            .config(source)
            .ok_or("Provider instance does not exist.")?;
        config.display_name = name.map(Optional::Present).unwrap_or(config.display_name);
        // A duplicate is a fresh login, never an alias of the source auth home.
        if let Optional::Present(value) = &mut config.config
            && let Some(value) = value.as_object_mut()
        {
            value.remove("homePath");
            value.remove("shadowHomePath");
            value.remove("legacyCatalogImport");
        }
        if let Optional::Present(env) = &mut config.environment {
            env.retain(|v| {
                ![
                    "CODEX_HOME",
                    "CLAUDE_CONFIG_DIR",
                    "HOME",
                    "USERPROFILE",
                    "GEMINI_HOME",
                    "PI_CODING_AGENT_DIR",
                    "ZERON_CURSOR_STATE_DIR",
                    "XDG_CONFIG_HOME",
                    "XDG_DATA_HOME",
                    "XDG_CACHE_HOME",
                ]
                .contains(&v.name.as_str())
            });
        }
        self.write_instance(target, config, true)
    }
}

fn reconcile(state: &mut State, configs: ProviderInstanceConfigMap) {
    let unchanged: HashSet<_> = configs
        .iter()
        .filter(|(id, config)| state.configs.as_ref().and_then(|c| c.get(*id)) == Some(*config))
        .map(|(id, _)| id.clone())
        .collect();
    state.runtimes.retain(|id, _| unchanged.contains(id));
    state.discovered.retain(|id, _| unchanged.contains(id));
    state
        .instance_authentication
        .retain(|id, _| unchanged.contains(id));
    state.configured = Some(configs.iter().map(|(id, c)| row(id, c)).collect());
    state.configs = Some(configs);
}

pub(super) fn settings_rows(
    registry: &ProviderInstanceRegistry,
    harnesses: &HarnessRegistry,
) -> Vec<ProviderInstanceSettings> {
    registry
        .snapshot(harnesses)
        .into_iter()
        .filter_map(|row| {
            let mut instance = registry.config(&row.provider_instance_id)?;
            if let Optional::Present(env) = &mut instance.environment {
                for variable in env {
                    if variable.sensitive {
                        let present = !variable.value.is_empty();
                        variable.value.clear();
                        variable.value_redacted = if present {
                            Optional::Present(true)
                        } else {
                            Optional::Absent
                        };
                    } else {
                        variable.value_redacted = Optional::Absent;
                    }
                }
            }
            // Driver config can also contain passwords/API keys.
            if let Optional::Present(value) = &mut instance.config
                && let Some(value) = value.as_object_mut()
            {
                for name in ["apiKey", "serverPassword"] {
                    if value
                        .get(name)
                        .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                    {
                        value.insert(name.into(), json!("[redacted]"));
                    }
                }
            }
            Some(ProviderInstanceSettings {
                instance_id: row.provider_instance_id.clone(),
                instance,
                authentication: serde_json::to_value(&row.authentication)
                    .ok()?
                    .as_str()?
                    .into(),
                model_count: row.models.len(),
                constraints: row.constraints(),
            })
        })
        .collect()
}

impl ProviderInstanceRegistry {
    pub fn settings(&self, harnesses: &HarnessRegistry) -> Vec<ProviderInstanceSettings> {
        settings_rows(self, harnesses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessDescriptor;

    fn registry() -> HarnessRegistry {
        let registry = HarnessRegistry::new();
        registry.register(Arc::new(zeron_harness::mock::MockHarness {
            script: Vec::new(),
        }));
        registry
    }

    fn config(driver: &str) -> ProviderInstanceConfig {
        serde_json::from_value(json!({"driver":driver, "enabled":true})).unwrap()
    }

    #[test]
    fn migration_uses_exact_legacy_ids_once_and_never_resurrects_removed_instances() {
        let registry = HarnessRegistry::new();
        for id in [HarnessId::Codex, HarnessId::ClaudeCode, HarnessId::Pi] {
            registry.register_lazy(
                HarnessDescriptor {
                    id,
                    name: format!("{id:?}"),
                    supports_steering: false,
                    steering_mode: zeron_proto::SteeringMode::TurnBoundary,
                    reasoning_levels: Vec::new(),
                    installed: true,
                    enabled: None,
                },
                Box::new(|| true),
                Box::new(|| {
                    Ok(Arc::new(zeron_harness::mock::MockHarness {
                        script: Vec::new(),
                    }))
                }),
            );
        }
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("harness-prefs.json"),
            r#"{"disabled":["pi"]}"#,
        )
        .unwrap();
        registry.load_prefs(root.path());
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        assert_eq!(
            catalog
                .settings(&registry)
                .iter()
                .map(|r| r.instance_id.as_ref())
                .collect::<Vec<_>>(),
            ["claudeAgent", "codex"]
        );
        catalog.delete_instance(&"codex".into()).unwrap();
        catalog.load(&registry, root.path()).unwrap();
        assert!(catalog.config(&"codex".into()).is_none());
        catalog.delete_instance(&"claudeAgent".into()).unwrap();
        catalog.load(&registry, root.path()).unwrap();
        assert!(catalog.snapshot(&registry).is_empty());
    }

    #[test]
    fn explicit_configs_override_legacy_enablement_and_both_disable_flags_win() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let mut a = config("mock");
        a.config = Optional::Present(json!({"enabled":false}));
        catalog.write_instance("a".into(), a, true).unwrap();
        catalog
            .write_instance("b".into(), config("mock"), true)
            .unwrap();
        let rows = catalog.snapshot(&registry);
        assert_eq!(
            rows.iter()
                .find(|r| r.provider_instance_id.as_ref() == "a")
                .unwrap()
                .constraints(),
            ["Provider instance is disabled."]
        );
        assert!(
            rows.iter()
                .find(|r| r.provider_instance_id.as_ref() == "b")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn secrets_redact_round_trip_by_name_and_writes_are_durable_and_private() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let secret: ProviderInstanceConfig = serde_json::from_value(json!({
            "driver":"mock", "config":{"apiKey":"private-config-secret"},
            "environment":[{"name":"API_KEY","value":"first","sensitive":true},
                {"name":"API_KEY","value":"second-secret","sensitive":true},
                {"name":"MODEL","value":"cpa/exact"}]
        }))
        .unwrap();
        catalog
            .write_instance("private".into(), secret, true)
            .unwrap();
        let mut public = catalog
            .settings(&registry)
            .into_iter()
            .find(|r| r.instance_id.as_ref() == "private")
            .unwrap()
            .instance;
        let serialized = serde_json::to_string(&public).unwrap();
        assert!(
            !serialized.contains("second-secret") && !serialized.contains("private-config-secret")
        );
        assert!(serialized.contains("valueRedacted"));
        public.environment = Optional::Present(
            public
                .environment
                .as_ref()
                .unwrap()
                .iter()
                .rev()
                .cloned()
                .collect(),
        );
        catalog
            .write_instance("private".into(), public, false)
            .unwrap();
        let saved = catalog.config(&"private".into()).unwrap();
        assert_eq!(
            saved.environment.as_ref().unwrap()[1].value,
            "second-secret"
        );
        assert_eq!(
            saved.config.as_ref().unwrap()["apiKey"],
            "private-config-secret"
        );
        catalog.load(&registry, root.path()).unwrap();
        assert_eq!(catalog.config(&"private".into()).unwrap(), saved);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(root.path().join("provider-instances.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn independent_launch_environment_home_and_quote_semantics_match_t3() {
        let root = tempfile::tempdir().unwrap();
        let cfg: ProviderInstanceConfig = serde_json::from_value(json!({
            "driver":"codex","environment":[
                {"name":"PROXY_MODEL","value":"$MODEL"},
                {"name":"PROXY_MODEL","value":"cpa/exact/model"},
                {"name":"CODEX_HOME","value":"~/custom-codex"}],
            "config":{"launchArgs":"-c 'model_provider=\"cpa\"' --flag \"two words\" \"\""}
        }))
        .unwrap();
        let profile = launch(&"proxy".into(), &cfg, root.path());
        assert_eq!(profile.environment["CODEX_HOME"], expand("~/custom-codex"));
        assert_eq!(profile.environment["PROXY_MODEL"], "cpa/exact/model");
        assert_eq!(
            profile.args,
            ["-c", "model_provider=\"cpa\"", "--flag", "two words", ""]
        );
        let first = launch(&"one".into(), &config("codex"), root.path());
        let second = launch(&"two".into(), &config("codex"), root.path());
        assert_ne!(
            first.environment["CODEX_HOME"],
            second.environment["CODEX_HOME"]
        );
        assert!(
            launch(&"codex".into(), &config("codex"), root.path())
                .environment
                .is_empty()
        );
        let claude = launch(&"claude_work".into(), &config("claudeAgent"), root.path());
        assert!(claude.environment.contains_key("CLAUDE_CONFIG_DIR"));
        assert!(
            !claude.environment.contains_key("HOME"),
            "T3 preserves the macOS keychain HOME"
        );
    }

    #[test]
    fn unknown_and_invalid_driver_configs_survive_and_refuse_before_launch() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let unknown: ProviderInstanceConfig = serde_json::from_value(json!({
            "driver":"futureDriver","config":{"opaque":[1,2,3]}
        }))
        .unwrap();
        catalog
            .write_instance("future".into(), unknown.clone(), true)
            .unwrap();
        let invalid: ProviderInstanceConfig = serde_json::from_value(json!({
            "driver":"codex","config":{"homePath":5}
        }))
        .unwrap();
        catalog
            .write_instance("broken".into(), invalid, true)
            .unwrap();
        let rows = catalog.snapshot(&registry);
        assert!(
            rows.iter()
                .find(|r| r.provider_instance_id.as_ref() == "future")
                .unwrap()
                .constraints()
                .iter()
                .any(|c| c == "Driver 'futureDriver' is not registered in this build.")
        );
        assert!(
            catalog
                .resolve_runtime(&registry, &"broken".into(), true)
                .is_err()
        );
        catalog.load(&registry, root.path()).unwrap();
        assert_eq!(catalog.config(&"future".into()).unwrap(), unknown);
    }

    #[test]
    fn pi_launch_args_are_validated_when_the_instance_is_written() {
        let registry = HarnessRegistry::new();
        registry.register_lazy(
            HarnessDescriptor {
                id: HarnessId::Pi,
                name: "Pi".into(),
                supports_steering: true,
                steering_mode: zeron_proto::SteeringMode::StepBoundary,
                reasoning_levels: Vec::new(),
                installed: true,
                enabled: None,
            },
            Box::new(|| true),
            Box::new(|| Ok(Arc::new(zeron_harness::PiHarness::new()))),
        );
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let config = |value: Value| -> ProviderInstanceConfig {
            serde_json::from_value(json!({"driver":"pi","config":value})).unwrap()
        };
        // The binary and launch resources are configurable...
        let binary = root.path().join(if cfg!(windows) { "pi.exe" } else { "pi" });
        std::fs::write(&binary, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        catalog
            .write_instance(
                "pi-ok".into(),
                config(json!({"binaryPath":binary,
                    "launchArgs":"--provider cpa --model gemini-3.8-flash --no-skills --my-ext-flag v"})),
                true,
            )
            .unwrap();
        // ...but Noches owns RPC mode and the native session.
        for (name, args, reason) in [
            ("pi-session", "--session /tmp/x.jsonl", "'--session' is controlled by Noches"),
            ("pi-mode", "--mode text", "'--mode' is controlled by Noches"),
            ("pi-prompt", "write a poem", "positional prompt"),
            ("pi-provider", "--provider openrouter", "'--provider' requires '--model'"),
        ] {
            let result = catalog.write_instance(
                name.into(),
                config(json!({"launchArgs":args})),
                true,
            );
            let rows = catalog.snapshot(&registry);
            let constrained = rows
                .iter()
                .find(|r| r.provider_instance_id.as_ref() == name)
                .map(|r| r.constraints().join(" "))
                .unwrap_or_default();
            assert!(
                result.is_err() || constrained.contains(reason),
                "{name}: {result:?} {constrained}"
            );
            assert!(
                result
                    .as_ref()
                    .err()
                    .map(|e| e.to_string().contains(reason))
                    .unwrap_or(true),
                "{name}: {result:?}"
            );
        }
        assert!(
            catalog
                .resolve_runtime(&registry, &"pi-ok".into(), true)
                .is_ok()
        );
    }

    #[test]
    fn executable_environment_override_supplies_instance_installation_readiness() {
        let registry = HarnessRegistry::new();
        registry.register_lazy(
            HarnessDescriptor {
                id: HarnessId::Codex,
                name: "Codex".into(),
                supports_steering: false,
                steering_mode: zeron_proto::SteeringMode::TurnBoundary,
                reasoning_levels: Vec::new(),
                installed: false,
                enabled: Some(false),
            },
            Box::new(|| false),
            Box::new(|| Err(HarnessError::NotInstalled("codex".into()))),
        );
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let binary = std::env::current_exe().unwrap();
        let config = serde_json::from_value(json!({
            "driver":"codex","environment":[{"name":"CODEX_EXECUTABLE","value":binary}]
        }))
        .unwrap();
        catalog
            .write_instance("custom".into(), config, true)
            .unwrap();
        assert!(catalog.snapshot(&registry)[0].installed);
        assert!(
            catalog
                .resolve_runtime(&registry, &"custom".into(), false)
                .is_ok()
        );
    }

    #[test]
    fn invalid_duplicate_and_environment_updates_have_no_side_effects() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let before = std::fs::read(root.path().join("provider-instances.json")).unwrap();
        assert!(
            catalog
                .write_instance("mock".into(), config("mock"), true)
                .is_err()
        );
        assert!(
            catalog
                .write_instance("../bad".into(), config("mock"), true)
                .is_err()
        );
        let mut cfg = config("mock");
        cfg.environment = Optional::Present(
            serde_json::from_value(json!([
                {"name":"NOCHES_SESSION_MCP_ENTRIES","value":"override"}
            ]))
            .unwrap(),
        );
        assert!(catalog.write_instance("bad".into(), cfg, true).is_err());
        assert_eq!(
            std::fs::read(root.path().join("provider-instances.json")).unwrap(),
            before
        );
    }

    #[test]
    fn config_envelope_uses_t3_schema_trimming_and_nonempty_validation() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let config = serde_json::from_value(json!({
            "driver":" mock ","displayName":" Personal ",
            "environment":[{"name":" MODEL ","value":"exact/model"}]
        }))
        .unwrap();
        catalog
            .write_instance(" personal ".into(), config, true)
            .unwrap();
        let mut saved = catalog.config(&"personal".into()).unwrap();
        assert_eq!(saved.driver.as_ref(), "mock");
        assert_eq!(saved.display_name.as_ref().unwrap(), "Personal");
        assert_eq!(saved.environment.as_ref().unwrap()[0].name, "MODEL");
        saved.display_name = Optional::Present(" ".into());
        assert!(
            catalog
                .write_instance("personal".into(), saved, false)
                .is_err()
        );
        assert_eq!(
            catalog
                .config(&"personal".into())
                .unwrap()
                .display_name
                .as_ref()
                .unwrap(),
            "Personal"
        );
    }

    #[test]
    fn duplicate_has_a_new_auth_home_and_unchanged_runtime_caches_survive_updates() {
        let registry = registry();
        let root = tempfile::tempdir().unwrap();
        let catalog = &registry.provider_instances;
        catalog.load(&registry, root.path()).unwrap();
        let mut source = config("mock");
        source.config =
            Optional::Present(json!({"homePath":"/shared","customModels":["cpa/exact"]}));
        catalog
            .write_instance("source".into(), source.clone(), true)
            .unwrap();
        catalog
            .duplicate_instance(&"source".into(), "copy".into(), Some("Copy".into()))
            .unwrap();
        assert!(
            catalog
                .config(&"copy".into())
                .unwrap()
                .config
                .as_ref()
                .unwrap()
                .get("homePath")
                .is_none()
        );
        let runtime = catalog
            .resolve_runtime(&registry, &"copy".into(), true)
            .unwrap();
        source.display_name = Optional::Present("Changed".into());
        catalog
            .write_instance("source".into(), source, false)
            .unwrap();
        assert!(Arc::ptr_eq(
            &runtime,
            &catalog
                .resolve_runtime(&registry, &"copy".into(), true)
                .unwrap()
        ));
    }
}
