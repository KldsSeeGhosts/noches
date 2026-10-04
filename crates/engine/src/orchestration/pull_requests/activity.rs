//! T3's activity policy: whole issue/review streams, at most ten pages of
//! review threads, first ten comments per thread. A failed thread walk defers
//! all remarks without treating successful detail/activity reads as failures.
use super::{Identity, watch::Remark};
use crate::source_control::{ChangeRequestError, GitHubCli};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

const THREAD_QUERY: &str = r#"query($owner:String!,$name:String!,$number:Int!,$cursor:String) {
  repository(owner:$owner,name:$name) { pullRequest(number:$number) {
    reviewThreads(first:100,after:$cursor) {
      pageInfo { hasNextPage endCursor }
      nodes { path comments(first:10) {
        pageInfo { hasNextPage endCursor }
        nodes { id author { login } body createdAt url }
      } }
    }
    reviewDismissals:timelineItems(itemTypes:[REVIEW_DISMISSED_EVENT],first:100) {
      pageInfo { hasNextPage endCursor }
      nodes { ... on ReviewDismissedEvent { dismissalMessage review { id } } }
    }
  } }
}"#;
const DISMISSAL_QUERY: &str = r#"query($owner:String!,$name:String!,$number:Int!,$cursor:String) {
  repository(owner:$owner,name:$name) { pullRequest(number:$number) {
    reviewDismissals:timelineItems(itemTypes:[REVIEW_DISMISSED_EVENT],first:100,after:$cursor) {
      pageInfo { hasNextPage endCursor }
      nodes { ... on ReviewDismissedEvent { dismissalMessage review { id } } }
    }
  } }
}"#;

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn remark(row: &Value, review: bool, path: Option<String>) -> Result<Remark, ChangeRequestError> {
    Ok(Remark {
        id: row["id"].as_str().ok_or(ChangeRequestError::Decode)?.into(),
        author: text(&row["author"]["login"]),
        body: row["body"].as_str().unwrap_or("").into(),
        created_at: row[if review { "submittedAt" } else { "createdAt" }]
            .as_str()
            .ok_or(ChangeRequestError::Decode)?
            .into(),
        url: text(&row["url"]),
        path,
        review_state: review.then(|| text(&row["state"])).flatten(),
    })
}
fn core_remarks(value: &Value) -> Result<Vec<Remark>, ChangeRequestError> {
    let mut remarks = vec![];
    for row in value["comments"].as_array().into_iter().flatten() {
        remarks.push(remark(row, false, None)?);
    }
    for row in value["reviews"].as_array().into_iter().flatten() {
        let verdict = matches!(
            text(&row["state"]).map(|s| s.to_uppercase()).as_deref(),
            Some("APPROVED" | "CHANGES_REQUESTED" | "DISMISSED")
        );
        if text(&row["submittedAt"]).is_none() || (text(&row["body"]).is_none() && !verdict) {
            continue;
        }
        remarks.push(remark(row, true, None)?);
    }
    Ok(remarks)
}
fn next_cursor(connection: &Value) -> Result<Option<String>, ChangeRequestError> {
    if connection["pageInfo"]["hasNextPage"] != true {
        return Ok(None);
    }
    text(&connection["pageInfo"]["endCursor"])
        .map(Some)
        .ok_or(ChangeRequestError::Decode)
}
fn dismissals(connection: &Value, entries: &mut HashMap<String, String>) {
    for row in connection["nodes"].as_array().into_iter().flatten() {
        if let (Some(id), Some(body)) = (text(&row["review"]["id"]), text(&row["dismissalMessage"]))
        {
            entries.insert(id, body);
        }
    }
}
#[derive(Default)]
struct ThreadRemarks {
    remarks: Vec<Remark>,
    dismissals: HashMap<String, String>,
    truncated: bool,
    long_thread: bool,
}
impl ThreadRemarks {
    fn append(&mut self, connection: &Value) -> Result<(), ChangeRequestError> {
        for thread in connection["nodes"]
            .as_array()
            .ok_or(ChangeRequestError::Decode)?
        {
            let comments = &thread["comments"];
            self.long_thread |= next_cursor(comments)?.is_some();
            self.truncated |= self.long_thread;
            for row in comments["nodes"]
                .as_array()
                .ok_or(ChangeRequestError::Decode)?
                .iter()
                .take(10)
            {
                self.remarks
                    .push(remark(row, false, text(&thread["path"]))?);
            }
        }
        Ok(())
    }
}
fn combine(
    mut core: Vec<Remark>,
    threads: Result<ThreadRemarks, ChangeRequestError>,
) -> Option<Vec<Remark>> {
    let threads = threads.ok()?;
    // Same degraded predicate as PullRequestWatchReactor: a long conversation
    // explains truncation; an incomplete thread listing by itself does not.
    if threads.truncated && !threads.long_thread {
        return None;
    }
    let html_comment = regex::Regex::new(r"(?s)<!--.*?-->").expect("constant regex");
    for review in &mut core {
        if review
            .review_state
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("DISMISSED"))
            && html_comment.replace_all(&review.body, "").trim().is_empty()
            && let Some(body) = threads.dismissals.get(&review.id)
        {
            review.body = body.clone();
        }
    }
    core.extend(threads.remarks);
    core.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Some(core)
}

