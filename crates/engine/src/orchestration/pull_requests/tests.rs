// T3 conformance tests below are source-oracle ports, not live host traces.
use super::watch::*;
use super::*;
use crate::orchestration::{command::Command, store::WriteBoundary};
use crate::source_control::ChangeRequestError;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const START: &str = "2026-10-02T12:00:00.000Z";
const NOW: i64 = 1_791_028_800_000;

fn baseline() -> ThreadPullRequestWatch {
    ThreadPullRequestWatch {
        started_at: START.into(),
        head_sha: None,
        failed_checks: vec![],
        passed: false,
        remarks_through: START.into(),
        remark_ids: vec![],
        conflicting: false,
        wakes: 0,
    }
}
fn check(name: &str, status: &str, required: bool) -> Check {
    Check {
        name: name.into(),
        status: status.into(),
        required,
        url: Some(format!("https://ci.example/{name}")),
    }
}
fn detail() -> Detail {
    Detail {
        head_sha: Some("aaaaaaaaaa".into()),
        checks: vec![
            check("lint", "success", false),
            check("test", "pending", false),
        ],
        mergeability: "mergeable".into(),
        viewer: Some("agent-user".into()),
        author: Some("agent-user".into()),
    }
}
fn remark(id: &str, author: &str, at: &str) -> Remark {
    Remark {
        id: id.into(),
        author: Some(author.into()),
        body: "<!-- bot -->Needs a test.".into(),
        created_at: at.into(),
        url: Some(format!("https://github.com/o/r/pull/1#{id}")),
        path: Some("src/index.ts".into()),
        review_state: None,
    }
}
fn identity(number: i64) -> Identity {
    Identity {
        host: "github.com".into(),
        repository: "o/r".into(),
        number,
        url: format!("https://github.com/o/r/pull/{number}"),
    }
}
fn snapshot(state: &str, head: &str, base: &str) -> ThreadPullRequestSnapshot {
    serde_json::from_value(json!({"state":state,"title":"PR","headBranch":head,"baseBranch":base,"isDraft":false,
        "updatedAt":START,"syncedAt":START,"mergedAt":if state == "merged" {Some("2026-10-04T12:00:00Z")} else {None},
        "closedAt":if state == "closed" {Some("2026-10-04T12:00:00Z")} else {None},"checksState":"pending"})).unwrap()
}
fn link(number: i64, head: Option<&str>, base: &str) -> ThreadPullRequestLink {
    let mut link = new_link(&identity(number), ThreadPullRequestLinkSource::Agent, START);
    link.snapshot = head.map(|head| snapshot("open", head, base));
    link
}

