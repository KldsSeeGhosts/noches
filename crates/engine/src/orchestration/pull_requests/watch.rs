//! Pure port of T3 `pullRequestWatch.ts`; no host reads or effects here.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeron_proto::orchestration::ThreadPullRequestWatch;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub url: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Remark {
    pub id: String,
    pub author: Option<String>,
    pub body: String,
    pub created_at: String,
    pub url: Option<String>,
    pub path: Option<String>,
    pub review_state: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct Detail {
    pub head_sha: Option<String>,
    pub checks: Vec<Check>,
    pub mergeability: String,
    pub viewer: Option<String>,
    pub author: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Failed(Vec<Check>),
    Passed { count: usize, required: bool },
    Remarks(Vec<Remark>),
    Conflicting,
}
#[derive(Debug, Clone)]
pub struct Report {
    pub changes: Vec<Change>,
    pub next: ThreadPullRequestWatch,
    pub exhausted: bool,
}
#[derive(Debug, Clone)]
pub struct Wake {
    pub text: String,
    pub notification: Value,
}

fn failed(check: &Check) -> bool {
    matches!(
        check.status.as_str(),
        "failure" | "cancelled" | "action-required"
    )
}
pub fn millis(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.timestamp_millis())
}

pub fn evaluate(
    watch: &ThreadPullRequestWatch,
    detail: &Detail,
    remarks: Option<&[Remark]>,
) -> Report {
    let mut changes = vec![];
    let moved = detail.head_sha != watch.head_sha;
    let mut next = watch.clone();
    next.head_sha = detail.head_sha.clone();
    if moved {
        next.failed_checks.clear();
        next.passed = false;
    }
    if !detail.checks.is_empty() {
        let failures: Vec<_> = detail
            .checks
            .iter()
            .filter(|c| failed(c))
            .cloned()
            .collect();
        let fresh: Vec<_> = failures
            .iter()
            .filter(|c| !next.failed_checks.contains(&c.name))
            .cloned()
            .collect();
        if !fresh.is_empty() {
            changes.push(Change::Failed(fresh));
        }
        next.failed_checks = failures.into_iter().map(|c| c.name).collect();
        let required: Vec<_> = detail.checks.iter().filter(|c| c.required).collect();
        let gate = if required.is_empty() {
            detail.checks.iter().collect::<Vec<_>>()
        } else {
            required.clone()
        };
        let passed = gate.iter().all(|c| c.status != "pending" && !failed(c));
        if passed && !next.passed {
            changes.push(Change::Passed {
                count: gate.len(),
                required: !required.is_empty(),
            });
        }
        next.passed = passed;
    }
    let own = detail
        .viewer
        .as_ref()
        .or(detail.author.as_ref())
        .map(|s| s.to_lowercase());
    let through = millis(&watch.remarks_through);
    let fresh: Vec<_> = remarks
        .unwrap_or_default()
        .iter()
        .filter(|r| {
            let at = millis(&r.created_at);
            at.is_some()
                && through.is_some()
                && (at > through || (at == through && !watch.remark_ids.contains(&r.id)))
                && r.author.as_ref().map(|s| s.to_lowercase()) != own
        })
        .cloned()
        .collect();
    if !fresh.is_empty() {
        let latest = fresh
            .iter()
            .filter_map(|r| millis(&r.created_at))
            .chain(through)
            .max();
        let at_latest: Vec<_> = fresh
            .iter()
            .filter(|r| millis(&r.created_at) == latest)
            .collect();
        if latest != through {
            next.remarks_through = at_latest[0].created_at.clone();
            next.remark_ids.clear();
        }
        next.remark_ids
            .extend(at_latest.iter().map(|r| r.id.clone()));
        changes.push(Change::Remarks(fresh));
    }
    if detail.mergeability == "conflicting" && !watch.conflicting {
        changes.push(Change::Conflicting);
    }
    next.conflicting = if detail.mergeability == "unknown" {
        watch.conflicting
    } else {
        detail.mergeability == "conflicting"
    };
    let comments_only =
        !changes.is_empty() && changes.iter().all(|c| matches!(c, Change::Remarks(_)));
    let progress = moved || (!changes.is_empty() && !comments_only);
    next.wakes = if progress { 0 } else { watch.wakes } + i64::from(comments_only);
    Report {
        exhausted: comments_only && next.wakes >= 10,
        changes,
        next,
    }
}