async fn graphql(
    cli: &GitHubCli,
    cwd: &Path,
    target: &Identity,
    query: &str,
    cursor: Option<&str>,
) -> Result<Value, ChangeRequestError> {
    let (owner, name) = target
        .repository
        .split_once('/')
        .ok_or(ChangeRequestError::UnsupportedRepository)?;
    let mut args = vec![
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
    ];
    if let Some(cursor) = cursor {
        args.extend(["-f".into(), format!("cursor={cursor}")]);
    }
    let response = cli.read_json(cwd, args).await?;
    if response.get("errors").is_some() {
        return Err(ChangeRequestError::Decode);
    }
    let pr = response["data"]["repository"]["pullRequest"].clone();
    if pr.is_null() {
        return Err(ChangeRequestError::Decode);
    }
    Ok(pr)
}
async fn thread_remarks(
    cli: &GitHubCli,
    cwd: &Path,
    target: &Identity,
) -> Result<ThreadRemarks, ChangeRequestError> {
    let mut threads = ThreadRemarks::default();
    let mut cursor = None;
    let mut dismissal_cursor = None;
    for page in 0..10 {
        let pr = graphql(cli, cwd, target, THREAD_QUERY, cursor.as_deref()).await?;
        threads.append(&pr["reviewThreads"])?;
        if page == 0 {
            dismissals(&pr["reviewDismissals"], &mut threads.dismissals);
            dismissal_cursor = next_cursor(&pr["reviewDismissals"])?;
        }
        cursor = next_cursor(&pr["reviewThreads"])?;
        if cursor.is_none() {
            break;
        }
    }
    threads.truncated |= cursor.is_some();
    for _ in 0..10 {
        let Some(cursor) = dismissal_cursor.as_deref() else {
            break;
        };
        let pr = graphql(cli, cwd, target, DISMISSAL_QUERY, Some(cursor)).await?;
        dismissals(&pr["reviewDismissals"], &mut threads.dismissals);
        dismissal_cursor = next_cursor(&pr["reviewDismissals"])?;
    }
    Ok(threads)
}
pub async fn read(
    cli: &GitHubCli,
    cwd: &Path,
    target: &Identity,
) -> Result<Option<Vec<Remark>>, ChangeRequestError> {
    let (core, threads) = tokio::join!(
        cli.read_json(
            cwd,
            vec![
                "pr".into(),
                "view".into(),
                target.number.to_string(),
                "--repo".into(),
                format!("{}/{}", target.host, target.repository),
                "--json".into(),
                "author,comments,reviews,commits".into(),
            ]
        ),
        thread_remarks(cli, cwd, target)
    );
    Ok(combine(core_remarks(&core?)?, threads))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn oracle_review_containers_verdicts_pending_and_node_ids() {
        let review = |id, body, state, at| {
            json!({
                "id":id,"body":body,"state":state,"submittedAt":at,"author":{"login":"reviewer"}
            })
        };
        let remarks = core_remarks(&json!({
            "comments":[{"id":"IC_1","body":"hello","createdAt":"2026-10-02T12:00:00Z"}],
            "reviews":[
                review("R_1","","COMMENTED",Some("2026-10-02T12:01:00Z")),
                review("R_2","","APPROVED",Some("2026-10-02T12:02:00Z")),
                review("R_3","unfinished","PENDING",None),
                review("R_4","<!-- bot -->","DISMISSED",Some("2026-10-02T12:04:00Z"))
            ]
        }))
        .unwrap();
        assert_eq!(
            remarks.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["IC_1", "R_2", "R_4"]
        );
        let threads = ThreadRemarks {
            dismissals: HashMap::from([("R_4".into(), "stale review".into())]),
            ..Default::default()
        };
        let combined = combine(remarks, Ok(threads)).unwrap();
        assert_eq!(combined.last().unwrap().body, "stale review");
    }
    #[test]
    fn oracle_first_ten_replies_and_degraded_thread_walk() {
        assert!(combine(vec![], Err(ChangeRequestError::CommandFailed)).is_none());
        assert!(
            combine(
                vec![],
                Ok(ThreadRemarks {
                    truncated: true,
                    ..Default::default()
                })
            )
            .is_none()
        );
        let mut threads = ThreadRemarks::default();
        threads
            .append(&json!({"nodes":[{
                "path":"src/lib.rs","comments":{
                    "pageInfo":{"hasNextPage":true,"endCursor":"eleventh"},
                    "nodes":(0..12).map(|i| json!({
                        "id":format!("RC_{i}"),"body":"reply","createdAt":"2026-10-02T12:00:00Z"
                    })).collect::<Vec<_>>()
                }
            }]}))
            .unwrap();
        let remarks = combine(vec![], Ok(threads)).unwrap();
        assert_eq!(remarks.len(), 10);
        assert_eq!(remarks[0].path.as_deref(), Some("src/lib.rs"));
        assert_eq!(remarks[9].id, "RC_9");
        assert!(next_cursor(&json!({"pageInfo":{"hasNextPage":true}})).is_err());
    }
}
