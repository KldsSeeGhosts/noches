use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use zeron_doc::{SessionMessageEntry, TranscriptFrame, apply_transcript_frame};
use zeron_proto::{Chat, Device, HarnessId, Model, Session, Space};
use zeron_rpc::{RpcClient, methods};

#[derive(Default)]
pub struct Origin {
    pub chat_id: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct HarnessInfo {
    pub id: HarnessId,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl HarnessInfo {
    pub fn available(&self) -> bool {
        self.installed && self.enabled.unwrap_or(true) && self.id != HarnessId::Mock
    }
}

pub struct Zeron {
    client: Arc<RpcClient>,
    pub(crate) origin: Origin,
}

impl Zeron {
    pub fn with_client(client: RpcClient, origin: Origin) -> Self {
        Self {
            client: Arc::new(client),
            origin,
        }
    }

    // Do not retry writes after an ambiguous transport failure.
    pub(crate) async fn call(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.client
            .call(method, params)
            .await
            .with_context(|| method.to_string())
    }

    async fn snapshot<T: serde::de::DeserializeOwned>(&self, method: &str) -> anyhow::Result<T> {
        let mut stream = self.client.subscribe_scoped(method, json!({})).await?;
        let value = tokio::time::timeout(Duration::from_secs(15), stream.recv())
            .await?
            .context("snapshot stream closed")?;
        serde_json::from_value(value).with_context(|| method.to_string())
    }

    pub(crate) async fn devices(&self) -> anyhow::Result<Vec<Device>> {
        self.snapshot(methods::WATCH_DEVICES).await
    }
    pub(crate) async fn spaces(&self) -> anyhow::Result<Vec<Space>> {
        self.snapshot(methods::WATCH_SPACES).await
    }
    pub(crate) async fn chats(&self) -> anyhow::Result<Vec<Chat>> {
        self.snapshot(methods::WATCH_CHATS).await
    }
    pub(crate) async fn sessions(&self) -> anyhow::Result<Vec<Session>> {
        self.snapshot(methods::WATCH_SESSIONS).await
    }
    pub(crate) async fn local_device(&self) -> anyhow::Result<String> {
        let value = self.call(methods::LOCAL_DEVICE, json!({})).await?;
        Ok(value["deviceId"]
            .as_str()
            .context("missing local device id")?
            .into())
    }
    pub(crate) async fn device(&self, key: Option<&str>) -> anyhow::Result<String> {
        let Some(key) = key.map(str::trim).filter(|key| !key.is_empty()) else {
            return self.local_device().await;
        };
        let devices = self.devices().await?;
        if let Some(device) = devices.iter().find(|device| device.id == key) {
            return Ok(device.id.clone());
        }
        let matches: Vec<_> = devices.iter().filter(|device| device.name == key).collect();
        ensure!(
            matches.len() == 1,
            "device {key:?} is missing or ambiguous; use a listed id"
        );
        Ok(matches[0].id.clone())
    }

    pub(crate) async fn target(
        &self,
        project: Option<&str>,
        device: Option<&str>,
    ) -> anyhow::Result<(Option<Space>, String)> {
        let device = match device.map(str::trim).filter(|key| !key.is_empty()) {
            Some(key) => Some(self.device(Some(key)).await?),
            None => None,
        };
        if let Some(project) = project {
            let mut spaces = self.spaces().await?;
            if let Some(device) = &device {
                if let Some(space) = spaces.iter().find(|space| space.id == project.trim()) {
                    ensure!(
                        space.device_id == *device,
                        "project {} belongs to device {}, not {device}",
                        space.id,
                        space.device_id
                    );
                }
                spaces.retain(|space| space.device_id == *device);
            }
            let space = resolve_space(&spaces, project)?;
            // Registry membership alone does not imply device ownership.
            self.device(Some(&space.device_id)).await?;
            return Ok((Some(space.clone()), space.device_id));
        }
        Ok((
            None,
            match device {
                Some(device) => device,
                None => self.local_device().await?,
            },
        ))
    }

    pub(crate) async fn require_routing(&self, device: &str) -> anyhow::Result<()> {
        if device != self.local_device().await? {
            let info = self.call(methods::ENGINE_INFO, json!({})).await?;
            ensure!(
                info["capabilities"]
                    .as_array()
                    .is_some_and(|caps| caps.iter().any(|cap| cap == "mcp-session-routing-v1")),
                "this engine does not advertise safe MCP device routing; update it before selecting device {device}"
            );
        }
        Ok(())
    }

    pub(crate) async fn harnesses(&self, device: &str) -> anyhow::Result<Vec<HarnessInfo>> {
        self.require_routing(device).await?;
        let rows = self
            .call(methods::LIST_HARNESSES, json!({"targetDeviceId":device}))
            .await
            .with_context(|| format!("harness catalog on device {device}"))?;
        Ok(serde_json::from_value(rows)?)
    }
    pub(crate) async fn models(
        &self,
        harness: HarnessId,
        device: &str,
    ) -> anyhow::Result<Vec<Model>> {
        self.require_routing(device).await?;
        let rows = self
            .call(
                methods::LIST_MODELS,
                json!({"harness":harness,"targetDeviceId":device}),
            )
            .await
            .with_context(|| format!("model catalog on device {device}"))?;
        Ok(serde_json::from_value(rows)?)
    }
    pub(crate) async fn chat(&self, key: &str) -> anyhow::Result<Chat> {
        let chats = self.chats().await?;
        if let Some(chat) = chats.iter().find(|chat| chat.id == key) {
            return Ok(chat.clone());
        }
        let matches: Vec<_> = chats
            .iter()
            .filter(|chat| chat.id.starts_with(key) || chat.title.as_deref() == Some(key))
            .collect();
        ensure!(
            !key.is_empty() && matches.len() == 1,
            "chat {key:?} is missing or ambiguous; use a listed id"
        );
        Ok(matches[0].clone())
    }
    pub(crate) async fn transcript(&self, chat: &Chat) -> anyhow::Result<Vec<SessionMessageEntry>> {
        self.require_routing(&chat.device_id).await?;
        let mut stream = self
            .client
            .subscribe_scoped(
                methods::WATCH_DOC_MESSAGES,
                json!({"chatId":chat.id,"targetDeviceId":chat.device_id}),
            )
            .await?;
        let value = tokio::time::timeout(Duration::from_secs(15), stream.recv())
            .await?
            .context("transcript stream closed")?;
        let frame: TranscriptFrame = serde_json::from_value(value)?;
        let mut entries = vec![];
        apply_transcript_frame(&mut entries, frame)?;
        Ok(entries)
    }
}

pub(crate) fn resolve_space(spaces: &[Space], key: &str) -> anyhow::Result<Space> {
    let key = key.trim().trim_end_matches(['/', '\\']);
    ensure!(!key.is_empty(), "project is required");
    if let Some(space) = spaces.iter().find(|space| space.id == key) {
        return Ok(space.clone());
    }
    let paths: Vec<_> = spaces
        .iter()
        .filter(|space| space.path.trim_end_matches(['/', '\\']) == key)
        .collect();
    let names: Vec<_> = spaces
        .iter()
        .filter(|space| space.display_name().eq_ignore_ascii_case(key))
        .collect();
    let suffixes: Vec<_> = spaces
        .iter()
        .filter(|space| space.path.trim_end_matches(['/', '\\']).ends_with(key))
        .collect();
    let matches = if !paths.is_empty() {
        paths
    } else if !names.is_empty() {
        names
    } else {
        suffixes
    };
    if matches.len() == 1 {
        return Ok(matches[0].clone());
    }
    bail!(
        "project {key:?} is missing or ambiguous; use a listed id: {}",
        matches
            .iter()
            .map(|space| format!("{} on {}", space.id, space.device_id))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
