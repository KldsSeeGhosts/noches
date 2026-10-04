//! GitHub reads reuse Noches' login PATH, bounded subprocess runner and safe
//! errors. Other forge identities remain linkable without guessing host state.
use super::{
    Identity,
    identity::ProjectHost,
    watch::{Check, Detail, Remark},
};
use crate::source_control::{ChangeRequestError, ChangeRequestResolver, GitHubCli};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::Path;
use zeron_proto::orchestration::{ThreadPullRequestSnapshot, ThreadPullRequestStack};

#[derive(Clone)]
pub struct HostRead {
    pub snapshot: ThreadPullRequestSnapshot,
    pub stack: Option<ThreadPullRequestStack>,
    pub stack_read_failed: bool,
    pub detail: Detail,
    pub remarks: Option<Vec<Remark>>,
}
#[async_trait]
pub trait PullRequestHost: Send + Sync {
    async fn project_host(&self, cwd: &Path) -> Result<Option<ProjectHost>, ChangeRequestError>;
    async fn read(
        &self,
        cwd: &Path,
        target: &Identity,
        activity: bool,
    ) -> Result<HostRead, ChangeRequestError>;
    async fn branch_is_open(&self, cwd: &Path, branch: &str) -> Result<bool, ChangeRequestError>;
}

#[derive(Default)]
pub struct GitHubHost {
    resolver: ChangeRequestResolver,
    cli: GitHubCli,
}