#[test]
fn oracle_url_variants_and_precedence() {
    for (url, host, repo) in [
        (
            "https://github.com/T3Tools/T3Code/pull/123/files",
            "github.com",
            "t3tools/t3code",
        ),
        (
            "https://github.example/Owner/Repo/pull/123",
            "github.example",
            "owner/repo",
        ),
        (
            "http://forge.example:3000/git/Owner/Repo/pulls/123",
            "forge.example:3000",
            "git/owner/repo",
        ),
        (
            "https://code.example/group/sub/repo/-/merge_requests/123",
            "code.example",
            "group/sub/repo",
        ),
        (
            "https://forge.example/group/sub/pulls/repo/pulls/123",
            "forge.example",
            "group/sub/pulls/repo",
        ),
        (
            "https://bitbucket.org/Owner/Repo/pull-requests/123",
            "bitbucket.org",
            "owner/repo",
        ),
        (
            "https://org.visualstudio.com/DefaultCollection/Project/_git/Repo/pullrequest/123",
            "dev.azure.com",
            "org/project/_git/repo",
        ),
    ] {
        let got = identity::resolve(
            identity::Target {
                url: Some(url.into()),
                repository: Some("ignored".into()),
                number: Some(99),
                host: Some("wrong".into()),
            },
            None,
        )
        .unwrap();
        assert_eq!(
            (&*got.host, &*got.repository, got.number),
            (host, repo, 123)
        );
        assert_eq!(got.url, url);
    }
    for url in [
        "https://github.com/o/r/issues/1?token=private-value",
        "https://ordinary.example/o/r/pull/1",
        "javascript:alert(1)",
        "https://github.com/o/r/pull/0",
        "https://github.com//o/r/pull/1",
    ] {
        let error = identity::resolve(
            identity::Target {
                url: Some(url.into()),
                ..Default::default()
            },
            None,
        )
        .unwrap_err();
        assert_eq!(error.tag, "PullRequestUrlInvalidError");
        assert!(!error.message.contains("private-value"));
    }
}
#[test]
fn oracle_azure_project_web_url_and_narrow_legacy_canonicalization() {
    let project = identity::ProjectHost {
        host: "org.visualstudio.com".into(),
        kind: "azure-devops".into(),
        remote_url: "https://org.visualstudio.com/DefaultCollection/Project/_git/Repo".into(),
    };
    let target = identity::resolve(
        identity::Target {
            repository: Some("DefaultCollection/Project/_git/Repo".into()),
            number: Some(42),
            ..Default::default()
        },
        Some(&project),
    )
    .unwrap();
    assert_eq!(target.host, "dev.azure.com");
    assert_eq!(target.repository, "org/project/_git/repo");
    assert_eq!(
        target.url,
        "https://dev.azure.com/org/project/_git/repo/pullrequest/42"
    );
    assert_eq!(
        identity::canonical("org.visualstudio.com", "unrelated/repo"),
        ("org.visualstudio.com".into(), "unrelated/repo".into())
    );
}
#[test]
fn oracle_incomplete_no_remote_and_forgejo_web_origin() {
    assert_eq!(
        identity::resolve(identity::Target::default(), None)
            .unwrap_err()
            .tag,
        "PullRequestTargetIncompleteError"
    );
    let target = || identity::Target {
        repository: Some("git/owner/repo".into()),
        number: Some(42),
        ..Default::default()
    };
    assert_eq!(
        identity::resolve(target(), None).unwrap_err().tag,
        "PullRequestHostRequiredError"
    );
    let project = identity::ProjectHost {
        host: "forge.example".into(),
        kind: "forgejo".into(),
        remote_url: "http://forge.example:3000/git/owner/repo.git".into(),
    };
    let got = identity::resolve(target(), Some(&project)).unwrap();
    assert_eq!(got.host, "forge.example:3000");
    assert_eq!(got.url, "http://forge.example:3000/git/owner/repo/pulls/42");
    let other = identity::resolve(
        identity::Target {
            host: Some("other.example".into()),
            ..target()
        },
        Some(&project),
    )
    .unwrap();
    assert_eq!(other.url, "https://other.example/git/owner/repo/pull/42");
}
#[test]
fn oracle_derived_chain_native_priority_and_dismissal() {
    let mut links = vec![
        link(3, Some("c"), "b"),
        link(1, Some("a"), "main"),
        link(2, Some("b"), "a"),
        link(9, None, "main"),
        link(10, None, "main"),
    ];
    links[3].source = ThreadPullRequestLinkSource::StackDismissed;
    let result = crate::orchestration::ui_pull_requests::mcp_list(&links);
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(
        value["chains"],
        json!([{"kind":"derived","numbers":[1,2,3]},{"kind":"derived","numbers":[10]}])
    );
    assert_eq!(
        value["pullRequests"][0]["stack"],
        json!({"kind":"derived","position":3,"size":3})
    );
    assert_eq!(value["pullRequests"][3]["state"], Value::Null);
    let stack: ThreadPullRequestStack = serde_json::from_value(json!({"kind":"native","id":"stack","number":1,"url":"https://github.com/o/r/stacks/1","base":"main","layers":[{"number":1,"headBranch":"a","state":"open"},{"number":2,"headBranch":"b","state":"open"}]})).unwrap();
    links[1].stack = Some(stack.clone());
    links[2].stack = Some(stack);
    let chains = chains::resolve(&links);
    assert_eq!(chains[0].kind, "native");
    assert_eq!(
        chains[0]
            .layers
            .iter()
            .map(|l| l.number)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}
#[test]
fn oracle_ambiguous_branch_and_cycles_do_not_invent_order() {
    let links = vec![link(1, Some("a"), "b"), link(2, Some("b"), "a")];
    assert!(chains::resolve(&links).iter().all(|c| c.layers.len() == 1));
    let links = vec![
        link(1, Some("a"), "main"),
        link(2, Some("a"), "main"),
        link(3, Some("b"), "a"),
    ];
    assert!(chains::resolve(&links).iter().all(|c| c.layers.len() == 1));
}
#[test]
fn oracle_forgejo_legacy_port_recovered() {
    let mut link = link(42, None, "main");
    link.host = "forge.example".into();
    link.url = "http://forge.example:3000/o/r/pulls/42".into();
    assert_eq!(chains::identity(&link).host, "forge.example:3000");
}
#[test]
fn legacy_forgejo_stack_sync_keeps_canonical_tombstone_and_sibling_ownership() {
    let f = Fixture::new();
    let id = ThreadId("thread".into());
    let mut thread = f.service.kernel.store.thread(&id).unwrap().unwrap().thread;
    let mut links = vec![link(42, None, "main"), link(43, None, "main")];
    for link in &mut links {
        link.host = "forge.example".into();
        link.url = format!("http://forge.example:3000/o/r/pulls/{}", link.number);
    }
    links[1].source = ThreadPullRequestLinkSource::StackDismissed;
    let target = chains::identity(&links[0]);
    thread.pull_requests = Optional::Present(links);
    f.service
        .kernel
        .store
        .dispatch(
            &Command {
                id: CommandId("legacy-forgejo".into()),
                thread_id: id.clone(),
                operation: Operation::SessionBinding(Box::new(thread)),
            },
            NOW,
        )
        .unwrap();
    let stack = serde_json::from_value(json!({
        "kind":"native","id":"stack","number":42,"url":"http://forge.example:3000/o/r/stacks/42",
        "base":"main","layers":[
            {"number":42,"headBranch":"a","state":"open"},
            {"number":43,"headBranch":"b","state":"open"}
        ]
    }))
    .unwrap();
    f.command(
        "thread",
        PrOperation::Sync {
            target,
            snapshot: Box::new(snapshot("open", "a", "main")),
            stack: Some(stack),
            preserve_stack: false,
        },
        NOW,
    )
    .unwrap();
    let links = f.service.links(&id).unwrap();
    assert_eq!(
        links.len(),
        2,
        "legacy host spelling cannot duplicate a layer"
    );
    assert_eq!(links[1].source, ThreadPullRequestLinkSource::StackDismissed);
}
#[test]
fn oracle_failures_immediate_reruns_and_head_reset() {
    let mut d = detail();
    d.checks = vec![
        check("lint", "failure", false),
        check("test", "pending", false),
    ];
    let first = evaluate(&baseline(), &d, Some(&[]));
    assert_eq!(
        first.changes,
        vec![Change::Failed(vec![check("lint", "failure", false)])]
    );
    assert!(evaluate(&first.next, &d, Some(&[])).changes.is_empty());
    d.checks.push(check("test", "cancelled", false));
    assert_eq!(evaluate(&first.next, &d, None).changes.len(), 1);
    let unreadable = evaluate(
        &first.next,
        &Detail {
            checks: vec![],
            ..d.clone()
        },
        None,
    );
    assert!(
        evaluate(
            &unreadable.next,
            &Detail {
                checks: vec![check("lint", "failure", false)],
                ..d.clone()
            },
            None
        )
        .changes
        .is_empty()
    );
    let running = evaluate(
        &first.next,
        &Detail {
            checks: vec![check("lint", "pending", false)],
            ..d.clone()
        },
        None,
    );
    assert_eq!(evaluate(&running.next, &d, None).changes.len(), 1);
    d.head_sha = Some("bbbbbbbbbb".into());
    assert_eq!(evaluate(&first.next, &d, None).changes.len(), 1);
}
#[test]
fn oracle_required_checks_pass_and_failures_reported_first() {
    let mut d = detail();
    d.checks = vec![
        check("test", "success", true),
        check("lint", "success", true),
        check("bot", "failure", false),
    ];
    let report = evaluate(&baseline(), &d, None);
    assert!(matches!(report.changes[0], Change::Failed(_)));
    assert_eq!(
        report.changes[1],
        Change::Passed {
            count: 2,
            required: true
        }
    );
    assert!(evaluate(&report.next, &d, None).changes.is_empty());
    d.checks = vec![
        check("test", "success", false),
        check("bot", "pending", false),
    ];
    assert!(evaluate(&baseline(), &d, None).changes.is_empty());
}
#[test]
fn oracle_comments_self_exclusion_author_fallback_and_boundary_ids() {
    let first = remark("first", "reviewer", "2026-10-02T12:06:00Z");
    let late = remark("late", "bot", "2026-10-02T12:06:00Z");
    let own = remark("own", "AGENT-USER", "2026-10-02T12:05:00Z");
    let old = remark("old", "reviewer", "2026-10-02T11:00:00Z");
    let partial = evaluate(&baseline(), &detail(), None);
    let report = evaluate(&partial.next, &detail(), Some(&[first.clone(), own, old]));
    assert_eq!(report.changes, vec![Change::Remarks(vec![first.clone()])]);
    let report = evaluate(&report.next, &detail(), Some(&[first, late.clone()]));
    assert_eq!(report.changes, vec![Change::Remarks(vec![late])]);
    assert_eq!(report.next.remark_ids, vec!["first", "late"]);
    let author = remark("author", "contributor", "2026-10-02T12:07:00Z");
    let d = Detail {
        author: Some("contributor".into()),
        ..detail()
    };
    assert_eq!(
        evaluate(&baseline(), &d, Some(std::slice::from_ref(&author)))
            .changes
            .len(),
        1
    );
    assert!(
        evaluate(&baseline(), &Detail { viewer: None, ..d }, Some(&[author]))
            .changes
            .is_empty()
    );
}
#[test]
fn oracle_conflict_unknown_is_not_resolution() {
    let d = Detail {
        mergeability: "conflicting".into(),
        ..detail()
    };
    let first = evaluate(&baseline(), &d, None);
    assert_eq!(first.changes, vec![Change::Conflicting]);
    let unknown = evaluate(
        &first.next,
        &Detail {
            mergeability: "unknown".into(),
            ..d.clone()
        },
        None,
    );
    assert!(unknown.next.conflicting);
    assert!(evaluate(&unknown.next, &d, None).changes.is_empty());
    let clean = evaluate(&first.next, &detail(), None);
    assert_eq!(
        evaluate(&clean.next, &d, None).changes,
        vec![Change::Conflicting]
    );
}
#[test]
fn oracle_ten_comment_wakes_progress_resets_and_exact_message() {
    let mut watch = baseline();
    watch.head_sha = detail().head_sha;
    watch.wakes = 9;
    let comments = [remark("reviewer", "reviewer", "2026-10-02T12:10:00Z")];
    let report = evaluate(&watch, &detail(), Some(&comments));
    assert!(report.exhausted);
    let pushed = evaluate(
        &watch,
        &Detail {
            head_sha: Some("cccccccccc".into()),
            ..detail()
        },
        Some(&comments),
    );
    assert!(!pushed.exhausted);
    assert_eq!(pushed.next.wakes, 1);
    let report = evaluate(
        &watch,
        &Detail {
            checks: vec![check("lint", "failure", false)],
            ..detail()
        },
        Some(&comments),
    );
    assert!(!report.exhausted);
    assert_eq!(report.next.wakes, 0);
    let wake = message(12, "https://github.com/o/r/pull/12", "main", &report);
    assert!(
        wake.text
            .contains("- Checks failed on aaaaaaa:\n  - lint https://ci.example/lint")
    );
    assert!(
        wake.text
            .contains("reviewer on src/index.ts: \"Needs a test.\"")
    );
    assert_eq!(
        wake.notification,
        json!({"source":{"kind":"monitor"},"outcome":"failed","summary":"#12: checks failed, new comments"})
    );
}

struct FakeHost {
    read: Mutex<std::result::Result<host::HostRead, ChangeRequestError>>,
    calls: AtomicUsize,
}
impl FakeHost {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            read: Mutex::new(Ok(host::HostRead {
                snapshot: snapshot("open", "a", "main"),
                stack: None,
                stack_read_failed: false,
                detail: detail(),
                remarks: Some(vec![]),
            })),
            calls: AtomicUsize::new(0),
        })
    }
}
#[async_trait]
impl host::PullRequestHost for FakeHost {
    async fn project_host(
        &self,
        _: &Path,
    ) -> std::result::Result<Option<identity::ProjectHost>, ChangeRequestError> {
        Ok(Some(identity::ProjectHost {
            host: "github.com".into(),
            kind: "github".into(),
            remote_url: "git@github.com:o/r.git".into(),
        }))
    }
    async fn read(
        &self,
        _: &Path,
        _: &Identity,
        _: bool,
    ) -> std::result::Result<host::HostRead, ChangeRequestError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.read.lock().unwrap().clone()
    }
    async fn branch_is_open(
        &self,
        _: &Path,
        _: &str,
    ) -> std::result::Result<bool, ChangeRequestError> {
        Ok(false)
    }
}
struct Context;
impl reactor::PrThreadContext for Context {
    fn cwd(&self, _: &OrchestrationV2AppThread) -> Option<PathBuf> {
        Some("/repo".into())
    }
}
struct Fixture {
    dir: tempfile::TempDir,
    service: Arc<PullRequestService>,
    host: Arc<FakeHost>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Kernel::open(
            Arc::new(zeron_sync::DocsStore::open(dir.path()).unwrap()),
            "host",
        )
        .unwrap();
        let host = FakeHost::new();
        let f = Self {
            dir,
            service: Arc::new(PullRequestService {
                kernel,
                host: host.clone(),
            }),
            host,
        };
        f.create("thread");
        f
    }
    fn create(&self, id: &str) {
        let c=Command::wire(serde_json::from_value(json!({"type":"thread.create","commandId":format!("create:{id}"),
            "createdBy":"user","creationSource":"web","threadId":id,"projectId":"project","title":"PR tests",
            "modelSelection":{"instanceId":"codex","model":"gpt-5"},"runtimeMode":"full-access","interactionMode":"default",
            "branch":null,"worktreePath":"/repo"})).unwrap()).unwrap();
        assert_eq!(
            self.service.kernel.store.dispatch(&c, NOW).unwrap().status,
            crate::orchestration::ReceiptStatus::Accepted
        );
    }
    fn command(
        &self,
        id: &str,
        op: PrOperation,
        at: i64,
    ) -> Result<crate::orchestration::CommandReceipt> {
        self.service.kernel.store.dispatch(
            &Command {
                id: CommandId(uuid::Uuid::new_v4().to_string()),
                thread_id: ThreadId(id.into()),
                operation: Operation::PullRequest(Box::new(op)),
            },
            at,
        )
    }
    fn watch(&self, n: i64) {
        assert_eq!(
            self.command(
                "thread",
                PrOperation::Watch {
                    target: identity(n),
                    watching: true
                },
                NOW
            )
            .unwrap()
            .status,
            crate::orchestration::ReceiptStatus::Accepted
        );
    }
    fn scope(&self) -> crate::orchestration::service::CallerScope {
        crate::orchestration::service::CallerScope {
            thread_id: ThreadId("thread".into()),
            run_id: RunId("run".into()),
            session_id: "session".into(),
            project_id: ProjectId("project".into()),
            workspace_root: "/repo".into(),
            runtime_mode: zeron_proto::RuntimeMode::FullAccess,
            interaction_mode: zeron_proto::InteractionMode::Default,
            provider_instance_id: "codex".into(),
        }
    }
}
#[tokio::test]
async fn oracle_mcp_idempotent_flags_and_not_open_error() {
    let f = Fixture::new();
    let scope = f.scope();
    let input = json!({"repository":"O/R","number":123});
    assert_eq!(
        f.service
            .invoke(&scope, "link_pull_request", input.clone())
            .await
            .unwrap()["alreadyLinked"],
        false
    );
    assert_eq!(
        f.service
            .invoke(&scope, "link_pull_request", input.clone())
            .await
            .unwrap()["alreadyLinked"],
        true
    );
    assert_eq!(
        f.service
            .invoke(&scope, "unlink_pull_request", input.clone())
            .await
            .unwrap()["wasLinked"],
        true
    );
    assert_eq!(
        f.service
            .invoke(&scope, "unlink_pull_request", input.clone())
            .await
            .unwrap()["wasLinked"],
        false
    );
    let watch = f
        .service
        .invoke(&scope, "watch_pull_request", input.clone())
        .await
        .unwrap();
    assert_eq!(
        (watch["watching"].clone(), watch["wasWatching"].clone()),
        (json!(true), json!(false))
    );
    let watch = f
        .service
        .invoke(&scope, "watch_pull_request", input.clone())
        .await
        .unwrap();
    assert_eq!(watch["wasWatching"], true);
    let before = f.service.links(&scope.thread_id).unwrap()[0].watch.clone();
    assert_eq!(before, f.service.links(&scope.thread_id).unwrap()[0].watch);
    f.command(
        "thread",
        PrOperation::Sync {
            target: identity(123),
            snapshot: Box::new(snapshot("merged", "done", "main")),
            stack: None,
            preserve_stack: false,
        },
        NOW + 1,
    )
    .unwrap();
    let error = f
        .service
        .invoke(&scope, "watch_pull_request", input.clone())
        .await
        .unwrap_err();
    assert_eq!(error.tag, "PullRequestNotOpenError");
    assert_eq!(
        error.message,
        "The pull request is merged, so there is nothing to watch."
    );
    let unwatch = f
        .service
        .invoke(&scope, "unwatch_pull_request", input)
        .await
        .unwrap();
    assert_eq!(unwatch["wasWatching"], true);
    assert_eq!(unwatch["watching"], false);
    assert_eq!(f.service.links(&scope.thread_id).unwrap().len(), 1);
}
#[tokio::test]
async fn oracle_missing_thread_and_failure_mode_framing() {
    let f = Fixture::new();
    let mut scope = f.scope();
    scope.thread_id = ThreadId("gone".into());
    let error = f
        .service
        .invoke(&scope, "list_thread_pull_requests", json!({}))
        .await
        .unwrap_err();
    assert_eq!(error.tag, "PullRequestThreadNotFoundError");
    assert_eq!(error.message, "Thread gone was not found.");
    let mut invocation = crate::mcp::auth::InvocationScope {
        environment_id: "env".into(),
        selection: serde_json::from_value(json!({"instanceId":"codex","model":"gpt-5"})).unwrap(),
        capabilities: Default::default(),
        issued_at: 0,
        task_id: None,
        caller: scope,
    };
    let result = mcp::dispatch(
        Some(f.service.clone()),
        &invocation,
        "link_pull_request",
        json!({}),
    )
    .await;
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["content"][0]["text"],
        "MCP credential does not grant the pull-requests capability."
    );
    invocation.capabilities.insert("pull-requests".into());
    let result = mcp::dispatch(
        Some(f.service.clone()),
        &invocation,
        "list_thread_pull_requests",
        json!({}),
    )
    .await;
    assert_eq!(result["isError"], true);
    assert!(result.get("structuredContent").is_none());
}
#[tokio::test]
async fn durable_restart_rebuild_publication_and_write_rollback() {
    let f = Fixture::new();
    f.watch(1);
    let id = ThreadId("thread".into());
    let before = f.service.links(&id).unwrap();
    let kernel = Kernel::open(
        Arc::new(zeron_sync::DocsStore::open(f.dir.path()).unwrap()),
        "host",
    )
    .unwrap();
    kernel.store.rebuild().unwrap();
    assert_eq!(
        links_of(&kernel.store.thread(&id).unwrap().unwrap().thread),
        before
    );
    let state = kernel.store.ui_state(&id).unwrap();
    assert_eq!(state["pullRequests"]["pullRequests"][0]["watching"], true);
    kernel.store.inject_failure(WriteBoundary::BeforeCommit, 1);
    let result = kernel.store.dispatch(
        &Command {
            id: CommandId("rollback".into()),
            thread_id: id.clone(),
            operation: Operation::PullRequest(Box::new(PrOperation::Unlink {
                target: identity(1),
            })),
        },
        NOW,
    );
    assert!(result.is_err());
    assert_eq!(
        links_of(&kernel.store.thread(&id).unwrap().unwrap().thread),
        before
    );
}
#[tokio::test]
async fn stale_watch_sync_does_not_resurrect_or_wake() {
    let f = Fixture::new();
    f.watch(1);
    let watch = f.service.links(&ThreadId("thread".into())).unwrap()[0]
        .watch
        .as_ref()
        .unwrap()
        .clone();
    f.command(
        "thread",
        PrOperation::Watch {
            target: identity(1),
            watching: false,
        },
        NOW + 100,
    )
    .unwrap();
    f.command(
        "thread",
        PrOperation::Watch {
            target: identity(1),
            watching: true,
        },
        NOW + 200,
    )
    .unwrap();
    let receipt=f.command("thread",PrOperation::WatchSync {target:identity(1),started_at:watch.started_at.clone(),watch:Some(watch),
        wake:Some(Wake {text:"stale".into(),notification:json!({"source":{"kind":"monitor"},"outcome":"updated","summary":"stale"})})},NOW+300).unwrap();
    assert_eq!(
        receipt.status,
        crate::orchestration::ReceiptStatus::Rejected
    );
    assert!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("thread".into()))
            .unwrap()
            .unwrap()
            .runs
            .is_empty()
    );
}
#[tokio::test]
async fn minute_failure_limit_delivers_one_durable_continuation() {
    let f = Fixture::new();
    f.watch(1);
    *f.host.read.lock().unwrap() = Err(ChangeRequestError::Authentication);
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    for _ in 0..14 {
        reactor.sweep().await.unwrap();
    }
    assert!(
        f.service.links(&ThreadId("thread".into())).unwrap()[0]
            .watch
            .as_ref()
            .is_some()
    );
    reactor.sweep().await.unwrap();
    let projection = f
        .service
        .kernel
        .store
        .thread(&ThreadId("thread".into()))
        .unwrap()
        .unwrap();
    assert!(links_of(&projection.thread)[0].watch.as_ref().is_none());
    assert_eq!(projection.runs.len(), 1);
    let message = crate::orchestration::task::records(&projection, "message")
        .last()
        .unwrap();
    assert!(message["text"].as_str().unwrap().contains("for 15 minutes"));
    assert_eq!(message["notification"]["source"]["kind"], "monitor");
    assert!(crate::orchestration::task::monitor_run(
        &projection,
        &projection.runs[0]
    ));
    reactor.sweep().await.unwrap();
    assert_eq!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("thread".into()))
            .unwrap()
            .unwrap()
            .runs
            .len(),
        1
    );
}
#[tokio::test]
async fn watcher_stops_terminal_skips_settled_and_shares_host_reads() {
    let f = Fixture::new();
    f.watch(1);
    f.create("other");
    f.command(
        "other",
        PrOperation::Watch {
            target: identity(1),
            watching: true,
        },
        NOW,
    )
    .unwrap();
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    reactor.sweep().await.unwrap();
    assert_eq!(f.host.calls.load(Ordering::Relaxed), 1);
    f.host.read.lock().unwrap().as_mut().unwrap().snapshot = snapshot("closed", "a", "main");
    reactor.sweep().await.unwrap();
    assert!(
        f.service.links(&ThreadId("thread".into())).unwrap()[0]
            .watch
            .as_ref()
            .is_none()
    );
    assert!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("thread".into()))
            .unwrap()
            .unwrap()
            .thread
            .settled_at
            .is_some()
    );
}
#[tokio::test]
async fn sync_discovers_stack_before_settlement_and_keeps_tombstones() {
    let f = Fixture::new();
    f.watch(42);
    let stack:ThreadPullRequestStack=serde_json::from_value(json!({"kind":"native","id":"stack","number":42,"url":"https://github.com/o/r/stacks/42","base":"main",
        "layers":[{"number":41,"headBranch":"a","state":"merged"},{"number":42,"headBranch":"b","state":"open"},{"number":43,"headBranch":"c","state":"open"}]})).unwrap();
    f.command(
        "thread",
        PrOperation::Link {
            target: identity(41),
            source: ThreadPullRequestLinkSource::Stack,
        },
        NOW,
    )
    .unwrap();
    f.command(
        "thread",
        PrOperation::Unlink {
            target: identity(41),
        },
        NOW,
    )
    .unwrap();
    f.command(
        "thread",
        PrOperation::Sync {
            target: identity(42),
            snapshot: Box::new(snapshot("merged", "b", "a")),
            stack: Some(stack),
            preserve_stack: false,
        },
        NOW,
    )
    .unwrap();
    let links = f.service.links(&ThreadId("thread".into())).unwrap();
    assert_eq!(
        links.iter().find(|l| l.number == 41).unwrap().source,
        ThreadPullRequestLinkSource::StackDismissed
    );
    assert_eq!(
        links.iter().find(|l| l.number == 43).unwrap().source,
        ThreadPullRequestLinkSource::Stack
    );
    assert!(
        links
            .iter()
            .find(|l| l.number == 43)
            .unwrap()
            .snapshot
            .is_none()
    );
    let c = settlement::candidate(
        &f.service
            .kernel
            .store
            .thread(&ThreadId("thread".into()))
            .unwrap()
            .unwrap(),
    );
    assert!(settlement::resolve(&c, &links, NOW + 1, true, None).is_none());
}
#[test]
fn oracle_settlement_guards_terminal_timestamp_and_unrelated_merge() {
    let c = settlement::Candidate {
        created_at: Some(NOW),
        completed_at: Some(NOW + 1),
        ..Default::default()
    };
    let mut merged = link(1, Some("a"), "main");
    merged.snapshot = Some(snapshot("merged", "a", "main"));
    assert!(settlement::resolve(&c, std::slice::from_ref(&merged), NOW + 2, true, None).is_some());
    assert!(settlement::resolve(&c, std::slice::from_ref(&merged), NOW + 2, false, None).is_none());
    assert!(
        settlement::resolve(
            &c,
            &[link(2, Some("unrelated"), "main")],
            NOW + 2,
            true,
            None
        )
        .is_none()
    );
    for c in [
        settlement::Candidate {
            active: true,
            ..c.clone()
        },
        settlement::Candidate {
            background: true,
            ..c.clone()
        },
        settlement::Candidate {
            pending_request: true,
            ..c.clone()
        },
        settlement::Candidate {
            pinned: true,
            ..c.clone()
        },
        settlement::Candidate {
            override_present: true,
            ..c.clone()
        },
        settlement::Candidate {
            archived: true,
            ..c.clone()
        },
        settlement::Candidate {
            auto_disabled: true,
            ..c.clone()
        },
        settlement::Candidate {
            snoozed_until: Some(NOW + 10),
            snoozed_at: Some(NOW + 5),
            ..c.clone()
        },
    ] {
        assert!(
            settlement::resolve(&c, std::slice::from_ref(&merged), NOW + 2, true, None).is_none()
        );
    }
    let fresh = settlement::Candidate {
        user_at: Some(NOW + 2),
        ..c.clone()
    };
    assert!(settlement::queued_start(&fresh, NOW + 3));
    let old = settlement::Candidate {
        user_at: Some(NOW + 900_000_000),
        ..c
    };
    assert!(settlement::resolve(&old, &[merged], NOW + 900_000_001, true, None).is_none());
}

