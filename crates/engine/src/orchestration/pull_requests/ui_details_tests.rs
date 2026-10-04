use super::{Fixture, PullRequestLinks, ThreadPullRequestLinkSource, json};

#[tokio::test]
async fn desktop_pr_actions_keep_manual_source_watch_and_verbatim_refusal() {
    let f = Fixture::new();
    let caller = f.scope();
    let linked = super::super::mcp::invoke_for_user(
        &f.service,
        &caller,
        "link_pull_request",
        json!({"url":"https://github.com/o/r/pull/123"}),
    )
    .await
    .unwrap();
    assert_eq!(linked["alreadyLinked"], false);
    let links = f.service.links(&caller.thread_id).unwrap();
    assert_eq!(links[0].source, ThreadPullRequestLinkSource::Manual);
    for (name, watching) in [
        ("watch_pull_request", true),
        ("unwatch_pull_request", false),
    ] {
        let result = super::super::mcp::invoke_for_user(
            &f.service,
            &caller,
            name,
            json!({"url":"https://github.com/o/r/pull/123"}),
        )
        .await
        .unwrap();
        assert_eq!(result["watching"], watching);
    }
    assert_eq!(f.service.links(&caller.thread_id).unwrap().len(), 1);
    let error = super::super::mcp::invoke_for_user(
        &f.service,
        &caller,
        "link_pull_request",
        json!({"url":"bad"}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.message,
        "This is not a recognised pull request URL. Pass repository and number instead."
    );
}