pub fn sibling_url(url: &str, number: i64) -> Option<String> {
    let mut url = reqwest::Url::parse(url).ok()?;
    let mut parts: Vec<_> = url.path().split('/').map(str::to_owned).collect();
    let index = parts.iter().position(|s| {
        matches!(
            s.as_str(),
            "pull" | "pulls" | "pull-requests" | "merge_requests" | "pullrequest"
        )
    })?;
    parts.truncate(index + 2);
    parts[index + 1] = number.to_string();
    url.set_path(&parts.join("/"));
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn check_status(check: &Value) -> String {
    if check["status"]
        .as_str()
        .is_some_and(|s| !matches!(s.trim().to_uppercase().as_str(), "" | "COMPLETED"))
    {
        return "pending".into();
    }
    match check["conclusion"]
        .as_str()
        .or(check["state"].as_str())
        .unwrap_or("")
        .trim()
        .to_uppercase()
        .as_str()
    {
        "FAILURE" | "ERROR" | "TIMED_OUT" | "STARTUP_FAILURE" => "failure",
        "CANCELLED" => "cancelled",
        "ACTION_REQUIRED" => "action-required",
        "SUCCESS" => "success",
        "NEUTRAL" => "neutral",
        "SKIPPED" => "skipped",
        "PENDING" | "EXPECTED" => "pending",
        _ => "neutral",
    }
    .into()
}

#[async_trait]
impl PullRequestHost for GitHubHost {
    async fn project_host(&self, cwd: &Path) -> Result<Option<ProjectHost>, ChangeRequestError> {
        // A missing/unrecognised remote is not a tool backend failure.
        let Ok(source) = self.resolver.inspect_checkout(cwd).await else {
            return Ok(None);
        };
        let Some(remote_url) = source.branch.remote_url else {
            return Ok(None);
        };
        let host = source.branch.host.or_else(|| remote_host(&remote_url));
        Ok(host.map(|host| {
            let kind = if host.contains("gitlab") {
                "gitlab"
            } else if host.contains("forge") || host.contains("gitea") {
                "forgejo"
            } else if host.contains("bitbucket") {
                "bitbucket"
            } else if host.contains("azure") || host.ends_with(".visualstudio.com") {
                "azure-devops"
            } else {
                "github"
            };
            ProjectHost {
                host,
                kind: kind.into(),
                remote_url,
            }
        }))
    }
    async fn branch_is_open(&self, cwd: &Path, branch: &str) -> Result<bool, ChangeRequestError> {
        let mut source = self.resolver.inspect_checkout(cwd).await?;
        source.branch = crate::source_control::BranchHeadContext::resolve(
            branch,
            None,
            source.branch.remote_name.as_deref(),
            source.branch.remote_url.as_deref(),
        );
        Ok(self
            .resolver
            .resolve_github_source(&source)
            .await?
            .is_some_and(|p| p.state == zeron_proto::ChangeRequestState::Open))
    }
    async fn read(
        &self,
        cwd: &Path,
        target: &Identity,
        activity: bool,
    ) -> Result<HostRead, ChangeRequestError> {
        let (owner, name) = target
            .repository
            .split_once('/')
            .ok_or(ChangeRequestError::UnsupportedRepository)?;
        let required = if target.host == "github.com" {
            "isRequired(pullRequestNumber: $number)"
        } else {
            ""
        };
        let query = format!(
            r#"query($owner:String!,$name:String!,$number:Int!,$cursor:String) {{
          viewer {{ login }}
          repository(owner:$owner,name:$name) {{
            pullRequest(number:$number) {{
              state title headRefName baseRefName isDraft updatedAt closedAt mergedAt
              author {{ login }} headRefOid mergeable reviewDecision additions deletions changedFiles
              commits(last:1) {{ nodes {{ commit {{ statusCheckRollup {{
                contexts(first:100,after:$cursor) {{ nodes {{
                  ... on StatusContext {{ context state targetUrl createdAt {required} }}
                  ... on CheckRun {{ name status conclusion detailsUrl startedAt completedAt {required}
                    checkSuite {{ workflowRun {{ workflow {{ name }} }} }}
                  }}
                }} pageInfo {{ hasNextPage endCursor }} }}
              }} }} }} }}
            }}
          }}
        }}"#
        );
        let response = self
            .cli
            .read_json(
                cwd,
                vec![
                    "api".into(),
                    "graphql".into(),
                    "--hostname".into(),
                    target.host.clone(),
                    "-f".into(),
                    format!("query={query}"),
                    "-f".into(),
                    format!("owner={owner}"),
                    "-f".into(),
                    format!("name={name}"),
                    "-F".into(),
                    format!("number={}", target.number),
                ],
            )
            .await?;
        if response.get("errors").is_some() {
            return Err(ChangeRequestError::Decode);
        }
        let pr = &response["data"]["repository"]["pullRequest"];
        let contexts = &pr["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"];
        let mut context_page = contexts.clone();
        let mut check_rows = contexts["nodes"].as_array().cloned().unwrap_or_default();
        let mut cursors = std::collections::HashSet::new();
        while context_page["pageInfo"]["hasNextPage"] == true {
            let cursor = context_page["pageInfo"]["endCursor"]
                .as_str()
                .ok_or(ChangeRequestError::Decode)?;
            if !cursors.insert(cursor.to_owned()) {
                return Err(ChangeRequestError::Decode);
            }
            let page = self
                .cli
                .read_json(
                    cwd,
                    vec![
                        "api".into(),
                        "graphql".into(),
                        "--hostname".into(),
                        target.host.clone(),
                        "-f".into(),
                        format!("query={query}"),
                        "-f".into(),
                        format!("owner={owner}"),
                        "-f".into(),
                        format!("name={name}"),
                        "-F".into(),
                        format!("number={}", target.number),
                        "-f".into(),
                        format!("cursor={cursor}"),
                    ],
                )
                .await?;
            let next_pr = &page["data"]["repository"]["pullRequest"];
            if page.get("errors").is_some() || next_pr["headRefOid"] != pr["headRefOid"] {
                return Err(ChangeRequestError::Decode);
            }
            context_page =
                next_pr["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"].clone();
            check_rows.extend(
                context_page["nodes"]
                    .as_array()
                    .ok_or(ChangeRequestError::Decode)?
                    .iter()
                    .cloned(),
            );
        }
        let checks = decode_checks(&json!(check_rows));
        let mergeability = match pr["mergeable"].as_str() {
            Some("CONFLICTING") => "conflicting",
            Some("MERGEABLE") => "mergeable",
            _ => "unknown",
        };
        let state = pr["state"]
            .as_str()
            .ok_or(ChangeRequestError::Decode)?
            .to_lowercase();
        let checks_state = if checks.is_empty() {
            Value::Null
        } else if checks
            .iter()
            .any(|c| matches!(c.status.as_str(), "failure" | "cancelled"))
        {
            json!("failing")
        } else if checks
            .iter()
            .any(|c| matches!(c.status.as_str(), "pending" | "action-required"))
        {
            json!("pending")
        } else if checks.iter().any(|c| c.status == "success") {
            json!("passing")
        } else {
            Value::Null
        };
        let snapshot = serde_json::from_value(json!({
            "state":state,"title":pr["title"],"headBranch":pr["headRefName"],"baseBranch":pr["baseRefName"],
            "isDraft":pr["isDraft"],"updatedAt":pr["updatedAt"],"syncedAt":crate::orchestration::event::iso(crate::now_ms()).map_err(|_| ChangeRequestError::Decode)?,
            "closedAt":pr["closedAt"],"mergedAt":pr["mergedAt"],"mergeability":mergeability,"checksState":checks_state,
            "author":{"login":pr["author"]["login"],"name":null,"avatarUrl":null},
            "additions":pr["additions"],"deletions":pr["deletions"],"changedFiles":pr["changedFiles"],
            "reviewDecision":match pr["reviewDecision"].as_str() { Some("APPROVED") => json!("approved"), Some("CHANGES_REQUESTED") => json!("changes-requested"), Some("REVIEW_REQUIRED") => json!("review-required"), _ => Value::Null }
        })).map_err(|_| ChangeRequestError::Decode)?;
        let api = |endpoint: String| {
            vec![
                "api".into(),
                "--hostname".into(),
                target.host.clone(),
                endpoint,
            ]
        };
        // 404 means no stacks preview; every other failure is retried rather
        // than clearing a previously known stack.
        let stack = self
            .cli
            .read_json(
                cwd,
                api(format!(
                    "repos/{}/stacks?pull_request={}",
                    target.repository, target.number
                )),
            )
            .await;
        let stack = match stack {
            Ok(value) => {
                let decoded = decode_stack(&value);
                if let Ok(Some(stack)) = decoded {
                    // The listing can omit details. T3 reads the stack itself.
                    let detail = self
                        .cli
                        .read_json(
                            cwd,
                            api(format!(
                                "repos/{}/stacks/{}",
                                target.repository, stack.number
                            )),
                        )
                        .await;
                    match detail {
                        Ok(value) => decode_stack(&json!([value])),
                        Err(ChangeRequestError::NotFound) => Ok(None),
                        Err(error) => Err(error),
                    }
                } else {
                    decoded
                }
            }
            Err(ChangeRequestError::NotFound) => Ok(None),
            Err(error) => Err(error),
        };
        let stack_read_failed = stack.is_err();
        let stack = stack.unwrap_or_default();
        let viewer = response["data"]["viewer"]["login"]
            .as_str()
            .map(str::to_owned);
        let remarks = if activity {
            super::activity::read(&self.cli, cwd, target).await?
        } else {
            None
        };
        Ok(HostRead {
            snapshot,
            stack,
            stack_read_failed,
            detail: Detail {
                head_sha: pr["headRefOid"].as_str().map(str::to_owned),
                checks,
                mergeability: mergeability.into(),
                viewer,
                author: pr["author"]["login"].as_str().map(str::to_owned),
            },
            remarks,
        })
    }
}