#[tokio::test]
async fn wake_queues_after_active_then_drains_through_shared_continuation() {
    use crate::orchestration::task::TaskOperation;
    use zeron_proto::{AgentEvent, DoneStatus};
    let f = Fixture::new();
    f.watch(1);
    let id = ThreadId("thread".into());
    f.service
        .kernel
        .task_command(
            &id,
            CommandId("start".into()),
            TaskOperation::StartMessage {
                prompt: "working".into(),
                driver: zeron_proto::provider_instance::ProviderDriverKind("codex".into()),
                message_id: MessageId("user".into()),
            },
        )
        .await
        .unwrap();
    let before = f.service.kernel.store.thread(&id).unwrap().unwrap();
    let baseline = links_of(&before.thread)[0].watch.as_ref().unwrap().clone();
    f.service.command(&id,PrOperation::WatchSync {target:identity(1),started_at:baseline.started_at.clone(),watch:Some(baseline),
        wake:Some(Wake {text:"PR news".into(),notification:json!({"source":{"kind":"monitor"},"outcome":"updated","summary":"news"})})}).await.unwrap();
    let queued = f.service.kernel.store.thread(&id).unwrap().unwrap();
    assert_eq!(queued.runs[0].status, OrchestrationV2RunStatus::Starting);
    assert_eq!(queued.runs[1].status, OrchestrationV2RunStatus::Queued);
    assert_eq!(
        queued.thread.active_provider_thread_id,
        before.thread.active_provider_thread_id
    );
    // No PR-specific turn runner or toast path.
    f.service
        .kernel
        .task_command(
            &id,
            CommandId("done".into()),
            TaskOperation::RunnerEvent {
                run_id: queued.runs[0].id.clone(),
                attempt_id: queued.runs[0].active_attempt_id.clone().unwrap(),
                event: AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: Some("done".into()),
                    error: None,
                    session_id: None,
                },
                capabilities: None,
            },
        )
        .await
        .unwrap();
    f.service
        .kernel
        .task_command(&id, CommandId("drain".into()), TaskOperation::DrainQueue)
        .await
        .unwrap();
    let drained = f.service.kernel.store.thread(&id).unwrap().unwrap();
    assert_eq!(drained.runs[1].status, OrchestrationV2RunStatus::Starting);
    assert!(crate::orchestration::task::monitor_run(
        &drained,
        &drained.runs[1]
    ));
}

