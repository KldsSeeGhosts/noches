//! User-authority settings RPC. Credentials never enter the replicated model.
use crate::{HarnessRegistry, SessionsEngine};
use serde_json::Value;
use zeron_proto::orchestration::Optional;
use zeron_proto::provider_settings::*;
use zeron_rpc::{RpcError, RpcReply, methods};

fn parse<T: serde::de::DeserializeOwned>(mut params: Value) -> Result<T, RpcError> {
    for key in ["instanceId", "newInstanceId"] {
        if let Some(id) = params.get_mut(key) {
            *id = Value::String(
                id.as_str()
                    .ok_or_else(|| {
                        RpcError::Failed("Provider instance id must be a string.".into())
                    })?
                    .trim()
                    .to_owned(),
            );
        }
    }
    serde_json::from_value(params).map_err(|e| RpcError::Failed(e.to_string()))
}

pub async fn dispatch(
    registry: &HarnessRegistry,
    sessions: &SessionsEngine,
    method: &str,
    params: Value,
) -> Result<RpcReply, RpcError> {
    let catalog = &registry.provider_instances;
    let gate_id = params
        .get(if method == methods::DUPLICATE_PROVIDER_INSTANCE {
            "newInstanceId"
        } else {
            "instanceId"
        })
        .and_then(Value::as_str)
        .map(|id| zeron_proto::provider_instance::ProviderInstanceId::from(id.trim()));
    let _lifecycle = if let Some(id) = &gate_id {
        Some(catalog.lifecycle(id).await)
    } else {
        None
    };
    let mut affected = None;
    let mut previous = None;
    match method {
        methods::GET_PROVIDER_INSTANCE_SETTINGS => {}
        methods::CREATE_PROVIDER_INSTANCE | methods::UPDATE_PROVIDER_INSTANCE => {
            let p: WriteProviderInstance = parse(params)?;
            previous = catalog.config(&p.instance_id);
            catalog
                .write_instance(
                    p.instance_id.clone(),
                    p.instance,
                    method == methods::CREATE_PROVIDER_INSTANCE,
                )
                .map_err(RpcError::Failed)?;
            affected = Some(p.instance_id);
        }
        methods::DUPLICATE_PROVIDER_INSTANCE => {
            let p: DuplicateProviderInstance = parse(params)?;
            catalog
                .duplicate_instance(&p.instance_id, p.new_instance_id, p.display_name)
                .map_err(RpcError::Failed)?;
        }
        methods::DELETE_PROVIDER_INSTANCE => {
            let p: ProviderInstanceKey = parse(params)?;
            previous = catalog.config(&p.instance_id);
            catalog
                .delete_instance(&p.instance_id)
                .map_err(RpcError::Failed)?;
            affected = Some(p.instance_id);
        }
        methods::SET_PROVIDER_INSTANCE_ENABLED => {
            let p: EnableProviderInstance = parse(params)?;
            let mut config = catalog
                .config(&p.instance_id)
                .ok_or_else(|| RpcError::Failed("Provider instance does not exist.".into()))?;
            previous = Some(config.clone());
            config.enabled = Optional::Present(p.enabled);
            // T3's effective flag is most restrictive. Clear a legacy raw
            // disable when the user explicitly enables the envelope.
            if let Optional::Present(value) = &mut config.config
                && let Some(object) = value.as_object_mut()
            {
                object.insert("enabled".into(), Value::Bool(p.enabled));
            }
            catalog
                .write_instance(p.instance_id.clone(), config, false)
                .map_err(RpcError::Failed)?;
            affected = Some(p.instance_id);
        }
        _ => {
            return Err(RpcError::Failed(
                "Unknown provider settings operation.".into(),
            ));
        }
    }
    // Invalid writes above cannot interrupt work. Successful no-op updates
    // retain the warm runtime; only the changed instance is torn down.
    if let Some(id) = affected
        && previous != catalog.config(&id)
    {
        sessions
            .interrupt_provider_instance(&id)
            .await
            .map_err(|e| RpcError::Failed(e.to_string()))?;
    }
    RpcReply::value(&catalog.settings(registry))
}
