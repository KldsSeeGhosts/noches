use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use zeron_rpc::{RpcError, RpcReply, RpcService, memory_client, methods};

use crate::{Origin, Tools, Zeron};

#[derive(Default)]
struct World {
    writes: Mutex<Vec<(String, Value)>>,
    sent: Mutex<Option<(String, Instant)>>,
    remote_queries: Mutex<Vec<Value>>,
    old: bool,
    catalog_error: bool,
    never_reply: bool,
}

fn stream(value: Value) -> RpcReply {
    RpcReply::Stream(futures::stream::once(async move { value }).boxed())
}

#[async_trait]
impl RpcService for World {
    async fn handle(&self, method: &str, params: Value) -> Result<RpcReply, RpcError> {
        let sent = self.sent.lock().unwrap().clone();
        Ok(match method {
            methods::LOCAL_DEVICE => RpcReply::Value(json!({"deviceId":"local"})),
            methods::ENGINE_INFO => RpcReply::Value(
                json!({"capabilities":if self.old {vec![]} else {vec!["mcp-session-routing-v1"]}}),
            ),
            methods::WATCH_DEVICES => stream(json!([
                {"id":"local","name":"Laptop","platform":"linux","lastSeenAt":null},
                {"id":"remote","name":"Worker","platform":"linux","lastSeenAt":null}
            ])),
            methods::WATCH_SPACES => stream(json!([
                {"id":"project-local","deviceId":"local","path":"/repo/repeated","createdAt":"2026-09-01T00:00:00Z","gitDetected":true},
                {"id":"project-remote","deviceId":"remote","path":"/repo/repeated","createdAt":"2026-09-01T00:00:00Z","gitDetected":true}
            ])),
            methods::WATCH_CHATS => {
                let mut chats = vec![
                    json!({"id":"existing","deviceId":"remote","archived":false,"createdAt":"2026-09-01T00:00:00Z","cwd":"/repo/repeated","config":{"harness":"codex","sandbox":"workspace-write"}}),
                ];
                for (method, write) in self.writes.lock().unwrap().iter() {
                    if method == methods::MUTATE && write["op"] == "createChat" {
                        chats.push(json!({"id":write["chatId"],"deviceId":write["deviceId"],"archived":false,"createdAt":"2026-09-01T00:00:00Z","cwd":"/repo/repeated","config":write["config"]}));
                    }
                }
                stream(json!(chats))
            }
            methods::LIST_HARNESSES | methods::LIST_MODELS => {
                self.remote_queries.lock().unwrap().push(params.clone());
                if self.catalog_error {
                    return Err(RpcError::Failed("catalog unavailable".into()));
                }
                if method == methods::LIST_HARNESSES {
                    RpcReply::Value(if params["targetDeviceId"] == "remote" {
                        json!([{"id":"codex","installed":true},{"id":"cursor","installed":true,"enabled":false}])
                    } else {
                        json!([{"id":"claude-code","installed":true}])
                    })
                } else {
                    RpcReply::Value(json!([{"id":"remote-model","label":"Remote model"}]))
                }
            }
            methods::MUTATE | methods::QUEUE_COMMAND => {
                self.writes
                    .lock()
                    .unwrap()
                    .push((method.into(), params.clone()));
                if method == methods::QUEUE_COMMAND && params["command"]["kind"] == "run" {
                    *self.sent.lock().unwrap() =
                        Some((params["chatId"].as_str().unwrap().into(), Instant::now()));
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                RpcReply::Value(json!({"ok":true,"commandId":"command"}))
            }
            methods::WATCH_SESSIONS => stream(match sent {
                Some((id, since)) if since.elapsed() > Duration::from_millis(150) => {
                    json!([{"chatId":id,"deviceId":"remote","status":"idle","updatedAt":"2026-09-01T00:00:00Z","lastCompletedTurn":"new-turn"}])
                }
                _ => json!([]),
            }),
            methods::WATCH_DOC_MESSAGES => {
                assert_eq!(params["targetDeviceId"], "remote");
                let mut messages = vec![];
                if params["chatId"] == "existing" {
                    messages.push(json!({"id":"previous","role":"assistant","deviceId":"remote","createdAt":999999,"status":"complete","parts":[{"kind":"text","id":"t","text":"old reply"}]}));
                }
                if let Some((_, since)) = sent
                    && since.elapsed() > Duration::from_millis(300)
                    && !self.never_reply
                {
                    // A host clock behind the caller still supplies the new reply.
                    messages.push(json!({"id":"new","role":"assistant","deviceId":"remote","createdAt":1,"status":"complete","parts":[{"kind":"text","id":"t","text":"new reply"}]}));
                }
                stream(json!({"reset":messages}))
            }
            _ => return Err(RpcError::UnknownMethod(method.into())),
        })
    }
}

fn tools(world: Arc<World>) -> Tools {
    Tools::new(Arc::new(Zeron::with_client(
        memory_client(world),
        Origin::default(),
    )))
}

#[tokio::test]
async fn invalid_creation_never_writes() {
    let world = Arc::new(World::default());
    let tools = tools(world.clone());
    for args in [
        json!({"kind":"side"}),
        json!({"kind":"unknown"}),
        json!({"parent":"existing"}),
        json!({"device":"missing"}),
        json!({"project":"/repo/repeated"}),
        json!({"device":"Worker","project":"project-local"}),
        json!({"device":"Worker","harness":"claude-code"}),
        json!({"device":"Worker","harness":"cursor"}),
        json!({"device":"Worker","harness":"mock"}),
        json!({"device":"Worker","model":"local-model"}),
        json!({"device":"Worker","cwd":"/private"}),
        json!({"device":"Worker","auto_approve":true}),
        json!({"device":"Worker","sandbox":"danger-full-access"}),
    ] {
        assert!(
            tools.call("create_chat", args.clone()).await.is_err(),
            "{args}"
        );
        assert!(world.writes.lock().unwrap().is_empty(), "{args}");
    }
}

#[tokio::test]
async fn creation_uses_host_catalogs_and_preserves_approval_routing() {
    let world = Arc::new(World::default());
    let tools = tools(world.clone());
    let created = tools.call("create_chat",json!({"device":"Worker","project":"/repo/repeated","model":"remote-model","prompt":"go"})).await.unwrap();
    assert_eq!(created["deviceId"], "remote");
    assert_eq!(created["project"]["id"], "project-remote");
    assert_eq!(created["harness"], "codex");
    assert!(created["parentChatId"].is_null());
    let writes = world.writes.lock().unwrap();
    assert_eq!(writes.len(), 2);
    let request = &writes[1].1["command"]["request"];
    assert_eq!(request["autoApprove"], false);
    assert_eq!(request["sandbox"], "workspace-write");
    assert_eq!(request["cwd"], "/repo/repeated");
    assert_eq!(writes[1].1["targetDeviceId"], "remote");
    assert!(writes[0].1.get("parentChatId").is_none());
    assert!(
        world
            .remote_queries
            .lock()
            .unwrap()
            .iter()
            .all(|params| params["targetDeviceId"] == "remote")
    );
}

#[tokio::test]
async fn old_callers_and_unreachable_catalogs_fail_before_writes() {
    for old in [true, false] {
        let world = Arc::new(World {
            old,
            catalog_error: !old,
            ..Default::default()
        });
        let error = tools(world.clone())
            .call("create_chat", json!({"device":"Worker"}))
            .await
            .unwrap_err();
        assert!(
            error.contains(if old { "update" } else { "catalog unavailable" }),
            "{error}"
        );
        assert!(world.writes.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn batch_preserves_order_and_errors() {
    let world = Arc::new(World::default());
    let result = tools(world.clone()).call("create_chats",json!({"requests":[
        {"kind":"chat","device":"Worker"},{"kind":"chat","parent":"existing"},{"kind":"chat"}
    ]})).await.unwrap();
    assert_eq!(result["results"][0]["result"]["deviceId"], "remote");
    assert_eq!(result["results"][1]["isError"], true);
    assert_eq!(result["results"][2]["result"]["deviceId"], "local");
    for index in 0..3 {
        assert_eq!(result["results"][index]["index"], index);
    }
    assert_eq!(world.writes.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn waits_keep_ids_across_missing_sessions_late_replies_and_clock_skew() {
    for create in [true, false] {
        for blocking in [true, false] {
            let tools = tools(Arc::new(World::default()));
            let result = if create {
                tools
                    .call(
                        "create_chat",
                        json!({"device":"Worker","prompt":"go","wait":blocking,"timeout_secs":2}),
                    )
                    .await
                    .unwrap()
            } else {
                tools
                    .call(
                        "send_message",
                        json!({"chat":"existing","text":"go","wait":blocking,"timeout_secs":2}),
                    )
                    .await
                    .unwrap()
            };
            let turn = if blocking {
                result["turn"].clone()
            } else {
                tools
                    .call(
                        "wait_for_turn",
                        json!({"chat":result["chatId"],"timeout_secs":2}),
                    )
                    .await
                    .unwrap()["turn"]
                    .clone()
            };
            assert_eq!(turn["outcome"], "completed", "{turn}");
            assert_eq!(turn["replies"].as_array().unwrap().len(), 1);
            assert_eq!(turn["replies"][0]["id"], "new");
        }
    }
}

#[tokio::test]
async fn missing_new_reply_times_out_without_substituting_history() {
    let tools = tools(Arc::new(World {
        never_reply: true,
        ..Default::default()
    }));
    let result = tools
        .call(
            "send_message",
            json!({"chat":"existing","text":"go","wait":true,"timeout_secs":1}),
        )
        .await
        .unwrap();
    assert_eq!(result["turn"]["outcome"], "timedOut");
    assert_eq!(result["turn"]["replies"], json!([]));
}

#[tokio::test]
async fn jsonrpc_discovers_standalone_tools_without_browser_replacement() {
    let tools = tools(Arc::new(World::default()));
    let initialized = crate::jsonrpc::handle_request(
        &tools,
        json!({"id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    )
    .await;
    assert_eq!(initialized["result"]["protocolVersion"], "2024-11-05");
    let catalog = tools.list();
    assert!(catalog.iter().any(|tool| tool["name"] == "create_chat"));
    assert!(!catalog.iter().any(|tool| tool["name"] == "browser_open"));
}