#[tokio::test]
async fn settled_watch_is_parked_merged_watch_stops_without_host_read() {
    let f = Fixture::new();
    f.watch(1);
    let id = ThreadId("thread".into());
    let mut projection = f.service.kernel.store.thread(&id).unwrap().unwrap();
    projection.thread.settled_at = Some(START.into());
    projection.thread.settled_override = Some(OrchestrationV2AppThreadSettledOverride::Settled);
    f.service
        .kernel
        .store
        .dispatch(
            &Command {
                id: CommandId("settle-test".into()),
                thread_id: id.clone(),
                operation: Operation::SessionBinding(Box::new(projection.thread)),
            },
            NOW,
        )
        .unwrap();
    f.command(
        "thread",
        PrOperation::Sync {
            target: identity(1),
            snapshot: Box::new(snapshot("open", "a", "main")),
            stack: None,
            preserve_stack: false,
        },
        NOW,
    )
    .unwrap();
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    // Snapshot sync may read at the slow cadence, but never evaluates/wakes a
    // settled thread or consumes its comment budget.
    reactor.sweep().await.unwrap();
    assert_eq!(
        f.service.links(&id).unwrap()[0]
            .watch
            .as_ref()
            .unwrap()
            .wakes,
        0
    );
    assert!(
        f.service
            .kernel
            .store
            .thread(&id)
            .unwrap()
            .unwrap()
            .runs
            .is_empty()
    );
    f.command(
        "thread",
        PrOperation::Sync {
            target: identity(1),
            snapshot: Box::new(snapshot("merged", "a", "main")),
            stack: None,
            preserve_stack: false,
        },
        NOW,
    )
    .unwrap();
    let calls = f.host.calls.load(Ordering::Relaxed);
    reactor.sweep().await.unwrap();
    assert_eq!(calls, f.host.calls.load(Ordering::Relaxed));
    assert!(f.service.links(&id).unwrap()[0].watch.as_ref().is_none());
}

