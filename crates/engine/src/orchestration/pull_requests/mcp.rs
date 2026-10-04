use super::{PrError, PullRequestLinks, PullRequestService, identity};
use crate::mcp::{auth::InvocationScope, codec};
use crate::orchestration::service::CallerScope;
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn dispatch(
    service: Option<Arc<dyn PullRequestLinks>>,
    scope: &InvocationScope,
    name: &str,
    args: Value,
) -> Value {
    if !scope.capabilities.contains("pull-requests") {
        return codec::error_text(&PrError::capability(scope).message);
    }
    match service {
        Some(service) => match service.invoke(&scope.caller, name, args).await {
            Ok(value) => codec::result(value),
            Err(error) => codec::error_text(&error.message), // T3 failureMode=error
        },
        None => codec::error_text(&PrError::new(failure_tag(name)).message),
    }
}

fn failure_tag(name: &str) -> &'static str {
    match name {
        "link_pull_request" => "PullRequestLinkFailedError",
        "unlink_pull_request" => "PullRequestUnlinkFailedError",
        "list_thread_pull_requests" => "PullRequestListFailedError",
        _ => "PullRequestWatchFailedError",
    }
}

pub async fn invoke(
    service: &PullRequestService,
    caller: &CallerScope,
    name: &str,
    args: Value,
) -> Result<Value, PrError> {
    let failed = || PrError::new(failure_tag(name));
    let thread = service
        .kernel
        .store
        .thread(&caller.thread_id)
        .map_err(|_| failed())?
        .ok_or_else(|| PrError::missing(&caller.thread_id))?;
    if name == "list_thread_pull_requests" {
        return Ok(
            serde_json::to_value(crate::orchestration::ui_pull_requests::mcp_list(
                &super::links_of(&thread.thread),
            ))
            .expect("PR list"),
        );
    }
    let project = service
        .host
        .project_host(&caller.workspace_root)
        .await
        .map_err(|_| failed())?;
    let target = identity::resolve(
        serde_json::from_value(args).map_err(|_| failed())?,
        project.as_ref(),
    )?;
    match name {
        "link_pull_request" => {
            let already = service
                .link(
                    &caller.thread_id,
                    target.clone(),
                    zeron_proto::orchestration::ThreadPullRequestLinkSource::Agent,
                )
                .await
                .map_err(|_| failed())?;
            let mut value = serde_json::to_value(target).unwrap();
            value["alreadyLinked"] = json!(already);
            Ok(value)
        }
        "unlink_pull_request" => {
            let was = service
                .unlink(&caller.thread_id, target.clone())
                .await
                .map_err(|_| failed())?;
            Ok(
                json!({"host":target.host,"repository":target.repository,"number":target.number,"wasLinked":was}),
            )
        }
        _ => {
            let watching = name == "watch_pull_request";
            if watching && let Some(link) = service.links(&caller.thread_id).map_err(|_| failed())?.iter().find(|l| super::chains::identity(l).key() == target.key() && l.source != zeron_proto::orchestration::ThreadPullRequestLinkSource::StackDismissed)
                && let Some(snapshot) = &link.snapshot && snapshot.state != zeron_proto::orchestration::PullRequestState::Open {
                return Err(PrError::not_open(&snapshot.state));
            }
            let (now, was) = service
                .set_watching(&caller.thread_id, target.clone(), watching)
                .await
                .map_err(|_| failed())?;
            let mut value = serde_json::to_value(target).unwrap();
            value["watching"] = json!(now);
            value["wasWatching"] = json!(was);
            Ok(value)
        }
    }
}
