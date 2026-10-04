//! Passive presentation state shared by local RPC and chat2 replicas.
use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::ThreadId;

use super::{Result, projection, task};

pub(crate) fn state(conn: &Connection, id: &ThreadId) -> Result<Value> {
    let Some(parent) = projection::read_thread(conn, id)? else {
        return Ok(
            json!({"threadId":id,"workState":"result_available","tasks":[],"latestResult":null}),
        );
    };
    let mut tasks = vec![];
    for task in task::records(&parent, "subagent")
        .iter()
        .filter(|t| t["origin"] == "app_owned")
    {
        let child = task["childThreadId"]
            .as_str()
            .map(|id| projection::read_thread(conn, &ThreadId(id.into())))
            .transpose()?
            .flatten();
        let work_state = child
            .as_ref()
            .map(|c| task::progress(c).0)
            .unwrap_or("working");
        let latest = child.as_ref().and_then(|c| {
            c.runs.iter().rev().find(|r| {
                super::command::run_terminal(&r.status)
                    && r.status != zeron_proto::orchestration::OrchestrationV2RunStatus::RolledBack
                    && !task::monitor_run(c, r)
                    && (r.started_at.is_some() || r.ordinal == 1)
            })
        });
        let latest_result = child
            .as_ref()
            .zip(latest)
            .map(|(c, run)| json!(task::result_text(c, run)));
        tasks.push(json!({
            "taskId":task["id"],"childThreadId":task["childThreadId"],"title":task["title"],
            "providerInstanceId":task["providerInstanceId"],"model":task["model"],
            "status":task["status"],"workState":work_state,"result":task["result"],
            "startedAt":task["startedAt"],"completedAt":task["completedAt"],
            "latestResult":latest_result.or_else(|| task["result"].as_str().map(|s| json!(s))),
            "latestTerminalRunId":latest.map(|r| &r.id),
            "completionDelivery":task["completionDelivery"]
        }));
    }
    let latest_result = task::progress(&parent).1.and_then(|run| {
        task::records(&parent, "message")
            .iter()
            .find(|m| m["runId"] == run.id.0 && m["role"] == "assistant" && m["streaming"] == false)
            .map(|m| m["text"].clone())
    });
    Ok(
        json!({"threadId":id,"version":parent.through_sequence,"lineage":parent.thread.lineage,
        "forkedFrom":parent.thread.forked_from,"transfers":super::transfer::transfers(conn,id)?,
        "checkpoints":super::checkpoint::timeline(conn,id)?,
        "workState":task::progress(&parent).0,"tasks":tasks,"latestResult":latest_result}),
    )
}

impl super::Store {
    pub fn ui_state(&self, id: &ThreadId) -> Result<Value> {
        self.read(|conn| state(conn, id))
    }
}