#[tokio::test]
async fn concurrent_link_calls_have_one_new_link_flag() {
    let f = Fixture::new();
    let id = ThreadId("thread".into());
    let (a, b) = tokio::join!(
        f.service
            .link(&id, identity(1), ThreadPullRequestLinkSource::Agent),
        f.service
            .link(&id, identity(1), ThreadPullRequestLinkSource::Agent)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    assert_eq!(f.service.links(&id).unwrap().len(), 1);
}

#[tokio::test]
async fn failed_stack_read_defers_terminal_snapshot_and_retries_before_settlement() {
    let f = Fixture::new();
    f.watch(1);
    {
        let mut read = f.host.read.lock().unwrap();
        let read = read.as_mut().unwrap();
        read.snapshot = snapshot("merged", "a", "main");
        read.stack_read_failed = true;
    }
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    reactor.sweep().await.unwrap();
    let id = ThreadId("thread".into());
    assert!(f.service.links(&id).unwrap()[0].snapshot.is_none());
    assert!(
        f.service
            .kernel
            .store
            .thread(&id)
            .unwrap()
            .unwrap()
            .thread
            .settled_at
            .is_none()
    );
    f.host
        .read
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .stack_read_failed = false;
    reactor.sweep().await.unwrap();
    assert_eq!(f.host.calls.load(Ordering::Relaxed), 2);
    assert_eq!(
        f.service.links(&id).unwrap()[0]
            .snapshot
            .as_ref()
            .unwrap()
            .state,
        PullRequestState::Merged
    );
    assert!(
        f.service
            .kernel
            .store
            .thread(&id)
            .unwrap()
            .unwrap()
            .thread
            .settled_at
            .is_some()
    );
}

#[tokio::test]
async fn unrelated_merge_does_not_settle_an_open_linked_thread() {
    let f = Fixture::new();
    f.watch(1);
    f.create("other");
    f.command(
        "other",
        PrOperation::Link {
            target: identity(2),
            source: ThreadPullRequestLinkSource::Agent,
        },
        NOW,
    )
    .unwrap();
    f.command(
        "other",
        PrOperation::Sync {
            target: identity(2),
            snapshot: Box::new(snapshot("merged", "done", "main")),
            stack: None,
            preserve_stack: false,
        },
        NOW,
    )
    .unwrap();
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    reactor.sweep().await.unwrap();
    assert!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("other".into()))
            .unwrap()
            .unwrap()
            .thread
            .settled_at
            .is_some()
    );
    assert!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("thread".into()))
            .unwrap()
            .unwrap()
            .thread
            .settled_at
            .is_none()
    );
}

