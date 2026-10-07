//! Registry identity admission. Import is conversation provenance, not execution.
use std::sync::Arc;

use serde_json::{Value, json};
use zeron_doc::{MessagePart, MessageRole};
use zeron_proto::orchestration::{OrchestrationV2AppThread, ProjectId, ThreadId};

use super::{Error, Result, event::iso};
use crate::{DocHost, HarnessRegistry, WorkspaceHost};

pub(crate) struct RegistryAdmission {
    pub workspace: WorkspaceHost,
    pub docs: DocHost,
    pub registry: Arc<HarnessRegistry>,
}

impl RegistryAdmission {
    pub fn unavailable(&self, id: &ThreadId, project: Option<&ProjectId>) -> bool {
        self.workspace
            .chat(&id.0)
            .ok()
            .flatten()
            .is_some_and(|chat| self.chat_unavailable(&chat, project))
    }

    fn chat_unavailable(&self, chat: &zeron_proto::Chat, project: Option<&ProjectId>) -> bool {
        chat.device_id != self.workspace.device_id()
            || project.is_some_and(|p| chat.space_id.as_ref().is_some_and(|s| *s != p.0))
            || chat.space_id.as_ref().is_some_and(|s| {
                self.workspace
                    .space(s)
                    .ok()
                    .flatten()
                    .is_none_or(|space| space.device_id != self.workspace.device_id())
            })
    }

    pub fn prepare(
        &self,
        id: &ThreadId,
        project: Option<&ProjectId>,
    ) -> Result<Option<(OrchestrationV2AppThread, Vec<Value>)>> {
        let Some(chat) = self.workspace.chat(&id.0).map_err(invariant)? else {
            return Ok(None);
        };
        if self.chat_unavailable(&chat, project) {
            return Ok(None);
        }
        let space = chat
            .space_id
            .as_ref()
            .map(|id| self.workspace.space(id).map_err(invariant))
            .transpose()?
            .flatten();
        let config = chat.config.as_ref();
        let harness = config
            .map(|c| c.harness)
            .unwrap_or_else(|| self.docs.harness_for(&id.0));
        // Never substitute another configured instance of the same vendor.
        let instance = config
            .and_then(|c| c.instance_id.clone())
            .unwrap_or_else(|| crate::provider_instances::legacy_instance_id(harness));
        let provider = self
            .registry
            .provider_instances
            .snapshot(&self.registry)
            .into_iter()
            .find(|p| p.provider_instance_id == instance);
        let model = config
            .and_then(|c| c.model.clone())
            .or_else(|| {
                provider
                    .as_ref()
                    .and_then(|p| p.models.first().map(|m| m.id.clone()))
            })
            // Same implicit provider default as ordinary Sessions admission.
            .unwrap_or_else(|| "default".into());
        let mut options = config.map(|c| c.model_options.clone()).unwrap_or_default();
        if let Some(model) = provider
            .as_ref()
            .and_then(|p| p.models.iter().find(|m| m.id == model))
        {
            for descriptor in model.options.as_ref().into_iter().flatten() {
                if let zeron_proto::provider_instance::ProviderOptionDescriptor::Boolean(option) =
                    descriptor
                    && let Some(value) = options.get_mut(&option.id)
                    && let Some(text) = value.as_str()
                {
                    *value = Value::Bool(matches!(text, "on" | "true"));
                }
            }
        }
        if let Some(reasoning) = config.and_then(|c| c.reasoning) {
            let key = crate::provider_instances::reasoning_option_key(harness);
            options.entry(key).or_insert(json!(reasoning));
        }
        let selection = zeron_proto::orchestration::normalize_contract(
            "ModelSelection",
            json!({"instanceId":instance,"model":model,"options":options}),
        )
        .map_err(invariant)?;
        let created = chat
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        // Keep an explicit cwd even at the project root: the runner need not
        // depend on launch's separate project projection being populated.
        let cwd = chat
            .cwd
            .clone()
            .or_else(|| chat.source_context.as_ref().map(|s| s.cwd.clone()))
            .or_else(|| space.as_ref().map(|s| s.path.clone()));
        let thread = serde_json::from_value(json!({
            "id":id,"projectId":project_id(&chat),
            "title":chat.title.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or("Conversation"),
            "createdBy":"user","creationSource":"web","providerInstanceId":instance,
            "modelSelection":selection,"runtimeMode":config.map(|c| c.runtime_mode).unwrap_or_default(),
            "interactionMode":config.map(|c| c.interaction_mode).unwrap_or_default(),
            "branch":chat.branch,"worktreePath":cwd,"activeProviderThreadId":null,
            "historyOrigin":"v1_import",
            "lineage":{"parentThreadId":null,"relationshipToParent":null,"rootThreadId":id},
            "forkedFrom":null,"createdAt":created,"updatedAt":created,
            "archivedAt":chat.archived.then_some(&created),"deletedAt":null
        }))?;
        let entries = self.docs.orchestration_history(&id.0).map_err(invariant)?;
        let mut messages = vec![];
        for entry in entries {
            if !matches!(entry.role, MessageRole::User | MessageRole::Assistant) {
                continue;
            }
            let text = entry
                .parts
                .iter()
                .filter_map(|p| match p {
                    MessagePart::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            let at = iso(entry.created_at)?;
            messages.push(json!({
                "id":entry.id,"threadId":id,"runId":null,"nodeId":null,"role":entry.role,
                "text":text,"streaming":false,"attachments":[],
                "createdBy":if entry.role == MessageRole::User {"user"} else {"agent"},
                "creationSource":"server","createdAt":at,"updatedAt":at
            }));
        }
        Ok(Some((thread, messages)))
    }
}

fn invariant(error: impl std::fmt::Display) -> Error {
    Error::Invariant(error.to_string())
}

pub(crate) fn project_id(chat: &zeron_proto::Chat) -> &str {
    chat.space_id
        .as_deref()
        .or(chat.cwd.as_deref())
        .or_else(|| chat.source_context.as_ref().map(|s| s.cwd.as_str()))
        .unwrap_or("scratch")
}