fn remote_host(remote: &str) -> Option<String> {
    if let Ok(url) = reqwest::Url::parse(remote) {
        return matches!(url.scheme(), "https" | "http" | "ssh")
            .then(|| url.host_str().map(str::to_owned))
            .flatten();
    }
    let (authority, path) = remote.split_once(':')?;
    if authority.contains('/') || !path.contains('/') {
        return None;
    }
    Some(authority.rsplit('@').next()?.to_lowercase())
}

fn decode_checks(rows: &Value) -> Vec<Check> {
    let mut newest: Vec<(String, Option<String>, Option<String>, Check)> = vec![];
    for row in rows.as_array().into_iter().flatten() {
        let nonempty = |value: &Value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let Some(name) = nonempty(&row["name"]).or_else(|| nonempty(&row["context"])) else {
            continue;
        };
        let workflow = nonempty(&row["checkSuite"]["workflowRun"]["workflow"]["name"]);
        let time = nonempty(&row["completedAt"])
            .filter(|s| s != "0001-01-01T00:00:00Z")
            .or_else(|| nonempty(&row["startedAt"]));
        let key = format!("{} {name}", workflow.as_deref().unwrap_or(""));
        let check = Check {
            name,
            status: check_status(row),
            required: row["isRequired"] == true,
            url: nonempty(&row["detailsUrl"]).or_else(|| nonempty(&row["targetUrl"])),
        };
        if let Some(existing) = newest.iter_mut().find(|(k, _, _, _)| *k == key) {
            if time >= existing.2 {
                *existing = (key, workflow, time, check);
            }
        } else {
            newest.push((key, workflow, time, check));
        }
    }
    newest
        .iter()
        .map(|(_, workflow, _, check)| {
            let mut result = check.clone();
            if let Some(workflow) = workflow
                && newest
                    .iter()
                    .filter(|(_, _, _, c)| c.name == check.name)
                    .count()
                    > 1
            {
                result.name = format!("{workflow} / {}", check.name);
            }
            result
        })
        .collect()
}

