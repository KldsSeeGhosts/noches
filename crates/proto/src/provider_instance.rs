//! Generated T3 V2 wire contracts. See docs/orchestration/contracts.md.
//! Regenerate: node crates/proto/tests/t3_oracle/generate.mjs
//! Source: T3 Tools Inc., MIT, pinned in tests/t3_oracle/fixtures/provenance.json.
#![allow(unused_imports)]
use crate::orchestration::*;
use crate::orchestration_mcp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BooleanProviderOptionDescriptorType {
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanProviderOptionDescriptor {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "type")]
    pub r#type: BooleanProviderOptionDescriptorType,
    #[serde(
        rename = "currentValue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub current_value: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomModelEntry {
    #[serde(rename = "slug")]
    pub slug: TrimmedNonEmptyString,
    #[serde(rename = "name", default, skip_serializing_if = "Optional::is_absent")]
    pub name: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "capabilities",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub capabilities: Optional<ModelCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomModelSetting {
    Variant1(IsoDateTime),
    Variant2(CustomModelEntry),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    #[serde(
        rename = "optionDescriptors",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub option_descriptors: Optional<Vec<ProviderOptionDescriptor>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ModelSelectionCanonical {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: TrimmedNonEmptyString,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderOptionSelection>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct ModelSelection {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: TrimmedNonEmptyString,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderOptionSelection>>,
}
impl TryFrom<serde_json::Value> for ModelSelection {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: ModelSelectionCanonical =
            serde_json::from_value(normalize_contract("ModelSelection", value)?)
                .map_err(|e| e.to_string())?;
        Ok(Self {
            instance_id: value.instance_id,
            model: value.model,
            options: value.options,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpProviderCapabilityModelsItem {
    #[serde(rename = "id")]
    pub id: IsoDateTime,
    #[serde(rename = "label", deserialize_with = "required_nullable")]
    pub label: Option<IsoDateTime>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderOptionDescriptor>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpProviderCapability {
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "driverKind")]
    pub driver_kind: ProviderDriverKind,
    #[serde(rename = "displayName", deserialize_with = "required_nullable")]
    pub display_name: Option<IsoDateTime>,
    #[serde(rename = "models")]
    pub models: Vec<OrchestratorMcpProviderCapabilityModelsItem>,
    #[serde(rename = "canRunChildTask")]
    pub can_run_child_task: bool,
    #[serde(rename = "canRunCrossProviderChildTask")]
    pub can_run_cross_provider_child_task: bool,
    #[serde(rename = "constraints")]
    pub constraints: Vec<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderDriverKind(pub String);
impl From<String> for ProviderDriverKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProviderDriverKind {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProviderDriverKind {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProviderDriverKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceConfig {
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(
        rename = "displayName",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub display_name: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "accentColor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub accent_color: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "environment",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub environment: Optional<Vec<ProviderInstanceEnvironmentVariable>>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "config",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub config: Optional<serde_json::Value>,
}

pub type ProviderInstanceConfigMap = BTreeMap<ProviderInstanceId, ProviderInstanceConfig>;

pub type ProviderInstanceEnvironment = Vec<ProviderInstanceEnvironmentVariable>;

fn provider_instance_environment_variable_value_default() -> String {
    serde_json::from_str("\"\"").expect("upstream default")
}

fn provider_instance_environment_variable_sensitive_default() -> bool {
    serde_json::from_str("false").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceEnvironmentVariable {
    #[serde(rename = "name")]
    pub name: ProviderInstanceEnvironmentVariableName,
    #[serde(
        rename = "value",
        default = "provider_instance_environment_variable_value_default"
    )]
    pub value: String,
    #[serde(
        rename = "sensitive",
        default = "provider_instance_environment_variable_sensitive_default"
    )]
    pub sensitive: bool,
    #[serde(
        rename = "valueRedacted",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub value_redacted: Optional<bool>,
}

pub type ProviderInstanceEnvironmentVariableName = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderInstanceId(pub String);
impl From<String> for ProviderInstanceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProviderInstanceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProviderInstanceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProviderInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceMutationCreate {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
    #[serde(rename = "instance")]
    pub instance: ProviderInstanceConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceMutationUpsert {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
    #[serde(rename = "instance")]
    pub instance: ProviderInstanceConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceMutationRemove {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum ProviderInstanceMutation {
    #[serde(rename = "create")]
    Create(Box<ProviderInstanceMutationCreate>),
    #[serde(rename = "upsert")]
    Upsert(Box<ProviderInstanceMutationUpsert>),
    #[serde(rename = "remove")]
    Remove(Box<ProviderInstanceMutationRemove>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstanceRef {
    #[serde(rename = "instanceId")]
    pub instance_id: ProviderInstanceId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderOptionChoice {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "isDefault",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub is_default: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderOptionDescriptorSelect {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "options")]
    pub options: Vec<ProviderOptionChoice>,
    #[serde(
        rename = "currentValue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub current_value: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "promptInjectedValues",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt_injected_values: Optional<Vec<TrimmedNonEmptyString>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderOptionDescriptorBoolean {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "currentValue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub current_value: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderOptionDescriptor {
    #[serde(rename = "select")]
    Select(Box<ProviderOptionDescriptorSelect>),
    #[serde(rename = "boolean")]
    Boolean(Box<ProviderOptionDescriptorBoolean>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderOptionDescriptorType {
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderOptionSelection {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "value")]
    pub value: ProviderOptionSelectionValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderOptionSelectionValue {
    Variant1(TrimmedNonEmptyString),
    Variant2(bool),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct ProviderOptionSelections(pub Vec<ProviderOptionSelection>);
impl TryFrom<serde_json::Value> for ProviderOptionSelections {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(normalize_contract("ProviderOptionSelections", value)?)
            .map(Self)
            .map_err(|e| e.to_string())
    }
}
impl Serialize for ProviderOptionSelections {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectProviderOptionDescriptorType {
    #[serde(rename = "select")]
    Select,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectProviderOptionDescriptor {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "type")]
    pub r#type: SelectProviderOptionDescriptorType,
    #[serde(rename = "options")]
    pub options: Vec<ProviderOptionChoice>,
    #[serde(
        rename = "currentValue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub current_value: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "promptInjectedValues",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt_injected_values: Optional<Vec<TrimmedNonEmptyString>>,
}
