//! Host-local canonical instance/model catalog. HarnessId is a driver/branding
//! compatibility key, not an instance identity. This file contains no secrets.
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::watch;
use zeron_proto::orchestration::{Optional, normalize_contract};
use zeron_proto::provider_instance::{
    ModelSelection, OrchestratorMcpProviderCapability, OrchestratorMcpProviderCapabilityModelsItem,
    ProviderDriverKind, ProviderInstanceId, ProviderOptionDescriptor,
};
use zeron_proto::{HarnessId, Model, ModelOption, ModelOptionChoice};

use crate::HarnessRegistry;
use crate::orchestration::service::ToolError;

#[path = "orchestration/provider_instances.rs"]
mod settings;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authentication {
    Authenticated,
    Unauthenticated,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub id: String,
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ProviderOptionDescriptor>>,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub reasoning_levels: Vec<zeron_proto::ReasoningLevel>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_reasoning_option: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInstance {
    pub provider_instance_id: ProviderInstanceId,
    pub driver_kind: ProviderDriverKind,
    /// None retains an unavailable shadow for a driver absent from this build.
    pub harness_id: Option<HarnessId>,
    pub display_name: Option<String>,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub authentication: Authentication,
    #[serde(default)]
    pub adapter_registered: bool,
    #[serde(default)]
    pub unavailable_reason: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub models: Vec<CatalogModel>,
    #[serde(default)]
    pub model_count: usize,
}

fn yes() -> bool {
    true
}

impl ProviderInstance {
    pub fn constraints(&self) -> Vec<String> {
        let mut result = Vec::new();
        if !self.adapter_registered {
            result.push("No V2 provider adapter is registered.".into());
        }
        if !self.enabled {
            result.push("Provider instance is disabled.".into());
        }
        if !self.installed {
            result.push("Provider executable is not installed.".into());
        }
        if let Some(reason) = &self.unavailable_reason {
            result.push(reason.clone());
        }
        if let Some(status @ ("error" | "disabled")) = self.status.as_deref() {
            result.push(
                self.message
                    .clone()
                    .unwrap_or_else(|| format!("Provider status is {status}.")),
            );
        }
        if self.authentication == Authentication::Unauthenticated {
            result.push("Provider is not authenticated.".into());
        }
        result
    }

