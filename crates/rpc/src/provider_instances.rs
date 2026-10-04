//! Typed UI client for host-addressable Settings → Providers operations.
use crate::{RpcClient, RpcError, methods};
use serde::Serialize;
use serde_json::{Value, json};
use zeron_proto::provider_settings::*;

fn params(value: impl Serialize, target: Option<&str>) -> Result<Value, RpcError> {
    let mut value = serde_json::to_value(value).map_err(|e| RpcError::Failed(e.to_string()))?;
    if let Some(target) = target {
        value["targetDeviceId"] = json!(target);
    }
    Ok(value)
}

impl RpcClient {
    pub async fn provider_instance_settings(
        &self,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(
            methods::GET_PROVIDER_INSTANCE_SETTINGS,
            params(json!({}), target)?,
        )
        .await
    }

    pub async fn create_provider_instance(
        &self,
        input: &WriteProviderInstance,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(methods::CREATE_PROVIDER_INSTANCE, params(input, target)?)
            .await
    }

    pub async fn update_provider_instance(
        &self,
        input: &WriteProviderInstance,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(methods::UPDATE_PROVIDER_INSTANCE, params(input, target)?)
            .await
    }

    pub async fn duplicate_provider_instance(
        &self,
        input: &DuplicateProviderInstance,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(methods::DUPLICATE_PROVIDER_INSTANCE, params(input, target)?)
            .await
    }

    pub async fn delete_provider_instance(
        &self,
        input: &ProviderInstanceKey,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(methods::DELETE_PROVIDER_INSTANCE, params(input, target)?)
            .await
    }

    pub async fn set_provider_instance_enabled(
        &self,
        input: &EnableProviderInstance,
        target: Option<&str>,
    ) -> Result<Vec<ProviderInstanceSettings>, RpcError> {
        self.call_as(
            methods::SET_PROVIDER_INSTANCE_ENABLED,
            params(input, target)?,
        )
        .await
    }
}