fn decode_stack(value: &Value) -> Result<Option<ThreadPullRequestStack>, ChangeRequestError> {
    let stack = value
        .as_array()
        .and_then(|v| v.first())
        .or_else(|| value["stacks"].as_array().and_then(|v| v.first()));
    let Some(stack) = stack else {
        return Ok(None);
    };
    if !stack["pull_requests"].is_array() {
        return Err(ChangeRequestError::Decode);
    }
    serde_json::from_value(json!({
        "kind":"native","id":if stack["id"].is_null() { stack["node_id"].as_str().map(str::to_owned).unwrap_or_else(|| stack["number"].to_string()) } else { stack["id"].as_str().map(str::to_owned).unwrap_or_else(|| stack["id"].to_string()) },"number":stack["number"],
        "url":stack["html_url"].as_str().or(stack["url"].as_str()),"base":stack["base"].as_str().or(stack["base"]["ref"].as_str()),
        "layers":stack["pull_requests"].as_array().into_iter().flatten().map(|l| json!({
            "number":l["number"],"headBranch":l["head"]["ref"],"state":if l["merged_at"].is_string() {"merged"} else {l["state"].as_str().unwrap_or("open")}
        })).collect::<Vec<_>>()
    })).map(Some).map_err(|_| ChangeRequestError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oracle_check_deduplication_reruns_and_workflow_qualified_names() {
        let row = |name: &str, workflow: &str, status: &str, at: &str| {
            json!({
                "name":name,"status":if status == "PENDING" {"IN_PROGRESS"} else {"COMPLETED"},
                "conclusion":status,"completedAt":at,"isRequired":true,
                "checkSuite":{"workflowRun":{"workflow":{"name":workflow}}}
            })
        };
        let rows = json!([
            row("test","ci","FAILURE","2026-10-02T12:00:00Z"),
            row("test","ci","PENDING","2026-10-02T12:01:00Z"),
            row("test","release","SUCCESS","2026-10-02T12:01:00Z"),
            {"context":"status","state":"EXPECTED"},{"conclusion":"FAILURE"}
        ]);
        let checks = decode_checks(&rows);
        assert_eq!(
            checks.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["ci / test", "release / test", "status"]
        );
        assert_eq!(checks[0].status, "pending");
        assert_eq!(checks[1].status, "success");
        assert!(checks[0].required);
        assert_eq!(
            check_status(&json!({"status":"COMPLETED","conclusion":"NEW_CONCLUSION"})),
            "neutral"
        );
        assert_eq!(
            check_status(&json!({"status":"QUEUED","conclusion":"FAILURE"})),
            "pending"
        );
    }
    #[test]
    fn oracle_stack_preview_shape_and_merged_at_outranks_state() {
        let raw = json!([{"id":42,"number":3,"node_id":"STK","url":"https://api.github.com/repos/acme/web/stacks/3",
        "base":{"ref":"main","sha":"abc"},"pull_requests":[
            {"number":10,"head":{"ref":"feat/one"},"state":"closed","merged_at":"2026-09-02T00:00:00Z"},
            {"number":11,"head":{"ref":"feat/two"},"state":"open","merged_at":null},
            {"number":12,"head":{"ref":"feat/three"},"state":"closed","merged_at":null}
        ]}]);
        let stack = decode_stack(&raw).unwrap().unwrap();
        assert_eq!(stack.id, "42");
        assert_eq!(stack.base, "main");
        assert_eq!(stack.url, "https://api.github.com/repos/acme/web/stacks/3");
        assert_eq!(
            stack.layers.iter().map(|l| l.number).collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
        assert_eq!(
            stack.layers[0].state,
            zeron_proto::orchestration::PullRequestState::Merged
        );
        assert!(decode_stack(&json!([])).unwrap().is_none());
        assert!(decode_stack(&json!([{"number":3,"base":"main"}])).is_err());
    }
    #[test]
    fn nested_self_hosted_ssh_remote_retains_host() {
        assert_eq!(
            remote_host("git@gitlab.example:group/sub/project.git").as_deref(),
            Some("gitlab.example")
        );
        assert_eq!(
            remote_host("https://forge.example:3000/git/owner/repo.git").as_deref(),
            Some("forge.example")
        );
        assert!(remote_host("/tmp/project").is_none());
    }
}