    pub fn capability(&self) -> OrchestratorMcpProviderCapability {
        let constraints = self.constraints();
        OrchestratorMcpProviderCapability {
            provider_instance_id: self.provider_instance_id.clone(),
            driver_kind: self.driver_kind.clone(),
            display_name: self.display_name.clone(),
            models: self
                .models
                .iter()
                .map(|m| OrchestratorMcpProviderCapabilityModelsItem {
                    id: m.id.clone(),
                    label: m.label.clone(),
                    options: m.options.clone().map(Optional::Present).unwrap_or_default(),
                })
                .collect(),
            can_run_child_task: constraints.is_empty(),
            can_run_cross_provider_child_task: constraints.is_empty(),
            constraints,
        }
    }
}

#[derive(Default)]
struct State {
    configured: Option<Vec<ProviderInstance>>,
    discovered: HashMap<ProviderInstanceId, Vec<CatalogModel>>,
    snapshot: Vec<ProviderInstance>,
    authentication: HashMap<HarnessId, Authentication>,
    instance_authentication: HashMap<ProviderInstanceId, Authentication>,
    configs: Option<zeron_proto::provider_instance::ProviderInstanceConfigMap>,
    data_dir: Option<std::path::PathBuf>,
    runtimes: HashMap<ProviderInstanceId, std::sync::Arc<dyn zeron_harness::Harness>>,
    lifecycle_gates: HashMap<ProviderInstanceId, std::sync::Arc<tokio::sync::Mutex<()>>>,
}

/// The watch receiver subscribes before reading its snapshot (no event gap).
pub struct ProviderInstanceRegistry {
    state: Mutex<State>,
    changes: watch::Sender<u64>,
}

impl Default for ProviderInstanceRegistry {
    fn default() -> Self {
        Self {
            state: Mutex::default(),
            changes: watch::channel(0).0,
        }
    }
}

impl ProviderInstanceRegistry {
    pub fn apply_accounts(&self, snapshot: &zeron_proto::AgentAccountsSnapshot) {
        for id in [
            HarnessId::ClaudeCode,
            HarnessId::Codex,
            HarnessId::Cursor,
            HarnessId::Grok,
            HarnessId::Devin,
            HarnessId::Hermes,
            HarnessId::Pi,
            HarnessId::Opencode,
            HarnessId::Antigravity,
        ] {
            let authentication = if snapshot
                .accounts
                .iter()
                .any(|a| a.harness == id && a.active)
            {
                Authentication::Authenticated
            } else if snapshot
                .warnings
                .iter()
                .any(|warning| warning.harness == id)
            {
                Authentication::Unknown
            } else if id == HarnessId::ClaudeCode {
                // OAuth slots do not enumerate API-key-backed Claude logins.
                Authentication::Unknown
            } else {
                Authentication::Unauthenticated
            };
            self.set_authentication(id, authentication);
        }
    }
    /// Auth/account discovery supplies readiness, never a secret. A configured
    /// instance can override this for its own independent login.
    pub fn set_authentication(&self, harness: HarnessId, authentication: Authentication) {
        let changed = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .authentication
            .insert(harness, authentication.clone())
            != Some(authentication);
        if changed {
            self.changes.send_modify(|version| *version += 1);
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }

    /// Atomically reconcile a complete settings-author-ordered inventory.
    /// Removed custom rows are never resurrected from the discovery cache.
    pub fn configure(&self, instances: Vec<ProviderInstance>) -> Result<(), String> {
        let mut seen = HashSet::new();
        let valid_slug = |id: &str| {
            !id.is_empty()
                && id.len() <= 64
                && id.as_bytes()[0].is_ascii_alphabetic()
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        };
        for instance in &instances {
            let id = instance.provider_instance_id.as_ref();
            if !valid_slug(id) || !valid_slug(instance.driver_kind.as_ref()) || !seen.insert(id) {
                return Err("invalid or duplicate provider instance id".into());
            }
        }
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .configured = Some(instances);
        self.changes.send_modify(|version| *version += 1);
        Ok(())
    }

    pub fn snapshot(&self, harnesses: &HarnessRegistry) -> Vec<ProviderInstance> {
        let descriptors = harnesses.descriptors();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut instances = if state.configured.is_none() {
            descriptors
                .iter()
                .map(|d| ProviderInstance {
                    provider_instance_id: legacy_instance_id(d.id),
                    driver_kind: legacy_driver(d.id),
                    harness_id: Some(d.id),
                    display_name: Some(d.name.clone()),
                    enabled: d.enabled.unwrap_or(d.installed) || d.id == HarnessId::Mock,
                    installed: d.installed,
                    authentication: state.authentication.get(&d.id).cloned().unwrap_or_default(),
                    // The runner bridge can replace this with actual adapter readiness.
                    adapter_registered: true,
                    unavailable_reason: None,
                    status: None,
                    message: None,
                    models: Vec::new(),
                    model_count: 0,
                })
                .collect()
        } else {
            state.configured.clone().unwrap_or_default()
        };
        for instance in &mut instances {
            if let Some(id) = instance.harness_id {
                if instance.authentication == Authentication::Unknown {
                    instance.authentication = state
                        .instance_authentication
                        .get(&instance.provider_instance_id)
                        .cloned()
                        .or_else(|| {
                            (state.configs.is_none()
                                || (instance.provider_instance_id == legacy_instance_id(id)
                                    && state
                                        .configs
                                        .as_ref()
                                        .and_then(|c| c.get(&instance.provider_instance_id))
                                        .is_none_or(|c| {
                                            c.environment.as_ref().is_none_or(Vec::is_empty)
                                                && c.config.as_ref().is_none_or(|v| {
                                                    v["homePath"].as_str().is_none_or(str::is_empty)
                                                        && v["shadowHomePath"]
                                                            .as_str()
                                                            .is_none_or(str::is_empty)
                                                })
                                        })))
                            .then(|| state.authentication.get(&id).cloned())
                            .flatten()
                        })
                        .unwrap_or_default();
                }
                if let Some(d) = descriptors.iter().find(|d| d.id == id) {
                    if id != HarnessId::Mock && instance.driver_kind != legacy_driver(id) {
                        instance.adapter_registered = false;
                    }
                    // Explicit instances are independent of the legacy toggle.
                    instance.installed = self.configured_installed(&state, instance, d.installed);
                } else {
                    instance.adapter_registered = false;
                    instance.installed = false;
                }
                if let Some(models) = state.discovered.get(&instance.provider_instance_id) {
                    let configured: HashSet<_> =
                        instance.models.iter().map(|m| m.id.clone()).collect();
                    instance.models.extend(
                        models
                            .iter()
                            .filter(|m| !configured.contains(&m.id))
                            .cloned(),
                    );
                }
            } else {
                instance.adapter_registered = false;
            }
            instance.model_count = instance.models.len();
        }
        // Only publish semantic catalog changes, never credential/turn traffic.
        if serde_json::to_value(&state.snapshot).ok() != serde_json::to_value(&instances).ok() {
            state.snapshot = instances.clone();
            self.changes.send_modify(|version| *version += 1);
        }
        instances
    }

    pub async fn refresh(
        &self,
        harnesses: &HarnessRegistry,
        id: HarnessId,
    ) -> Result<Vec<Model>, zeron_harness::HarnessError> {
        let instance = self
            .snapshot(harnesses)
            .into_iter()
            .find(|instance| instance.provider_instance_id == legacy_instance_id(id))
            .ok_or_else(|| {
                zeron_harness::HarnessError::Protocol(
                    "Provider is absent from the live catalog.".into(),
                )
            })?;
        self.refresh_instance(harnesses, &instance.provider_instance_id)
            .await
    }

    pub async fn refresh_instance(
        &self,
        harnesses: &HarnessRegistry,
        instance_id: &ProviderInstanceId,
    ) -> Result<Vec<Model>, zeron_harness::HarnessError> {
        let harness = self.resolve_runtime(harnesses, instance_id, false)?;
        let id = harness.id();
        let config = self.config(instance_id);
        let models = harness.models().await?;
        let auth = harness.authenticated().await.ok().flatten();
        let enrolled = config.as_ref().is_some_and(|c| {
            c.config
                .as_ref()
                .is_some_and(|v| v["legacyCatalogImport"] == true)
        }) || self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .configured
            .is_none();
        let custom = if enrolled {
            enrolled_models(id)
        } else {
            vec![]
        };
        let mut models: Vec<_> = models
            .into_iter()
            .map(|model| from_legacy_model(id, model))
            .collect();
        for model in custom {
            if let Some(existing) = models.iter_mut().find(|entry| entry.id == model.id) {
                *existing = model;
            } else {
                models.push(model);
            }
        }
        {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            // An in-flight discovery cannot resurrect a removed/reconfigured row.
            if state
                .configs
                .as_ref()
                .and_then(|c| c.get(instance_id))
                .cloned()
                != config
            {
                return Err(zeron_harness::HarnessError::Protocol(
                    "Provider settings changed during discovery; retry.".into(),
                ));
            }
            if state
                .runtimes
                .get(instance_id)
                .is_none_or(|runtime| !std::sync::Arc::ptr_eq(runtime, &harness))
            {
                return Err(zeron_harness::HarnessError::Protocol(
                    "Provider settings changed during discovery; retry.".into(),
                ));
            }
            state.discovered.insert(instance_id.clone(), models);
            if let Some(authenticated) = auth {
                state.instance_authentication.insert(
                    instance_id.clone(),
                    if authenticated {
                        Authentication::Authenticated
                    } else {
                        Authentication::Unauthenticated
                    },
                );
            }
        }
        let snapshot = self.snapshot(harnesses);
        Ok(snapshot
            .into_iter()
            .find(|p| &p.provider_instance_id == instance_id)
            .map(|p| p.models.into_iter().map(to_legacy_model).collect())
            .unwrap_or_default())
    }

    pub async fn refresh_all(&self, harnesses: &HarnessRegistry) {
        let ids: Vec<_> = self
            .snapshot(harnesses)
            .iter()
            .filter(|p| p.enabled && p.installed)
            .map(|p| p.provider_instance_id.clone())
            .collect();
        // Driver discovery may contact a process. No registry mutex crosses it.
        futures::future::join_all(ids.into_iter().map(|id| async move {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                self.refresh_instance(harnesses, &id),
            )
            .await;
        }))
        .await;
    }

