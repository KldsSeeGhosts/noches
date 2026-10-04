//! Host-owned, read-only discovery and separately authorized history import.
//! The page requests previews only for list rows being laid out; four global
//! client lanes cap preview concurrency across projects.
use std::collections::{HashMap, HashSet, VecDeque};

use gpui::Context;
use serde_json::json;
use zeron_proto::git_actions::*;
use zeron_rpc::git_actions::methods;

use crate::{
    git_store::{call, routed},
    state::AppState,
};

#[derive(Clone, Default)]
pub struct ProjectHistory {
    pub scan: Option<HistoryScanState>,
    pub import: Option<HistoryImportState>,
    pub previews: HashMap<String, HistoryPreview>,
    pub preview_errors: HashMap<String, String>,
    pub busy: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct HistoryImportStore {
    pub projects: HashMap<String, ProjectHistory>,
    queue: VecDeque<(String, String)>,
    queued: HashSet<(String, String)>,
    active: usize,
}

impl AppState {
    pub fn scan_history(&mut self, space_id: &str, resume: bool, cx: &mut Context<Self>) {
        let Some(space) = self.spaces.iter().find(|s| s.id == space_id).cloned() else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let project = self
            .history_import
            .projects
            .entry(space.id.clone())
            .or_default();
        if project.busy
            || project.scan.as_ref().is_some_and(|s| s.status == "running")
            || project
                .import
                .as_ref()
                .is_some_and(|s| s.status == "running")
        {
            return;
        }
        let scan_id = if resume {
            project.scan.as_ref().map(|s| s.scan_id.clone())
        } else {
            None
        };
        if !resume {
            project.scan = None;
            project.previews.clear();
            project.preview_errors.clear();
        }
        project.busy = true;
        project.error = None;
        cx.spawn(async move |this, cx| {
            let request = HistoryScanRequest {
                space_id: space.id.clone(),
                scan_id,
            };
            let result: Result<HistoryScanState, _> = call(
                engine.client(),
                &space.device_id,
                methods::SCAN_HISTORY,
                &request,
            )
            .await;
            let scan = match result {
                Ok(scan) => scan,
                Err(error) => {
                    this.update(cx, |state, cx| {
                        let project = state
                            .history_import
                            .projects
                            .entry(space.id.clone())
                            .or_default();
                        project.busy = false;
                        project.error = Some(error.to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let id = scan.scan_id.clone();
            let running = scan.status == "running";
            if this
                .update(cx, |state, cx| {
                    let project = state
                        .history_import
                        .projects
                        .entry(space.id.clone())
                        .or_default();
                    project.busy = false;
                    project.error = scan.error.clone();
                    project.scan = Some(scan);
                    cx.notify();
                })
                .is_err()
                || !running
            {
                return;
            }
            loop {
                if let Ok(mut stream) = engine
                    .client()
                    .subscribe_checked(
                        methods::WATCH_SCAN,
                        routed(&space.device_id, json!({"scanId": id})),
                    )
                    .await
                {
                    while let Some(value) = stream.recv().await {
                        if let Ok(scan) = serde_json::from_value::<HistoryScanState>(value) {
                            let done = scan.status != "running";
                            if this
                                .update(cx, |state, cx| {
                                    let project = state
                                        .history_import
                                        .projects
                                        .entry(space.id.clone())
                                        .or_default();
                                    project.error = scan.error.clone();
                                    project.scan = Some(scan);
                                    cx.notify();
                                })
                                .is_err()
                                || done
                            {
                                return;
                            }
                        }
                    }
                }
                let result: Result<HistoryScanState, _> = call(
                    engine.client(),
                    &space.device_id,
                    methods::GET_SCAN,
                    json!({"scanId": id}),
                )
                .await;
                let done = result.as_ref().is_ok_and(|s| s.status != "running");
                if this
                    .update(cx, |state, cx| {
                        let project = state
                            .history_import
                            .projects
                            .entry(space.id.clone())
                            .or_default();
                        match result {
                            Ok(scan) => {
                                project.error = scan.error.clone();
                                project.scan = Some(scan);
                            }
                            Err(error) => project.error = Some(error.to_string()),
                        }
                        cx.notify();
                    })
                    .is_err()
                    || done
                {
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(3))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    pub fn ensure_history_preview(
        &mut self,
        space_id: &str,
        candidate_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.history_import.projects.get(space_id) else {
            return;
        };
        if project.previews.contains_key(candidate_id)
            || project.preview_errors.contains_key(candidate_id)
        {
            return;
        }
        let key = (space_id.to_string(), candidate_id.to_string());
        if self.history_import.queued.insert(key.clone()) {
            self.history_import.queue.push_back(key);
        }
        self.pump_history_previews(cx);
    }

    fn pump_history_previews(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        while self.history_import.active < 4 {
            let Some((space_id, candidate_id)) = self.history_import.queue.pop_front() else {
                break;
            };
            let Some(space) = self.spaces.iter().find(|s| s.id == space_id) else {
                continue;
            };
            let owner = space.device_id.clone();
            let engine = engine.clone();
            self.history_import.active += 1;
            cx.spawn(async move |this, cx| {
                let result: Result<HistoryPreview, _> = call(
                    engine.client(),
                    &owner,
                    methods::PREVIEW_HISTORY,
                    json!({"candidateId": candidate_id}),
                )
                .await;
                this.update(cx, |state, cx| {
                    state.history_import.active -= 1;
                    state
                        .history_import
                        .queued
                        .remove(&(space_id.clone(), candidate_id.clone()));
                    let project = state.history_import.projects.entry(space_id).or_default();
                    // A replacement scan must not publish the previous scan's previews.
                    if project
                        .scan
                        .as_ref()
                        .is_some_and(|s| s.candidate_ids.contains(&candidate_id))
                    {
                        match result {
                            Ok(preview) => {
                                project.previews.insert(candidate_id, preview);
                            }
                            Err(error) => {
                                project
                                    .preview_errors
                                    .insert(candidate_id, error.to_string());
                            }
                        }
                    }
                    state.pump_history_previews(cx);
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
    }

    pub fn import_history(
        &mut self,
        space_id: &str,
        candidate_ids: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if candidate_ids.is_empty() || candidate_ids.len() > 100 {
            return;
        }
        let Some(space) = self.spaces.iter().find(|s| s.id == space_id).cloned() else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let project = self
            .history_import
            .projects
            .entry(space.id.clone())
            .or_default();
        if project.busy
            || project
                .import
                .as_ref()
                .is_some_and(|i| i.status == "running")
        {
            return;
        }
        project.busy = true;
        project.error = None;
        cx.spawn(async move |this, cx| {
            let request = HistoryImportRequest {
                space_id: space.id.clone(),
                candidate_ids,
            };
            let result: Result<HistoryImportState, _> = call(
                engine.client(),
                &space.device_id,
                methods::IMPORT_HISTORY,
                &request,
            )
            .await;
            let import = match result {
                Ok(import) => import,
                Err(error) => {
                    this.update(cx, |state, cx| {
                        let project = state
                            .history_import
                            .projects
                            .entry(space.id.clone())
                            .or_default();
                        project.busy = false;
                        project.error = Some(error.to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let id = import.import_id.clone();
            let running = import.status == "running";
            if this
                .update(cx, |state, cx| {
                    state.receive_history_import(&space.id, import, cx)
                })
                .is_err()
                || !running
            {
                return;
            }
            loop {
                if let Ok(mut stream) = engine
                    .client()
                    .subscribe_checked(
                        methods::WATCH_IMPORT,
                        routed(&space.device_id, json!({"importId": id})),
                    )
                    .await
                {
                    while let Some(value) = stream.recv().await {
                        if let Ok(import) = serde_json::from_value::<HistoryImportState>(value) {
                            let done = import.status != "running";
                            if this
                                .update(cx, |state, cx| {
                                    state.receive_history_import(&space.id, import, cx)
                                })
                                .is_err()
                                || done
                            {
                                return;
                            }
                        }
                    }
                }
                let result: Result<HistoryImportState, _> = call(
                    engine.client(),
                    &space.device_id,
                    methods::GET_IMPORT,
                    json!({"importId": id}),
                )
                .await;
                let done = result.as_ref().is_ok_and(|i| i.status != "running");
                if this
                    .update(cx, |state, cx| match result {
                        Ok(import) => state.receive_history_import(&space.id, import, cx),
                        Err(error) => {
                            state
                                .history_import
                                .projects
                                .entry(space.id.clone())
                                .or_default()
                                .error = Some(error.to_string());
                            cx.notify();
                        }
                    })
                    .is_err()
                    || done
                {
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(3))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    fn receive_history_import(
        &mut self,
        space_id: &str,
        import: HistoryImportState,
        cx: &mut Context<Self>,
    ) {
        let project = self
            .history_import
            .projects
            .entry(space_id.into())
            .or_default();
        project.busy = false;
        for (candidate, chat) in import.completed_candidate_ids.iter().zip(&import.chat_ids) {
            if let Some(preview) = project.previews.get_mut(candidate) {
                preview.already_imported_chat_id = Some(chat.clone());
            }
        }
        project.error = import.error.clone();
        project.import = Some(import);
        cx.notify();
    }

    pub fn cancel_history(&mut self, space_id: &str, operation_id: String, cx: &mut Context<Self>) {
        let Some(space) = self.spaces.iter().find(|s| s.id == space_id).cloned() else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let result: Result<(), _> = call(
                engine.client(),
                &space.device_id,
                methods::CANCEL_HISTORY,
                json!({"operationId":operation_id}),
            )
            .await;
            if let Err(error) = result {
                this.update(cx, |state, cx| {
                    state
                        .history_import
                        .projects
                        .entry(space.id)
                        .or_default()
                        .error = Some(error.to_string());
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}
