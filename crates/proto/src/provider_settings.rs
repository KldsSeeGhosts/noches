//! Host-local provider settings RPCs. Never store launch environments in CRDTs.
use serde::{Deserialize, Serialize};

use crate::provider_instance::{ProviderInstanceConfig, ProviderInstanceId};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInstanceSettings {
    pub instance_id: ProviderInstanceId,
    pub instance: ProviderInstanceConfig,
    #[serde(default)]
    pub authentication: String,
    #[serde(default)]
    pub model_count: usize,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteProviderInstance {
    pub instance_id: ProviderInstanceId,
    pub instance: ProviderInstanceConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInstanceKey {
    pub instance_id: ProviderInstanceId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateProviderInstance {
    pub instance_id: ProviderInstanceId,
    pub new_instance_id: ProviderInstanceId,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableProviderInstance {
    pub instance_id: ProviderInstanceId,
    pub enabled: bool,
}