    pub fn resolve_target(
        &self,
        harnesses: &HarnessRegistry,
        inherited: &ModelSelection,
        target: Option<&Value>,
    ) -> Result<ModelSelection, ToolError> {
        use zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode as Code;
        let fail = |code, message| ToolError { code, message };
        let providers = self.snapshot(harnesses);
        let requested_id = target.and_then(|t| t["providerInstanceId"].as_str());
        let driver = target.and_then(|t| t["driverKind"].as_str());
        let instance_id = if let Some(id) = requested_id {
            id.to_owned()
        } else if let Some(driver) = driver {
            let candidates: Vec<_> = providers
                .iter()
                .filter(|p| p.driver_kind.as_ref() == driver && p.adapter_registered)
                .collect();
            if candidates.is_empty() {
                return Err(fail(
                    Code::ProviderUnavailable,
                    format!("No V2 provider adapter is registered for driver {driver}."),
                ));
            }
            candidates
                .iter()
                .find(|p| {
                    p.provider_instance_id == inherited.instance_id && p.constraints().is_empty()
                })
                .or_else(|| candidates.iter().find(|p| p.constraints().is_empty()))
                .ok_or_else(|| {
                    fail(
                        Code::ProviderUnavailable,
                        format!("No available V2 provider instance for driver {driver}."),
                    )
                })?
                .provider_instance_id
                .0
                .clone()
        } else {
            inherited.instance_id.0.clone()
        };
        let provider = providers
            .iter()
            .find(|p| p.provider_instance_id.as_ref() == instance_id)
            .ok_or_else(|| {
                fail(
                    Code::ProviderUnavailable,
                    format!("Provider instance {instance_id} is not registered."),
                )
            })?;
        if let Some(driver) = driver
            && provider.driver_kind.as_ref() != driver
        {
            return Err(fail(
                Code::InvalidRequest,
                format!(
                    "Provider instance {instance_id} uses driver {}, not {driver}.",
                    provider.driver_kind
                ),
            ));
        }
        let constraints = provider.constraints();
        if !constraints.is_empty() {
            return Err(fail(
                Code::ProviderUnavailable,
                format!(
                    "Provider {instance_id} cannot run a child task: {}",
                    constraints.join(" ")
                ),
            ));
        }
        let requested_model = target.and_then(|t| t["model"].as_str());
        let same_instance = provider.provider_instance_id == inherited.instance_id;
        let model = requested_model
            .or_else(|| {
                if same_instance {
                    Some(inherited.model.as_str())
                } else {
                    provider.models.first().map(|m| m.id.as_str())
                }
            })
            .ok_or_else(|| {
                fail(
                    Code::ModelUnavailable,
                    format!("Provider {instance_id} has no model available for inheritance."),
                )
            })?;
        if !provider.models.is_empty() && !provider.models.iter().any(|m| m.id == model) {
            return Err(fail(
                Code::ModelUnavailable,
                format!("Model {model} is not advertised by provider {instance_id}."),
            ));
        }
        let options = target
            .and_then(|t| t.get("options"))
            .filter(|v| !v.is_null());
        let mut normalized = options
            .map(|v| normalize_contract("OrchestratorMcpTargetOptions", v.clone()))
            .transpose()
            .map_err(|_| {
                fail(
                    Code::InvalidRequest,
                    "Invalid model option selections.".into(),
                )
            })?;
        if same_instance && model == inherited.model && normalized.is_none() {
            normalized = inherited
                .options
                .as_ref()
                .map(|options| serde_json::to_value(options).expect("model options"));
        }
        if let Some(options) = &normalized {
            let descriptors = provider
                .models
                .iter()
                .find(|m| m.id == model)
                .and_then(|m| m.options.as_deref());
            let problems = invalid_options(options, descriptors);
            if !problems.is_empty() {
                return Err(fail(
                    Code::InvalidRequest,
                    format!(
                        "Model {model} on provider {instance_id} rejected options: {}",
                        problems.join(" ")
                    ),
                ));
            }
        }
        if same_instance && model == inherited.model && normalized.is_none() {
            return Ok(inherited.clone());
        }
        let mut selection = json!({"instanceId":instance_id,"model":model});
        if let Some(options) = normalized {
            selection["options"] = options;
        }
        serde_json::from_value(selection)
            .map_err(|_| fail(Code::InvalidRequest, "Invalid model selection.".into()))
    }
}

fn invalid_options(
    options: &Value,
    descriptors: Option<&[ProviderOptionDescriptor]>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for option in options.as_array().into_iter().flatten() {
        let id = option["id"].as_str().unwrap_or("");
        if !seen.insert(id) {
            problems.push(format!("Option {id} was specified more than once."));
            continue;
        }
        let Some(descriptors) = descriptors else {
            continue;
        };
        let descriptors: Vec<_> = descriptors
            .iter()
            .map(|d| serde_json::to_value(d).expect("descriptor"))
            .collect();
        let Some(descriptor) = descriptors.iter().find(|d| d["id"] == id) else {
            let known = descriptors
                .iter()
                .filter_map(|d| d["id"].as_str())
                .collect::<Vec<_>>()
                .join(", ");
            problems.push(format!(
                "Unknown option {id}; supported options: {}.",
                if known.is_empty() { "none" } else { &known }
            ));
            continue;
        };
        match descriptor["type"].as_str() {
            Some("boolean") if !option["value"].is_boolean() => {
                problems.push(format!("Option {id} expects a boolean value."))
            }
            Some("select")
                if !descriptor["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|c| c["id"] == option["value"]) =>
            {
                let choices = descriptor["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c["id"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(format!("Option {id} must be one of: {choices}."));
            }
            _ => {}
        }
    }
    problems
}

/// Read installed CPA discovery metadata only; never run sync, fetch a catalog,
/// copy templates/instructions, or read/persist inference credentials. Explicit
/// provider-instances.json remains authoritative over this compatibility import.
fn enrolled_models(id: HarnessId) -> Vec<CatalogModel> {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return vec![];
    };
    let config_path = std::env::var_os("CPA_SYNC_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".config/cpa-catalog/sync.json"));
    let read = |path: &std::path::Path| -> Option<Value> {
        if std::fs::metadata(path).ok()?.len() > 8 * 1024 * 1024 {
            return None;
        }
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    };
    let Some(config) = read(&config_path) else {
        return vec![];
    };
    let driver = match id {
        HarnessId::Codex => "codex",
        HarnessId::ClaudeCode => "claude",
        _ => return vec![],
    };
    let Some(path) = config[driver]["path"].as_str() else {
        return vec![];
    };
    let Some(metadata) = read(std::path::Path::new(path)) else {
        return vec![];
    };
    enrolled_metadata_models(id, &metadata)
}

fn enrolled_metadata_models(id: HarnessId, metadata: &Value) -> Vec<CatalogModel> {
    let rows = if id == HarnessId::Codex {
        &metadata["models"]
    } else {
        &metadata["modelPicker"]["options"]
    };
    rows.as_array().into_iter().flatten().filter_map(|row| {
        let model = row[if id == HarnessId::Codex { "slug" } else { "model" }].as_str()?;
        if model.is_empty() || model.len() > 250 || model.chars().any(char::is_control) { return None; }
        let mut options = vec![];
        let mut reasoning_levels = vec![];
        if id == HarnessId::Codex {
            let choices: Vec<_> = row["supported_reasoning_levels"].as_array().into_iter().flatten()
                .filter_map(|level| level["effort"].as_str()).map(|level| {
                    if let Ok(parsed) = serde_json::from_value(json!(level)) { reasoning_levels.push(parsed); }
                    json!({"id":level,"label":level,"isDefault":row["default_reasoning_level"] == level})
                }).collect();
            if !choices.is_empty() {
                options.push(json!({"type":"select","id":"reasoningEffort","label":"Reasoning",
                    "currentValue":row["default_reasoning_level"],"options":choices}));
            }
            let tiers: Vec<_> = row["service_tiers"].as_array().into_iter().flatten()
                .filter_map(|tier| tier["id"].as_str().map(|id| json!({"id":id,"label":tier["name"].as_str().unwrap_or(id),
                    "isDefault":row["default_service_tier"] == id}))).collect();
            if !tiers.is_empty() {
                options.push(json!({"type":"select","id":"serviceTier","label":"Service tier",
                    "currentValue":row["default_service_tier"],"options":tiers}));
            }
        }
        Some(CatalogModel { id:model.into(), label:row[if id == HarnessId::Codex { "display_name" } else { "label" }].as_str().map(str::to_owned),
            options: if options.is_empty() { None } else { serde_json::from_value(json!(options)).ok() },
            is_custom:true, reasoning_levels, description:None,
            legacy_reasoning_option: (id == HarnessId::Codex).then(|| "reasoningEffort".into()) })
    }).collect()
}

pub fn legacy_instance_id(id: HarnessId) -> ProviderInstanceId {
    ProviderInstanceId(legacy_driver(id).0)
}

/// The saved option id that carries a harness's reasoning level.
pub(crate) fn reasoning_option_key(harness: HarnessId) -> &'static str {
    match harness {
        HarnessId::ClaudeCode => "effort",
        HarnessId::Pi => "thinking",
        _ => "reasoningEffort",
    }
}

/// A selection's options as the id → value map `ChatConfig::model_options` uses.
pub(crate) fn selection_options(
    selection: &zeron_proto::provider_instance::ModelSelection,
) -> serde_json::Map<String, serde_json::Value> {
    selection
        .options
        .as_ref()
        .into_iter()
        .flatten()
        .filter_map(|o| Some((o.id.to_string(), serde_json::to_value(&o.value).ok()?)))
        .collect()
}

pub fn legacy_driver(id: HarnessId) -> ProviderDriverKind {
    let id = match id {
        HarnessId::ClaudeCode => "claudeAgent".to_owned(),
        _ => serde_json::to_value(id)
            .expect("harness id")
            .as_str()
            .expect("string")
            .to_owned(),
    };
    ProviderDriverKind(id)
}

fn from_legacy_model(harness: HarnessId, model: Model) -> CatalogModel {
    let mut options: Vec<ProviderOptionDescriptor> = model.options.into_iter().map(|o| {
        serde_json::from_value(json!({
            "type":"select","id":o.id,"label":o.label,"currentValue":o.default_choice,
            "options":o.choices.into_iter().map(|c| json!({"id":c.id,"label":c.label,"isDefault":c.id == o.default_choice})).collect::<Vec<_>>()
        })).expect("legacy option descriptor")
    }).collect();
    let mut legacy_reasoning_option = None;
    if !model.reasoning_levels.is_empty() {
        let id = reasoning_option_key(harness);
        if !options.iter().any(|descriptor| match descriptor {
            ProviderOptionDescriptor::Select(d) => d.id == id,
            ProviderOptionDescriptor::Boolean(d) => d.id == id,
        }) {
            // This is the existing composer's default policy. Configured T3
            // descriptors override it and retain their own isDefault/currentValue.
            let default = [
                zeron_proto::ReasoningLevel::High,
                zeron_proto::ReasoningLevel::Medium,
            ]
            .into_iter()
            .find(|level| model.reasoning_levels.contains(level))
            .unwrap_or(model.reasoning_levels[0]);
            let default = serde_json::to_value(default)
                .expect("reasoning")
                .as_str()
                .expect("string")
                .to_owned();
            let choices: Vec<_> = model
                .reasoning_levels
                .iter()
                .map(|level| {
                    let value = serde_json::to_value(level).expect("reasoning");
                    let id = value.as_str().expect("string");
                    json!({"id":id,"label":format!("{level:?}"),"isDefault":id == default})
                })
                .collect();
            options.insert(0, serde_json::from_value(json!({
                "id":id,"label":"Reasoning","type":"select","options":choices,"currentValue":default
            })).expect("reasoning descriptor"));
            legacy_reasoning_option = Some(id.into());
        }
    }
    CatalogModel {
        id: model.id,
        label: Some(model.label),
        options: (!options.is_empty()).then_some(options),
        is_custom: false,
        reasoning_levels: model.reasoning_levels,
        description: model.description,
        legacy_reasoning_option,
    }
}

fn to_legacy_model(model: CatalogModel) -> Model {
    let options = model
        .options
        .unwrap_or_default()
        .into_iter()
        // Legacy composer has a separate reasoning chip; don't show it twice.
        .filter(|descriptor| match descriptor {
            ProviderOptionDescriptor::Select(d) => {
                model.legacy_reasoning_option.as_ref() != Some(&d.id)
            }
            ProviderOptionDescriptor::Boolean(d) => {
                model.legacy_reasoning_option.as_ref() != Some(&d.id)
            }
        })
        .map(|o| match o {
            ProviderOptionDescriptor::Select(o) => ModelOption {
                id: o.id,
                label: o.label,
                default_choice: o
                    .current_value
                    .as_ref()
                    .cloned()
                    .or_else(|| {
                        o.options
                            .iter()
                            .find(|c| c.is_default.as_ref() == Some(&true))
                            .map(|c| c.id.clone())
                    })
                    .unwrap_or_default(),
                choices: o
                    .options
                    .into_iter()
                    .map(|c| ModelOptionChoice {
                        id: c.id,
                        label: c.label,
                    })
                    .collect(),
            },
            ProviderOptionDescriptor::Boolean(o) => ModelOption {
                id: o.id,
                label: o.label,
                default_choice: if o.current_value.as_ref().copied().unwrap_or(false) {
                    "on"
                } else {
                    "off"
                }
                .into(),
                choices: vec![
                    ModelOptionChoice {
                        id: "off".into(),
                        label: "Off".into(),
                    },
                    ModelOptionChoice {
                        id: "on".into(),
                        label: "On".into(),
                    },
                ],
            },
        })
        .collect();
    Model {
        label: model.label.unwrap_or_else(|| model.id.clone()),
        id: model.id,
        description: model.description,
        reasoning_levels: model.reasoning_levels,
        options,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeron_harness::mock::MockHarness;

    #[test]
    fn reasoning_key_and_selection_options_share_one_mapping() {
        assert_eq!(reasoning_option_key(HarnessId::ClaudeCode), "effort");
        assert_eq!(reasoning_option_key(HarnessId::Pi), "thinking");
        assert_eq!(reasoning_option_key(HarnessId::Codex), "reasoningEffort");
        let selection: zeron_proto::provider_instance::ModelSelection = serde_json::from_value(
            json!({"instanceId":"codex","model":"m","options":[
                {"id":"reasoningEffort","value":"high"},{"id":"fast","value":true}]}),
        )
        .unwrap();
        assert_eq!(
            serde_json::Value::Object(selection_options(&selection)),
            json!({"reasoningEffort":"high","fast":true})
        );
    }

    #[test]
    fn installed_cpa_metadata_preserves_exact_ids_options_and_discards_templates() {
        let models = enrolled_metadata_models(
            HarnessId::Codex,
            &json!({"models":[{
                "slug":"opencode-go/deepseek-v4.1-flash","display_name":"CPA model",
                "model_messages":{"instructions_template":"must not copy"},
                "default_reasoning_level":"high","supported_reasoning_levels":[{"effort":"low"},{"effort":"high"},{"effort":"max"}],
                "default_service_tier":"priority","service_tiers":[{"id":"priority","name":"Fast"}]
            }]}),
        );
        assert_eq!(models[0].id, "opencode-go/deepseek-v4.1-flash");
        let model = serde_json::to_value(&models[0]).unwrap();
        assert_eq!(model["options"][0]["id"], "reasoningEffort");
        assert_eq!(model["options"][0]["currentValue"], "high");
        assert_eq!(model["options"][1]["id"], "serviceTier");
        assert!(!model.to_string().contains("must not copy"));
        let claude = enrolled_metadata_models(
            HarnessId::ClaudeCode,
            &json!({"modelPicker":{"options":[
                {"model":"cpa/claude-opus-5-5[1m]","label":"Opus"},
                {"model":"claude-opus-5-5[1m]","label":"Other route"}
            ]}}),
        );
        assert_eq!(claude[0].id, "cpa/claude-opus-5-5[1m]");
        assert_eq!(claude[1].id, "claude-opus-5-5[1m]");
        assert!(enrolled_metadata_models(HarnessId::Codex, &json!({"models":[]})).is_empty());
    }

    fn registry() -> HarnessRegistry {
        let registry = HarnessRegistry::new();
        registry.register(std::sync::Arc::new(MockHarness { script: Vec::new() }));
        registry
    }

    fn instance(id: &str) -> ProviderInstance {
        serde_json::from_value(json!({
            "providerInstanceId":id,"driverKind":"mock","harnessId":"mock","displayName":id,
            "enabled":true,"installed":true,"authentication":"authenticated","adapterRegistered":true,
            "models":[{"id":"cpa/opencode-go/deepseek-v4.1-flash","label":"Custom CPA model","isCustom":true,
                "options":[{"id":"thinking","label":"Thinking","type":"boolean","currentValue":true},
                    {"id":"reasoningEffort","label":"Reasoning","type":"select","currentValue":"high",
                        "options":[{"id":"low","label":"Low"},{"id":"high","label":"High","isDefault":true}]}
                ]}]
        })).unwrap()
    }

    #[test]
    fn capabilities_readiness_matrix_and_removal() {
        let registry = registry();
        let mut disabled = instance("disabled");
        disabled.enabled = false;
        let mut signed_out = instance("signed-out");
        signed_out.authentication = Authentication::Unauthenticated;
        let mut adapter_missing = instance("no-adapter");
        adapter_missing.adapter_registered = false;
        let mut unknown = instance("unknown-driver");
        unknown.driver_kind = "futureDriver".into();
        unknown.harness_id = None;
        unknown.unavailable_reason = Some("Provider driver is unavailable.".into());
        let observer = registry.provider_instances.subscribe();
        registry
            .provider_instances
            .configure(vec![
                instance("ready"),
                disabled,
                signed_out,
                adapter_missing,
                unknown,
            ])
            .unwrap();
        let snapshot = registry.provider_instances.snapshot(&registry);
        assert!(observer.has_changed().unwrap());
        let capabilities: Vec<_> = snapshot.iter().map(ProviderInstance::capability).collect();
        assert!(capabilities[0].can_run_child_task);
        assert!(capabilities[0].can_run_cross_provider_child_task);
        assert_eq!(
            capabilities[1].constraints,
            ["Provider instance is disabled."]
        );
        assert_eq!(
            capabilities[2].constraints,
            ["Provider is not authenticated."]
        );
        assert_eq!(
            capabilities[3].constraints,
            ["No V2 provider adapter is registered."]
        );
        assert_eq!(
            capabilities[4].constraints,
            [
                "No V2 provider adapter is registered.",
                "Provider driver is unavailable."
            ]
        );
        for capability in &capabilities[1..] {
            assert!(
                !capability.can_run_child_task && !capability.can_run_cross_provider_child_task
            );
        }
        registry.provider_instances.configure(vec![]).unwrap();
        assert!(registry.provider_instances.snapshot(&registry).is_empty());
    }

    #[test]
    fn same_and_cross_instance_target_options_and_inheritance() {
        let registry = registry();
        let mut other = instance("other");
        other.driver_kind = "codex".into();
        registry
            .provider_instances
            .configure(vec![instance("parent"), other])
            .unwrap();
        let inherited: ModelSelection = serde_json::from_value(json!({
            "instanceId":"parent","model":"cpa/opencode-go/deepseek-v4.1-flash",
            "options":[{"id":"thinking","value":true}]
        }))
        .unwrap();
        let catalog = &registry.provider_instances;
        assert_eq!(
            catalog.resolve_target(&registry, &inherited, None).unwrap(),
            inherited
        );
        let cross = catalog
            .resolve_target(
                &registry,
                &inherited,
                Some(&json!({"providerInstanceId":"other"})),
            )
            .unwrap();
        assert_eq!(cross.instance_id.as_ref(), "other");
        assert!(cross.options.as_ref().is_none());
        assert_eq!(
            catalog
                .resolve_target(&registry, &inherited, Some(&json!({"driverKind":"mock"})))
                .unwrap(),
            inherited
        );
        let explicit = catalog.resolve_target(&registry, &inherited, Some(&json!({"providerInstanceId":"other","options":{"thinking":false,"reasoningEffort":"high"}}))).unwrap();
        assert_eq!(explicit.options.as_ref().unwrap().len(), 2);
        assert!(
            catalog
                .resolve_target(
                    &registry,
                    &inherited,
                    Some(&json!({"providerInstanceId":"other","options":{"thinking":4}}))
                )
                .is_err()
        );
        let invalid = catalog
            .resolve_target(
                &registry,
                &inherited,
                Some(&json!({"options":{"thinking":"true"}})),
            )
            .unwrap_err();
        assert!(
            invalid
                .message
                .contains("Option thinking expects a boolean value.")
        );
        let invalid = catalog.resolve_target(&registry, &inherited, Some(&json!({"options":[{"id":"thinking","value":true},{"id":"thinking","value":false}]}))).unwrap_err();
        assert!(
            invalid
                .message
                .contains("Option thinking was specified more than once.")
        );
        let invalid = catalog
            .resolve_target(
                &registry,
                &inherited,
                Some(&json!({"driverKind":"codex","providerInstanceId":"parent"})),
            )
            .unwrap_err();
        assert_eq!(
            serde_json::to_value(invalid.code).unwrap(),
            "invalid_request"
        );
        let invalid = catalog
            .resolve_target(&registry, &inherited, Some(&json!({"model":"missing"})))
            .unwrap_err();
        assert_eq!(
            serde_json::to_value(invalid.code).unwrap(),
            "model_unavailable"
        );
        let mut unavailable_parent = instance("parent");
        unavailable_parent.enabled = false;
        catalog
            .configure(vec![unavailable_parent, instance("other")])
            .unwrap();
        assert_eq!(
            catalog
                .resolve_target(&registry, &inherited, Some(&json!({"driverKind":"mock"})))
                .unwrap()
                .instance_id
                .as_ref(),
            "other"
        );
    }

    #[test]
    fn absent_descriptors_and_empty_inventory_are_not_empty_descriptor_lists() {
        let registry = registry();
        let mut no_descriptors = instance("ready");
        no_descriptors.models[0].options = None;
        registry
            .provider_instances
            .configure(vec![no_descriptors.clone()])
            .unwrap();
        let inherited: ModelSelection = serde_json::from_value(json!({
            "instanceId":"ready","model":"cpa/opencode-go/deepseek-v4.1-flash"
        }))
        .unwrap();
        assert!(
            serde_json::to_value(no_descriptors.capability()).unwrap()["models"][0]
                .get("options")
                .is_none()
        );
        assert!(
            registry
                .provider_instances
                .resolve_target(
                    &registry,
                    &inherited,
                    Some(&json!({"options":{"futureOption":true}}))
                )
                .is_ok()
        );
        no_descriptors.models[0].options = Some(vec![]);
        registry
            .provider_instances
            .configure(vec![no_descriptors.clone()])
            .unwrap();
        assert!(
            registry
                .provider_instances
                .resolve_target(
                    &registry,
                    &inherited,
                    Some(&json!({"options":{"futureOption":true}}))
                )
                .is_err()
        );
        no_descriptors.models.clear();
        registry
            .provider_instances
            .configure(vec![no_descriptors])
            .unwrap();
        let arbitrary = registry
            .provider_instances
            .resolve_target(
                &registry,
                &inherited,
                Some(
                    &json!({"model":"configured-but-unadvertised","options":{"futureOption":true}}),
                ),
            )
            .unwrap();
        assert_eq!(arbitrary.model, "configured-but-unadvertised");
        registry.provider_instances.configure(vec![]).unwrap();
        assert!(
            registry
                .provider_instances
                .resolve_target(&registry, &inherited, None)
                .is_err()
        );
    }

    #[test]
    fn inherited_models_and_options_are_revalidated_after_catalog_changes() {
        let registry = registry();
        let catalog = &registry.provider_instances;
        catalog.configure(vec![instance("parent")]).unwrap();
        let inherited: ModelSelection = serde_json::from_value(json!({
            "instanceId":"parent","model":"removed-custom-model"
        }))
        .unwrap();
        let invalid = catalog
            .resolve_target(&registry, &inherited, None)
            .unwrap_err();
        assert_eq!(
            serde_json::to_value(invalid.code).unwrap(),
            "model_unavailable"
        );
        let inherited: ModelSelection = serde_json::from_value(json!({
            "instanceId":"parent","model":"cpa/opencode-go/deepseek-v4.1-flash",
            "options":[{"id":"reasoningEffort","value":"max"}]
        }))
        .unwrap();
        let invalid = catalog
            .resolve_target(&registry, &inherited, None)
            .unwrap_err();
        assert_eq!(
            serde_json::to_value(invalid.code).unwrap(),
            "invalid_request"
        );
        assert!(invalid.message.contains("reasoningEffort"));
    }

    #[test]
    fn unknown_configured_auth_follows_discovery_but_explicit_auth_is_preserved() {
        let registry = registry();
        let catalog = &registry.provider_instances;
        let mut unknown = instance("discoverable");
        unknown.authentication = Authentication::Unknown;
        catalog
            .configure(vec![unknown, instance("explicit")])
            .unwrap();
        catalog.set_authentication(HarnessId::Mock, Authentication::Unauthenticated);
        let snapshot = catalog.snapshot(&registry);
        assert_eq!(snapshot[0].authentication, Authentication::Unauthenticated);
        assert!(!snapshot[0].constraints().is_empty());
        assert_eq!(snapshot[1].authentication, Authentication::Authenticated);
        catalog.set_authentication(HarnessId::Mock, Authentication::Authenticated);
        assert!(catalog.snapshot(&registry)[0].constraints().is_empty());
    }

    #[tokio::test]
    async fn composer_and_capabilities_use_the_same_custom_model_snapshot() {
        let registry = registry();
        registry
            .provider_instances
            .configure(vec![instance("cpa")])
            .unwrap();
        let composer = registry
            .provider_instances
            .refresh_instance(&registry, &"cpa".into())
            .await
            .unwrap();
        assert!(
            registry
                .provider_instances
                .refresh(&registry, HarnessId::Mock)
                .await
                .is_err(),
            "a missing canonical slot must not select an arbitrary sibling"
        );
        let snapshot = registry.provider_instances.snapshot(&registry);
        let capability = snapshot[0].capability();
        assert_eq!(
            composer.iter().map(|m| &m.id).collect::<Vec<_>>(),
            capability.models.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
        assert_eq!(composer[0].id, "cpa/opencode-go/deepseek-v4.1-flash");
        assert_eq!(composer[0].options[0].default_choice, "on");
        assert_eq!(composer[0].options[1].default_choice, "high");
        let wire = serde_json::to_value(&capability).unwrap();
        assert_eq!(wire["models"][0]["options"][0]["currentValue"], true);
        assert_eq!(
            wire["models"][0]["options"][1]["options"][1]["isDefault"],
            true
        );
        let mut changed = instance("cpa");
        changed.models.clear();
        registry
            .provider_instances
            .configure(vec![changed])
            .unwrap();
        assert!(
            !registry.provider_instances.snapshot(&registry)[0]
                .models
                .iter()
                .any(|m| m.is_custom)
        );
    }
}