fn listed<T>(items: &[T], line: impl Fn(&T) -> String) -> Vec<String> {
    let mut lines: Vec<_> = items.iter().take(10).map(line).collect();
    if items.len() > 10 {
        lines.push(format!("  - and {} more", items.len() - 10));
    }
    lines
}
fn snippet(body: &str) -> String {
    let no_comments = regex::Regex::new(r"(?s)<!--.*?-->")
        .unwrap()
        .replace_all(body, " ")
        .into_owned();
    let text = no_comments.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.encode_utf16().count() <= 200 {
        text
    } else {
        // Upstream slices JS UTF-16, including a potential split surrogate.
        format!(
            "{}...",
            String::from_utf16_lossy(&text.encode_utf16().take(197).collect::<Vec<_>>())
        )
    }
}
pub fn message(number: i64, url: &str, base: &str, report: &Report) -> Wake {
    let commit = report
        .next
        .head_sha
        .as_ref()
        .map(|s| format!(" on {}", s.chars().take(7).collect::<String>()))
        .unwrap_or_default();
    let mut lines = vec![format!(
        "Update on pull request #{number} ({url}), which T3 Code is watching for you:"
    )];
    let mut summary = vec![];
    for change in &report.changes {
        match change {
            Change::Failed(checks) => {
                summary.push("checks failed");
                lines.push(format!("- Checks failed{commit}:"));
                lines.extend(listed(checks, |c| {
                    format!(
                        "  - {}{}{}",
                        c.name,
                        if c.status == "failure" {
                            "".into()
                        } else {
                            format!(" ({})", c.status)
                        },
                        c.url
                            .as_ref()
                            .filter(|u| !u.is_empty())
                            .map(|u| format!(" {u}"))
                            .unwrap_or_default()
                    )
                }));
            }
            Change::Passed { count, required } => {
                summary.push("checks passed");
                lines.push(format!(
                    "- All {count} {}{} passed{commit}.",
                    if *required { "required " } else { "" },
                    if *count == 1 { "check" } else { "checks" }
                ));
            }
            Change::Remarks(remarks) => {
                summary.push("new comments");
                lines.push(format!(
                    "- {} new {}:",
                    remarks.len(),
                    if remarks.len() == 1 {
                        "comment"
                    } else {
                        "comments"
                    }
                ));
                lines.extend(listed(remarks, |r| {
                    let body = snippet(&r.body);
                    let said = if body.is_empty() {
                        r.review_state.clone().unwrap_or_else(|| "reviewed".into())
                    } else {
                        format!("\"{body}\"")
                    };
                    format!(
                        "  - {}{}: {}{}",
                        r.author.as_deref().unwrap_or("someone"),
                        r.path
                            .as_ref()
                            .map(|p| format!(" on {p}"))
                            .unwrap_or_default(),
                        said,
                        r.url
                            .as_ref()
                            .filter(|u| !u.is_empty())
                            .map(|u| format!(" {u}"))
                            .unwrap_or_default()
                    )
                }));
            }
            Change::Conflicting => {
                summary.push("merge conflict");
                lines.push(format!("- The branch now conflicts with {base}."));
            }
        }
    }
    lines.push("".into());
    lines.push(if report.exhausted {
        summary.push("stopped watching");
        "T3 Code stopped watching after 10 comment-only updates in a row. Call watch_pull_request to watch it again."
    } else {
        "Look into each item and act on it as your task requires. T3 Code keeps watching and wakes you on the next change, so end your turn when you are done. Call unwatch_pull_request when you no longer need updates."
    }.into());
    let outcome = if report
        .changes
        .iter()
        .any(|c| matches!(c, Change::Failed(_) | Change::Conflicting))
    {
        "failed"
    } else if report
        .changes
        .iter()
        .all(|c| matches!(c, Change::Passed { .. }))
    {
        "completed"
    } else {
        "updated"
    };
    Wake {
        text: lines.join("\n"),
        notification: json!({"source":{"kind":"monitor"},"outcome":outcome,"summary":format!("#{number}: {}",summary.join(", "))}),
    }
}
