use std::path::Path;

use anyhow::Context;
use futures::StreamExt;
use serde_json::Value;
use zeron_rpc::git_actions::methods::*;
use zeron_rpc::{RpcError, RpcReply, parse_params};

use super::{GitActionsService, history::ScanJob};

fn field<'a>(params: &'a Value, name: &str) -> Result<&'a str, RpcError> {
    params[name]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| RpcError::BadParams(format!("{name} required")))
}

pub async fn dispatch(
    service: &GitActionsService,
    method: &str,
    params: Value,
) -> Result<RpcReply, RpcError> {
    let reply: anyhow::Result<RpcReply> = async {
        Ok(match method {
            GET_GIT_STATUS => {
                RpcReply::value(&service.status(Path::new(field(&params, "cwd")?)).await?)?
            }
            PREVIEW_GIT => RpcReply::value(&service.preview(parse_params(params)?).await?)?,
            START_GIT => RpcReply::value(&service.start(parse_params(params)?).await?)?,
            GET_WRITER => RpcReply::value(&service.settings()?)?,
            SET_WRITER => {
                service.set_settings(parse_params(params)?)?;
                RpcReply::value(&())?
            }
            GET_PULL => {
                let id = field(&params, "spaceId")?;
                service.local_space(id)?;
                RpcReply::value(&service.pull_state(id)?)?
            }
            SET_PULL => RpcReply::value(&service.set_pull_policy(parse_params(params)?).await?)?,
            RETRY_PULL => RpcReply::value(&service.retry_pull(field(&params, "spaceId")?).await?)?,
            SCAN_HISTORY => RpcReply::value(&service.scan_history(parse_params(params)?).await?)?,
            PREVIEW_HISTORY => {
                RpcReply::value(&service.history_preview(field(&params, "candidateId")?)?)?
            }
            IMPORT_HISTORY => {
                RpcReply::value(&service.import_history(parse_params(params)?).await?)?
            }
            CANCEL_HISTORY => {
                service.cancel_history(field(&params, "operationId")?)?;
                RpcReply::value(&())?
            }
            CONTINUE_HISTORY => {
                service
                    .continue_history(
                        field(&params, "spaceId")?,
                        field(&params, "candidateId")?,
                        field(&params, "chatId")?,
                    )
                    .await?;
                RpcReply::value(&())?
            }
            GET_GIT_ACTION | WATCH_GIT_ACTION | GET_SCAN | WATCH_SCAN | GET_IMPORT
            | WATCH_IMPORT => {
                let (ns, key) = match method {
                    GET_GIT_ACTION | WATCH_GIT_ACTION => ("action", "actionId"),
                    GET_SCAN | WATCH_SCAN => ("scan", "scanId"),
                    _ => ("import", "importId"),
                };
                let id = field(&params, key)?;
                if [WATCH_GIT_ACTION, WATCH_SCAN, WATCH_IMPORT].contains(&method) {
                    let receiver = service.watch(ns, id)?;
                    let scan = ns == "scan";
                    let stream = futures::stream::unfold(
                        (receiver, true, false),
                        move |(mut rx, first, terminal)| async move {
                            if terminal {
                                return None;
                            }
                            if !first && rx.changed().await.is_err() {
                                return None;
                            }
                            let value = rx.borrow_and_update().clone();
                            let value = if scan { value["state"].clone() } else { value };
                            let terminal = value["status"].as_str().is_some_and(|s| s != "running");
                            Some((value, (rx, false, terminal)))
                        },
                    )
                    .boxed();
                    RpcReply::Stream(stream)
                } else if ns == "scan" {
                    RpcReply::value(&service.state::<ScanJob>(ns, id)?.state)?
                } else {
                    let state: Value = service.state(ns, id).context("Unknown operation")?;
                    RpcReply::value(&state)?
                }
            }
            _ => return Err(RpcError::UnknownMethod(method.into()).into()),
        })
    }
    .await;
    reply.map_err(|error| RpcError::Failed(error.to_string()))
}
