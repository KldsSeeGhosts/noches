use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use zeron_doc::{MessageRole, MessageStatus, SessionCommandPayload};
use zeron_proto::{
    Chat, ChatConfig, HarnessId, ReasoningLevel, RunRequest, SandboxLevel, SessionStatus,
    UserInputAnswer,
};
use zeron_rpc::methods;

use crate::Zeron;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    kind: Option<String>,
    parent: Option<String>,
    project: Option<String>,
    device: Option<String>,
    harness: Option<HarnessId>,
    model: Option<String>,
    reasoning: Option<ReasoningLevel>,
    sandbox: Option<SandboxLevel>,
    title: Option<String>,
    prompt: Option<String>,
    #[serde(default)]
    wait: bool,
    timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendArgs {
    chat: String,
    text: String,
    #[serde(default)]
    wait: bool,
    timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitArgs {
    chat: String,
    timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputArgs {
    chat: String,
    request_id: String,
    answers: Vec<UserInputAnswer>,
}

#[derive(Clone)]
struct PendingTurn {
    message_ids: Vec<String>,
}

pub struct Tools {
    zeron: Arc<Zeron>,
    pending: Mutex<HashMap<String, Arc<PendingTurn>>>,
    // Serialize dispatch, not waits, so each send snapshots the preceding send.
    dispatch: Mutex<()>,
}

fn timeout(secs: Option<u64>) -> Duration {
    Duration::from_secs(secs.unwrap_or(120).clamp(1, 600))
}

fn key(args: &Value, name: &str) -> anyhow::Result<String> {
    Ok(args[name]
        .as_str()
        .filter(|key| !key.trim().is_empty())
        .with_context(|| format!("{name} is required"))?
        .trim()
        .into())
}

impl Tools {
    pub fn new(zeron: Arc<Zeron>) -> Self {
        Self {
            zeron,
            pending: Default::default(),
            dispatch: Default::default(),
        }
    }

    pub fn list(&self) -> Vec<Value> {
        let device = json!({"type":"string","description":"Listed device id or exact name. Omit for the local engine."});
        let chat =
            json!({"type":"string","description":"Chat id, unique id prefix or exact title."});
        let create = json!({
            "kind":{"type":"string","enum":["chat"],"description":"Only standalone sessions are supported in Noches."},
            "project":{"type":"string","description":"Listed project id, path or name, scoped to device when given."},
            "device":device,
            "harness":{"type":"string"},
            "model":{"type":"string"},
            "reasoning":{"type":"string"},
            "sandbox":{"type":"string","enum":["read-only","workspace-write"],"default":"workspace-write"},
            "title":{"type":"string"},
            "prompt":{"type":"string"},
            "wait":{"type":"boolean","default":false},
            "timeout_secs":{"type":"integer","minimum":1,"maximum":600}
        });
        [
            ("whoami", "Local engine identity and optional originating conversation.", json!({}), vec![]),
            ("list_devices", "Devices in this engine's workspace.", json!({}), vec![]),
            ("list_projects", "Known project folders, optionally filtered to one device.", json!({"device":device}), vec![]),
            ("list_harnesses", "Installed and enabled harnesses on the selected host.", json!({"device":device}), vec![]),
            ("list_models", "Models offered by a harness on the selected host.", json!({"device":device,"harness":{"type":"string"}}), vec!["harness"]),
            ("list_chats", "Sessions in this workspace.", json!({}), vec![]),
            ("create_chat", "Create a standalone session on a listed device/project. Validates that host's catalogs before writing. Approval remains interactive. No arbitrary cwd or side-chat parents.", create.clone(), vec![]),
            ("create_chats", "Create a batch of standalone sessions; results keep request order and per-request errors. Successful requests are not rolled back.", json!({"requests":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"object","properties":create,"additionalProperties":false}}}), vec!["requests"]),
            ("read_chat", "Read the transcript from the session's owning host.", json!({"chat":chat}), vec!["chat"]),
            ("send_message", "Start an idle session turn. Working sessions must finish or be interrupted first.", json!({"chat":chat,"text":{"type":"string"},"wait":{"type":"boolean"},"timeout_secs":{"type":"integer"}}), vec!["chat","text"]),
            ("wait_for_turn", "Wait for a send on this connection, preserving its message-id baseline. Otherwise report current posture.", json!({"chat":chat,"timeout_secs":{"type":"integer"}}), vec!["chat"]),
            ("interrupt_chat", "Deliver interrupt through the existing host command queue.", json!({"chat":chat}), vec!["chat"]),
            ("respond_to_input", "Answer a pending question or approval through the existing durable command route.", json!({"chat":chat,"request_id":{"type":"string"},"answers":{"type":"array"}}), vec!["chat","request_id","answers"]),
        ].into_iter().map(|(name, description, properties, required)| json!({
            "name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}
        })).collect()
    }

    pub async fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        self.execute(name, args)
            .await
            .map_err(|error| format!("{error:#}"))
    }

    async fn execute(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        ensure!(args.is_object(), "arguments must be an object");
        if name == "create_chats" {
            let requests = args["requests"]
                .as_array()
                .context("requests must be an array")?;
            ensure!(
                !requests.is_empty() && requests.len() <= 32,
                "batch size must be 1..32"
            );
            let results = futures::future::join_all(requests.iter().enumerate().map(
                |(index, request)| async move {
                    let result = match serde_json::from_value::<CreateArgs>(request.clone()) {
                        Ok(args) => self
                            .create(args)
                            .await
                            .map_err(|error| format!("{error:#}")),
                        Err(error) => Err(error.to_string()),
                    };
                    match result {
                        Ok(result) => json!({"index":index,"result":result,"isError":false}),
                        Err(error) => json!({"index":index,"error":error,"isError":true}),
                    }
                },
            ))
            .await;
            return Ok(json!({"results":results}));
        }
        match name {
            "whoami" => Ok(
                json!({"engine":self.zeron.call(methods::ENGINE_INFO,json!({})).await?,"chatId":self.zeron.origin.chat_id,"deviceId":self.zeron.origin.device_id}),
            ),
            "list_devices" => Ok(json!({"devices":self.zeron.devices().await?})),
            "list_projects" => {
                let mut spaces = self.zeron.spaces().await?;
                if let Some(device) = args["device"].as_str() {
                    let device = self.zeron.device(Some(device)).await?;
                    spaces.retain(|space| space.device_id == device);
                }
                Ok(json!({"projects":spaces}))
            }
            "list_harnesses" => {
                let device = self.zeron.device(args["device"].as_str()).await?;
                self.zeron.require_routing(&device).await?;
                let harnesses = self
                    .zeron
                    .call(methods::LIST_HARNESSES, json!({"targetDeviceId":device}))
                    .await?;
                Ok(json!({"deviceId":device,"harnesses":harnesses}))
            }
            "list_models" => {
                let device = self.zeron.device(args["device"].as_str()).await?;
                let harness = serde_json::from_value::<HarnessId>(json!(key(&args, "harness")?))?;
                Ok(json!({"deviceId":device,"models":self.zeron.models(harness,&device).await?}))
            }
            "list_chats" => Ok(json!({"chats":self.zeron.chats().await?})),
            "create_chat" => self.create(serde_json::from_value(args)?).await,
            "read_chat" => {
                let chat = self.zeron.chat(&key(&args, "chat")?).await?;
                Ok(json!({"chatId":chat.id,"messages":self.zeron.transcript(&chat).await?}))
            }
            "send_message" => self.send(serde_json::from_value(args)?).await,
            "wait_for_turn" => {
                let args: WaitArgs = serde_json::from_value(args)?;
                let chat = self.zeron.chat(&args.chat).await?;
                let pending = self.pending.lock().await.get(&chat.id).cloned();
                self.wait(&chat, pending, timeout(args.timeout_secs)).await
            }
            "interrupt_chat" => {
                let chat = self.zeron.chat(&key(&args, "chat")?).await?;
                self.queue(&chat, SessionCommandPayload::Interrupt {}).await
            }
            "respond_to_input" => {
                let args: InputArgs = serde_json::from_value(args)?;
                let chat = self.zeron.chat(&args.chat).await?;
                self.queue(
                    &chat,
                    SessionCommandPayload::RespondInput {
                        request_id: args.request_id,
                        answers: args.answers,
                    },
                )
                .await
            }
            _ => anyhow::bail!("unknown tool: {name}"),
        }
    }

    async fn create(&self, args: CreateArgs) -> anyhow::Result<Value> {
        ensure!(
            args.kind.as_deref().is_none_or(|kind| kind == "chat"),
            "Noches supports only kind chat; side chats are not implemented"
        );
        ensure!(
            args.parent
                .as_deref()
                .is_none_or(|parent| parent.trim().is_empty()),
            "standalone sessions cannot have a parent"
        );
        let (space, device) = self
            .zeron
            .target(args.project.as_deref(), args.device.as_deref())
            .await?;
        let harnesses = self.zeron.harnesses(&device).await?;
        let harness = match args.harness {
            Some(id) => {
                ensure!(
                    harnesses
                        .iter()
                        .any(|info| info.id == id && info.available()),
                    "harness {id:?} is unavailable on device {device}"
                );
                id
            }
            None => {
                harnesses
                    .iter()
                    .find(|info| info.id == HarnessId::ClaudeCode && info.available())
                    .or_else(|| harnesses.iter().find(|info| info.available()))
                    .context("no available harness on selected device")?
                    .id
            }
        };
        if let Some(model) = &args.model {
            let models = self.zeron.models(harness, &device).await?;
            ensure!(
                models.iter().any(|offered| offered.id == *model),
                "model {model:?} is not offered on device {device}"
            );
        }
        let sandbox = args.sandbox.unwrap_or(SandboxLevel::WorkspaceWrite);
        ensure!(
            sandbox != SandboxLevel::DangerFullAccess,
            "MCP session creation cannot disable the sandbox"
        );
        let config = ChatConfig {
            harness,
            model: args.model,
            reasoning: args.reasoning,
            model_options: Default::default(),
            sandbox,
            // This legacy tool explicitly promises constrained creation.
            // It cannot inherit the unrestricted desktop session default.
            runtime_mode: zeron_proto::RuntimeMode::ApprovalRequired,
            interaction_mode: zeron_proto::InteractionMode::Default,
        };
        self.require_runtime_policy(&device, &config).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let cwd = space
            .as_ref()
            .map(|space| space.path.clone())
            .unwrap_or_else(|| "~".into());
        let pending;
        {
            let _dispatch = self.dispatch.lock().await;
            ensure!(
                self.pending.lock().await.len() < 256,
                "too many pending sends; collect their turns before creating more"
            );
            self.zeron.call(methods::MUTATE,json!({"op":"createChat","chatId":id,"spaceId":space.as_ref().map(|space| &space.id),"deviceId":device,"config":config})).await?;
            if let Some(title) = args.title {
                self.zeron
                    .call(
                        methods::MUTATE,
                        json!({"op":"renameChat","chatId":id,"title":title}),
                    )
                    .await?;
            }
            // Construct the row from validated inputs; registry propagation may lag.
            pending = if let Some(prompt) = args.prompt.filter(|prompt| !prompt.trim().is_empty()) {
                let request = run_request(&config, &cwd, prompt);
                self.zeron.call(methods::QUEUE_COMMAND,json!({"chatId":id,"targetDeviceId":device,"command":SessionCommandPayload::Run {request,message_id:uuid::Uuid::new_v4().to_string()}})).await?;
                let turn = Arc::new(PendingTurn {
                    message_ids: vec![],
                });
                self.pending.lock().await.insert(id.clone(), turn.clone());
                Some(turn)
            } else {
                None
            };
        }
        let mut result = json!({"chatId":id,"kind":"chat","deviceId":device,"project":space,"parentChatId":null,"harness":harness,"model":config.model});
        if args.wait && pending.is_some() {
            // Only transcript/status identity is used by the wait.
            let chat: Chat = serde_json::from_value(
                json!({"id":id,"deviceId":device,"archived":false,"createdAt":"1970-01-01T00:00:00Z","cwd":cwd,"config":config}),
            )?;
            result["turn"] = self
                .wait(&chat, pending, timeout(args.timeout_secs))
                .await?["turn"]
                .clone();
        }
        Ok(result)
    }

    async fn send(&self, args: SendArgs) -> anyhow::Result<Value> {
        ensure!(!args.text.trim().is_empty(), "text is required");
        let chat = self.zeron.chat(&args.chat).await?;
        ensure!(
            self.zeron.origin.chat_id.as_deref() != Some(&chat.id),
            "a chat cannot send to itself"
        );
        let pending;
        {
            let _dispatch = self.dispatch.lock().await;
            ensure!(
                self.pending.lock().await.len() < 256,
                "too many pending sends; collect their turns before sending more"
            );
            let sessions = self.zeron.sessions().await?;
            ensure!(
                !sessions.iter().any(|session| session.chat_id == chat.id
                    && session.device_id == chat.device_id
                    && matches!(
                        session.status,
                        SessionStatus::Working | SessionStatus::AwaitingInput
                    )),
                "session is working or awaiting input; wait, interrupt or respond_to_input first"
            );
            let config = chat
                .config
                .as_ref()
                .context("session has no harness configuration")?;
            let harnesses = self.zeron.harnesses(&chat.device_id).await?;
            ensure!(
                harnesses
                    .iter()
                    .any(|info| info.id == config.harness && info.available()),
                "session harness is unavailable on its host"
            );
            ensure!(
                config.sandbox != SandboxLevel::DangerFullAccess,
                "MCP cannot start an unsandboxed session"
            );
            let entries = self.zeron.transcript(&chat).await?;
            pending = Arc::new(PendingTurn {
                message_ids: entries.into_iter().map(|entry| entry.id).collect(),
            });
            let mut text = args.text;
            if let Some(origin) = &self.zeron.origin.chat_id {
                text = format!("[Message from Noches chat {origin}]\n\n{text}");
            }
            self.queue(
                &chat,
                SessionCommandPayload::Run {
                    request: run_request(config, chat.cwd.as_deref().unwrap_or("~"), text),
                    message_id: uuid::Uuid::new_v4().to_string(),
                },
            )
            .await?;
            self.pending
                .lock()
                .await
                .insert(chat.id.clone(), pending.clone());
        }
        let mut result = json!({"chatId":chat.id,"sent":true});
        if args.wait {
            result["turn"] = self
                .wait(&chat, Some(pending), timeout(args.timeout_secs))
                .await?["turn"]
                .clone();
        }
        Ok(result)
    }

    async fn queue(&self, chat: &Chat, command: SessionCommandPayload) -> anyhow::Result<Value> {
        self.zeron.device(Some(&chat.device_id)).await?;
        self.zeron.require_routing(&chat.device_id).await?;
        if !matches!(command, SessionCommandPayload::Interrupt { .. })
            && let Some(config) = &chat.config
        {
            self.require_runtime_policy(&chat.device_id, config).await?;
        }
        self.zeron
            .call(
                methods::QUEUE_COMMAND,
                json!({"chatId":chat.id,"targetDeviceId":chat.device_id,"command":command}),
            )
            .await
    }

    async fn require_runtime_policy(
        &self,
        device: &str,
        config: &ChatConfig,
    ) -> anyhow::Result<()> {
        if config.runtime_mode == zeron_proto::RuntimeMode::FullAccess
            && config.interaction_mode == zeron_proto::InteractionMode::Default
        {
            return Ok(());
        }
        let capability = zeron_proto::capabilities::RUNTIME_POLICY_V1;
        let info = self.zeron.call(methods::ENGINE_INFO, json!({})).await?;
        ensure!(
            info["capabilities"]
                .as_array()
                .is_some_and(|caps| caps.iter().any(|c| c == capability)),
            "Update the local engine to enforce this runtime mode"
        );
        if device != self.zeron.local_device().await? {
            ensure!(
                self.zeron
                    .devices()
                    .await?
                    .iter()
                    .any(|d| d.id == device && d.supports(capability)),
                "Update the execution engine to enforce this runtime mode"
            );
        }
        Ok(())
    }

    async fn wait(
        &self,
        chat: &Chat,
        pending: Option<Arc<PendingTurn>>,
        timeout: Duration,
    ) -> anyhow::Result<Value> {
        let deadline = tokio::time::Instant::now() + timeout;
        let result = tokio::time::timeout_at(deadline, async {
            loop {
                let sessions = self.zeron.sessions().await?;
                let session = sessions.iter().find(|session| {
                    session.chat_id == chat.id && session.device_id == chat.device_id
                });
                let entries = self.zeron.transcript(chat).await?;
                let replies: Vec<_> = entries
                    .into_iter()
                    .filter(|entry| entry.role == MessageRole::Assistant)
                    .filter(|entry| {
                        pending
                            .as_ref()
                            .is_none_or(|pending| !pending.message_ids.contains(&entry.id))
                    })
                    .collect();
                let outcome = match session.map(|session| session.status) {
                    Some(SessionStatus::AwaitingInput) => Some("awaitingInput"),
                    Some(SessionStatus::Errored) if pending.is_none() || !replies.is_empty() => {
                        Some("errored")
                    }
                    Some(SessionStatus::Idle)
                        if pending.is_none()
                            || replies
                                .iter()
                                .any(|reply| reply.status == Some(MessageStatus::Complete)) =>
                    {
                        Some("completed")
                    }
                    None if pending.is_none() => Some("completed"),
                    _ => None,
                };
                if let Some(outcome) = outcome {
                    return Ok::<_, anyhow::Error>(
                        json!({"outcome":outcome,"timedOut":false,"replies":replies}),
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;
        let turn = match result {
            Ok(result) => result?,
            Err(_) => json!({"outcome":"timedOut","timedOut":true,"replies":[]}),
        };
        if turn["timedOut"] == false
            && let Some(pending) = pending
        {
            let mut turns = self.pending.lock().await;
            if turns
                .get(&chat.id)
                .is_some_and(|current| Arc::ptr_eq(current, &pending))
            {
                turns.remove(&chat.id);
            }
        }
        Ok(json!({"chatId":chat.id,"turn":turn}))
    }
}

fn run_request(config: &ChatConfig, cwd: &str, prompt: String) -> RunRequest {
    RunRequest {
        prompt,
        harness: Some(config.harness),
        model: config.model.clone(),
        reasoning: config.reasoning,
        model_options: config.model_options.clone(),
        cwd: cwd.into(),
        sandbox: config.sandbox,
        runtime_mode: config.runtime_mode,
        interaction_mode: config.interaction_mode,
        auto_approve: false,
        resume: None,
        attachments: vec![],
        worktree: None,
    }
}