#[tokio::test]
async fn unchanged_snapshot_does_not_republish_and_closed_reopens_on_slow_cadence() {
    let f = Fixture::new();
    f.command(
        "thread",
        PrOperation::Link {
            target: identity(1),
            source: ThreadPullRequestLinkSource::Agent,
        },
        NOW,
    )
    .unwrap();
    let id = ThreadId("thread".into());
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    reactor.sweep().await.unwrap();
    let version = f
        .service
        .kernel
        .store
        .thread(&id)
        .unwrap()
        .unwrap()
        .through_sequence;
    reactor.sweep().await.unwrap();
    assert_eq!(
        f.service
            .kernel
            .store
            .thread(&id)
            .unwrap()
            .unwrap()
            .through_sequence,
        version
    );
    f.host.read.lock().unwrap().as_mut().unwrap().snapshot = snapshot("closed", "a", "main");
    reactor.sweep().await.unwrap();
    let calls = f.host.calls.load(Ordering::Relaxed);
    f.host.read.lock().unwrap().as_mut().unwrap().snapshot = snapshot("open", "a", "main");
    reactor.sweep().await.unwrap();
    assert_eq!(f.host.calls.load(Ordering::Relaxed), calls);
    // A fresh reactor's absent slow-cadence clock performs the first read.
    let mut reactor = reactor::PullRequestReactor::new(f.service.clone(), Arc::new(Context));
    reactor.sweep().await.unwrap();
    assert_eq!(
        f.service.links(&id).unwrap()[0]
            .snapshot
            .as_ref()
            .unwrap()
            .state,
        PullRequestState::Open
    );
}

#[tokio::test]
async fn real_toolkit_schema_framing_and_caller_owned_link_dispatch() {
    let f = Fixture::new();
    let toolkit = crate::mcp::toolkit::Toolkit::new(Arc::new(crate::HarnessRegistry::new()));
    toolkit.set_pull_requests(f.service.clone());
    let scope = crate::mcp::auth::InvocationScope {
        environment_id: "env".into(),
        caller: f.scope(),
        selection: serde_json::from_value(json!({"instanceId":"codex","model":"gpt-5"})).unwrap(),
        capabilities: ["pull-requests".into()].into_iter().collect(),
        issued_at: 0,
        task_id: None,
    };
    let request = |name: &str, args: Value| json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}});
    for args in [json!({"number":0,"repository":"o/r"}), json!({"url":null})] {
        let reply = toolkit
            .request(scope.clone(), request("link_pull_request", args))
            .await
            .unwrap();
        assert_eq!(reply["error"]["code"], -32602);
        assert!(f.service.links(&scope.caller.thread_id).unwrap().is_empty());
    }
    let reply = toolkit
        .request(
            scope.clone(),
            request(
                "link_pull_request",
                json!({"url":"https://github.com/O/R/pull/1"}),
            ),
        )
        .await
        .unwrap();
    let result: zeron_proto::orchestration_mcp::LinkPullRequestResult =
        serde_json::from_value(reply["result"]["structuredContent"].clone()).unwrap();
    assert!(!result.already_linked);
    assert_eq!(result.repository, "o/r");
    assert_eq!(reply["result"]["isError"], false);
    let missing = toolkit
        .request(scope.clone(), request("link_pull_request", json!({})))
        .await
        .unwrap();
    assert_eq!(missing["result"]["isError"], true);
    assert_eq!(
        missing["result"]["content"][0]["text"],
        "Pass either url, or both repository and number."
    );
    let reply = toolkit
        .request(
            scope.clone(),
            request("list_thread_pull_requests", json!({})),
        )
        .await
        .unwrap();
    let list: zeron_proto::orchestration_mcp::ListThreadPullRequestsResult =
        serde_json::from_value(reply["result"]["structuredContent"].clone()).unwrap();
    assert_eq!(list.pull_requests.len(), 1);
    assert_eq!(
        f.service.links(&scope.caller.thread_id).unwrap()[0].source,
        ThreadPullRequestLinkSource::Agent
    );
}
