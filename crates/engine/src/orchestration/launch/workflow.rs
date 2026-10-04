use super::{
    HostLaunchService, LaunchOperation, ToolError, id, invalid, lock, unavailable, worktree_failure,
};
use crate::mcp::auth::InvocationScope;
use crate::orchestration::{Command, Operation, ReceiptStatus};
use serde_json::{Value, json};
use std::path::Path;
use zeron_proto::orchestration::{CommandId, OrchestrationV2AppThread, ThreadId};
use zeron_proto::provider_instance::ModelSelection;

impl HostLaunchService {
    pub(crate) async fn operation(
        &self,
        thread: &str,
        command_id: &str,
        operation: LaunchOperation,
    ) -> Result<(), ToolError> {
        let receipt = self
            .kernel
            .dispatch(
                &Command {
                    id: CommandId(command_id.into()),
                    thread_id: ThreadId(thread.into()),
                    operation: Operation::Launch(Box::new(operation)),
                },
                crate::now_ms(),
            )
            .await
            .map_err(|_| unavailable())?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(invalid(receipt.error.unwrap_or_default()));
        }
        Ok(())
    }

    pub(crate) fn workflow(&self, thread: &str) -> Result<Value, ToolError> {
        self.rows("orchestration_launch_workflows")
            .map_err(|_| unavailable())?
            .into_iter()
            .find(|w| w["threadId"] == thread)
            .ok_or_else(unavailable)
    }

    pub(crate) async fn launch(
        &self,
        scope: &InvocationScope,
        mut input: Value,
    ) -> Result<Value, ToolError> {
        let caller = self.require_full(
            scope,
            "Project launches require a full-access/default calling thread.",
        )?;
        if input.get("projectId").is_none() && input["scratch"] != true {
            input["projectId"] = json!(caller.project_id);
        }
        if input.get("modelSelection").is_none() {
            input["modelSelection"] = json!(caller.model_selection);
        }
        if input.get("runtimeMode").is_none() {
            input["runtimeMode"] = json!(caller.runtime_mode);
        }
        if input.get("interactionMode").is_none() {
            input["interactionMode"] = json!(caller.interaction_mode);
        }
        // Provenance comes from authenticated authority, never tool extras.
        input["createdBy"] = json!("agent");
        input["creationSource"] = json!("mcp");
        input["senderThreadId"] = json!(scope.caller.thread_id);
        let command_id = format!(
            "command:mcp:{}:{}",
            crate::orchestration::event::encode_component(&scope.caller.session_id),
            id()
        );
        self.launch_host(input, command_id).await
    }

    async fn launch_host(&self, input: Value, command_id: String) -> Result<Value, ToolError> {
        let tid = command_id.clone();
        // A scheduler claim can replay after acceptance was committed but its
        // response was lost. The ordinary receipt identity owns the launch.
        if let Some(p) = self
            .kernel
            .store
            .thread(&ThreadId(tid.clone()))
            .map_err(|_| unavailable())?
        {
            let w = self.workflow(&tid)?;
            if w["commandId"] != command_id
                || p.thread.project_id.0 != input["projectId"].as_str().unwrap_or("")
            {
                return Err(unavailable());
            }
            self.spawn_preparation(tid.clone());
            return Ok(
                json!({"threadId":tid,"projectId":p.thread.project_id,"modelSelection":p.thread.model_selection,
                "runId":p.runs.first().map(|r|&r.id),"status":p.runs.first().map(|r|&r.status)}),
            );
        }
        let attachments = input["attachments"].as_array().cloned().unwrap_or_default();
        if attachments.iter().any(|a| !super::attachments::pending(a)) {
            return Err(invalid(
                "A new thread accepts only pending attachment uploads.",
            ));
        }
        if input["scratch"] == true
            && (input.get("projectId").is_some() || input.get("workspaceStrategy").is_some())
        {
            return Err(invalid(
                "scratch:true picks its own project and folder; omit projectId and workspaceStrategy.",
            ));
        }
        let project = if input["scratch"] == true {
            if self.repos.is_repo(&self.data_dir).await {
                return Err(ToolError::new(
                    super::Code::OrchestrationError,
                    "Threads without a project are not available on this environment.",
                ));
            }
            let scratch = self.data_dir.join("scratch");
            std::fs::create_dir_all(&scratch).map_err(|_| unavailable())?;
            let root = scratch.to_string_lossy();
            if let Some(project) = self.projects()?.into_iter().find(|p| {
                p["workspaceRoot"]
                    .as_str()
                    .is_some_and(|p| same_checkout(Path::new(p), &scratch))
                    && p["deletedAt"].is_null()
            }) {
                project
            } else {
                let mut project = self
                    .create_project(json!({"title":"No project","workspaceRoot":root}))
                    .await?;
                project["projectIcon"] =
                    json!({"kind":"lucide","name":"message-square-dashed","color":"gray"});
                self.persist_project(&project)?;
                project
            }
        } else {
            self.project(input["projectId"].as_str().ok_or_else(unavailable)?)?
        };
        let selection: ModelSelection =
            serde_json::from_value(input["modelSelection"].clone()).map_err(|_| unavailable())?;
        let target = json!({"providerInstanceId":selection.instance_id,"model":selection.model,"options":selection.options});
        self.registry
            .provider_instances
            .refresh_all(&self.registry)
            .await;
        let resolved = self.registry.provider_instances.resolve_target(
            &self.registry,
            &selection,
            Some(&target),
        )?;
        let driver = self
            .registry
            .provider_instances
            .snapshot(&self.registry)
            .into_iter()
            .find(|p| p.provider_instance_id == resolved.instance_id)
            .ok_or_else(unavailable)?
            .driver_kind
            .0;
        let strategy = input
            .get("workspaceStrategy")
            .cloned()
            .unwrap_or(json!({"type":"root"}));
        let root = project["workspaceRoot"].as_str().unwrap();
        let scratch_path = if same_checkout(Path::new(root), &self.data_dir.join("scratch")) {
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let words = super::projects::folder_words(input["message"].as_str().unwrap_or(""));
            let normalized_id: String = tid
                .to_lowercase()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect();
            let name = |suffix: &str| {
                [date.as_str(), words.as_str(), suffix]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("-")
            };
            let mut attempt = 1;
            let folder = loop {
                let name = match attempt {
                    1 => name(&normalized_id[normalized_id.len().saturating_sub(8)..]),
                    2 => name(&normalized_id),
                    _ => format!("{}-{}", name(&normalized_id), &id()[..8]),
                };
                let folder = self.data_dir.join("scratch").join(name);
                match std::fs::create_dir(&folder) {
                    Ok(()) => break folder,
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => attempt += 1,
                    Err(_) => {
                        return Err(ToolError::new(
                            super::Code::OrchestrationError,
                            "Failed to create the folder for threads without a project.",
                        ));
                    }
                }
            };
            Some(folder.to_string_lossy().into_owned())
        } else {
            None
        };
        let thread:OrchestrationV2AppThread=serde_json::from_value(json!({
            "id":tid,"projectId":project["id"],"title":input["title"],"createdBy":input["createdBy"],"creationSource":input["creationSource"],
            "providerInstanceId":resolved.instance_id,"modelSelection":resolved,
            "runtimeMode":input["runtimeMode"],
            "interactionMode":input["interactionMode"],
            "branch":null,"worktreePath":scratch_path,"activeProviderThreadId":null,
            "lineage":{"parentThreadId":null,"relationshipToParent":null,"rootThreadId":tid},"forkedFrom":null,
            "createdAt":crate::orchestration::event::iso(crate::now_ms()).map_err(|_|unavailable())?,
            "updatedAt":crate::orchestration::event::iso(crate::now_ms()).map_err(|_|unavailable())?,
            "archivedAt":null,"settledOverride":null,"settledAt":null,"snoozedUntil":null,"snoozedAt":null,"lastVisitedAt":null,"deletedAt":null
        })).map_err(|_|unavailable())?;
        let (claimed, paths) = self.claim(&tid, attachments)?;
        let mut w = json!({"threadId":tid,"projectId":project["id"],"commandId":command_id,"status":"preparing","stage":"fetch",
            "projectWorkspaceRoot":root,"strategy":strategy,"driver":driver,"branch":null,"worktreePath":scratch_path,"setup":null,"createdWorktree":false});
        if input.get("message").is_some() || !claimed.is_empty() {
            let mut message = json!({"text":input["message"].as_str().unwrap_or(""),"attachments":claimed,
                "createdBy":input["createdBy"],"creationSource":input["creationSource"]});
            for key in ["messageId", "scheduledTaskId", "senderThreadId"] {
                if let Some(value) = input.get(key) {
                    message[key] = value.clone();
                }
            }
            w["initialMessage"] = message;
        }
        let result = self
            .operation(
                &tid,
                &command_id,
                LaunchOperation::Create {
                    thread: Box::new(thread.clone()),
                    workflow: w,
                    driver,
                },
            )
            .await;
        if let Err(error) = result {
            // Lost response is not proof of nonacceptance.
            if self
                .kernel
                .store
                .thread(&ThreadId(tid.clone()))
                .ok()
                .flatten()
                .is_none()
            {
                for path in paths {
                    let _ = std::fs::remove_file(path);
                }
                if let Some(path) = thread
                    .worktree_path
                    .as_ref()
                    .filter(|path| Path::new(path).starts_with(self.data_dir.join("scratch")))
                {
                    let _ = std::fs::remove_dir(path);
                }
            }
            return Err(error);
        }
        self.workspace
            .create_chat(
                &tid,
                project["id"].as_str(),
                Some(self.workspace.device_id()),
                None,
                scratch_path,
            )
            .map_err(|_| unavailable())?;
        let p = self
            .kernel
            .store
            .thread(&ThreadId(tid.clone()))
            .map_err(|_| unavailable())?
            .unwrap();
        self.spawn_preparation(tid.clone());
        Ok(
            json!({"threadId":tid,"projectId":thread.project_id,"modelSelection":thread.model_selection,
            "runId":p.runs.first().map(|r|&r.id),"status":p.runs.first().map(|r|&r.status)}),
        )
    }

    pub(crate) fn spawn_preparation(&self, thread: String) {
        let service = self.clone();
        tokio::spawn(async move {
            let _guards = service
                .preparation_locks
                .acquire([ThreadId(thread.clone())])
                .await;
            if service.workflow(&thread).is_ok_and(|w| {
                matches!(w["status"].as_str(), Some("ready" | "failed" | "cancelled"))
            }) {
                return;
            }
            if let Err(error) = service.prepare(&thread).await {
                if let Ok(mut w) = service.workflow(&thread) {
                    if let (Some(root), Some(path), Some(owner)) = (
                        w["projectWorkspaceRoot"].as_str(),
                        w["worktreePath"].as_str(),
                        w["commandId"].as_str(),
                    ) {
                        let _ = service
                            .repos
                            .release_worktree_claim(Path::new(root), Path::new(path), owner)
                            .await;
                    }
                    if service
                        .kernel
                        .store
                        .thread(&ThreadId(thread.clone()))
                        .ok()
                        .flatten()
                        .is_none_or(|p| p.thread.deleted_at.is_some())
                    {
                        w["status"] = json!("cancelled");
                        let _ = service.save("orchestration_launch_workflows", &thread, &w);
                        return;
                    }
                    let _ = service
                        .operation(
                            &thread,
                            &format!("{}:fail", w["commandId"].as_str().unwrap()),
                            LaunchOperation::Fail {
                                detail: error.message,
                            },
                        )
                        .await;
                }
            }
        });
    }

    /// Boot recovery only resumes preparation; it never re-dispatches an
    /// accepted initial message or regenerates ids/resources.
    pub fn recover_preparations(&self) -> Result<(), ToolError> {
        for mut w in self
            .rows("orchestration_launch_workflows")
            .map_err(|_| unavailable())?
        {
            let tid = w["threadId"].as_str().unwrap().to_owned();
            if self
                .kernel
                .store
                .thread(&ThreadId(tid.clone()))
                .map_err(|_| unavailable())?
                .is_none_or(|p| p.thread.deleted_at.is_some())
            {
                w["status"] = json!("cancelled");
                self.save("orchestration_launch_workflows", &tid, &w)
                    .map_err(|_| unavailable())?;
                continue;
            }
            if matches!(w["setup"]["status"].as_str(), Some("pending" | "running")) {
                w["setup"]["status"] = json!("failed");
                w["setup"]["detail"] =
                    json!("The environment restarted during setup; retry or continue explicitly.");
                if w["setup"]["blocking"] == true {
                    w["status"] = json!("blocked");
                }
                self.save("orchestration_launch_workflows", &tid, &w)
                    .map_err(|_| unavailable())?;
            } else if w["status"] == "preparing" {
                self.spawn_preparation(tid);
            }
        }
        Ok(())
    }

    fn stage(&self, thread: &str, w: &mut Value, stage: &str) -> Result<(), ToolError> {
        if self
            .kernel
            .store
            .thread(&ThreadId(thread.into()))
            .map_err(|_| unavailable())?
            .is_none_or(|p| p.thread.deleted_at.is_some())
        {
            return Err(invalid("The thread was not found."));
        }
        w["stage"] = json!(stage);
        self.save("orchestration_launch_workflows", thread, w)
            .map_err(|_| unavailable())
    }

    async fn prepare(&self, thread: &str) -> Result<(), ToolError> {
        let mut w = self.workflow(thread)?;
        let project = self.project(w["projectId"].as_str().unwrap())?;
        let root = std::path::PathBuf::from(project["workspaceRoot"].as_str().unwrap());
        let strategy = w["strategy"].clone();
        let command_id = w["commandId"].as_str().unwrap().to_owned();
        let kind = strategy["type"].as_str().unwrap();
        let mut path = w["worktreePath"].as_str().map(str::to_owned);
        let mut branch = w["branch"].as_str().map(str::to_owned);
        if !w["bound"].as_bool().unwrap_or(false) {
            if kind == "worktree" {
                let requested = strategy["branch"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("t3code/{}", &ShaName::of(&command_id)[..8]));
                branch = Some(requested.clone());
                let destination = w["worktreePath"]
                    .as_str()
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| self.repos.orchestration_worktree_path(&root, &requested));
                path = Some(destination.to_string_lossy().into_owned());
                // Persist the exact intended resource before entering Git.
                if w["checkoutIntent"] != true {
                    if destination.symlink_metadata().is_ok() {
                        return Err(invalid("Worktree destination already exists."));
                    }
                    w["worktreePath"] = json!(path);
                    w["branch"] = json!(branch);
                    w["checkoutIntent"] = json!(true);
                    self.stage(thread, &mut w, "fetch")?;
                }
                let mut base = w["resolvedBaseRef"]
                    .as_str()
                    .unwrap_or(strategy["baseRef"].as_str().unwrap())
                    .to_owned();
                let origin = strategy["startFromOrigin"] == true
                    && self
                        .repos
                        .orchestration_git(&root, &["remote", "get-url", "origin"])
                        .await
                        .is_ok();
                if origin && w["fetchCompleted"] != true {
                    self.repos
                        .orchestration_git(&root, &["fetch", "origin", &base])
                        .await
                        .map_err(|_| unavailable())?;
                    let remote = if base.starts_with("origin/") {
                        base.clone()
                    } else {
                        format!("origin/{base}")
                    };
                    if let Ok(commit) = self
                        .repos
                        .orchestration_git(
                            &root,
                            &["rev-parse", "--verify", &format!("{remote}^{{commit}}")],
                        )
                        .await
                    {
                        base = commit.trim().into();
                    }
                }
                w["resolvedBaseRef"] = json!(base);
                w["fetchCompleted"] = json!(true);
                self.save("orchestration_launch_workflows", thread, &w)
                    .map_err(|_| unavailable())?;
                self.stage(thread, &mut w, "checkout")?;
                if destination.exists() {
                    // Crash after checkout: verify the journaled binding, never
                    // adopt an unrelated destination and never overwrite it.
                    let actual = self
                        .repos
                        .current_branch(&destination)
                        .await
                        .map_err(|_| unavailable())?;
                    if (w["createdWorktree"] != true
                        && !self
                            .repos
                            .worktree_claim_matches(&destination, &command_id)
                            .await)
                        || actual != requested
                        || !self
                            .repos
                            .workspace_checkout(&root, &destination)
                            .await
                            .is_some_and(|actual| same_checkout(&actual, &destination))
                    {
                        return Err(invalid(
                            "The prepared worktree no longer matches its launch.",
                        ));
                    }
                } else {
                    self.repos
                        .create_claimed_worktree(
                            &root,
                            &destination,
                            &requested,
                            &base,
                            Some(&command_id),
                        )
                        .await
                        .map_err(|_| unavailable())?;
                }
                w["createdWorktree"] = json!(true);
            } else if kind == "existing_worktree" {
                let requested = Path::new(strategy["worktreePath"].as_str().unwrap());
                if !requested.is_absolute() {
                    return Err(invalid("Worktree path must be absolute."));
                }
                let existing = self
                    .repos
                    .workspace_checkout(&root, requested)
                    .await
                    .ok_or_else(unavailable)?;
                if !same_checkout(&existing, requested) {
                    return Err(invalid(
                        "The existing worktree is not a checkout of the project repository.",
                    ));
                }
                let actual = self
                    .repos
                    .current_branch(&existing)
                    .await
                    .map_err(|_| unavailable())?;
                if let Some(expected) = strategy["branch"].as_str() {
                    if actual != expected {
                        return Err(invalid("The existing worktree branch does not match."));
                    }
                }
                path = Some(existing.to_string_lossy().into_owned());
                branch = if actual == "HEAD" { None } else { Some(actual) };
            } else {
                // Root binding remains null, with project root used as cwd.
                if let Some(requested) = strategy["branch"].as_str() {
                    self.repos
                        .switch_ref(&root, requested)
                        .await
                        .map_err(|_| unavailable())?;
                    branch = Some(requested.into());
                }
            }
            self.stage(thread, &mut w, "bind")?;
            let expected = self
                .kernel
                .store
                .thread(&ThreadId(thread.into()))
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?
                .thread
                .worktree_path;
            self.operation(
                thread,
                &format!("{command_id}:workspace"),
                LaunchOperation::Bind {
                    expected: if w["kind"] == "handoff" {
                        None
                    } else {
                        expected
                    },
                    path: path.clone(),
                    branch: branch.clone(),
                    continuation: w["continuationPrompt"].as_str().map(str::to_owned),
                    driver: w["driver"].as_str().unwrap().into(),
                    workflow: if w["kind"] == "handoff" {
                        w["worktreePath"] = json!(path);
                        w["branch"] = json!(branch);
                        w["bound"] = json!(true);
                        Some(w.clone())
                    } else {
                        None
                    },
                },
            )
            .await?;
            w["worktreePath"] = json!(path);
            w["branch"] = json!(branch);
            w["bound"] = json!(true);
            self.stage(thread, &mut w, "setup-script")?;
        }
        let cwd = Path::new(
            path.as_deref()
                .unwrap_or(project["workspaceRoot"].as_str().unwrap()),
        );
        if w["createdWorktree"] == true {
            self.repos
                .release_worktree_claim(&root, cwd, &command_id)
                .await
                .map_err(|_| unavailable())?;
        }
        if w["kind"] == "handoff" && w["detachCompleted"] != true {
            self.intake.detach(thread).await?;
            w["detachCompleted"] = json!(true);
            self.save("orchestration_launch_workflows", thread, &w)
                .map_err(|_| unavailable())?;
        }
        if w["runSetupScript"] != false && !self.setup(thread, &project, cwd, &mut w, true).await? {
            return Ok(());
        }
        // Read fresh setup completion (the observer updated its own durable row).
        self.operation(
            thread,
            &format!("{command_id}:release"),
            LaunchOperation::Release,
        )
        .await
    }

    pub(crate) fn worktree_status(&self, scope: &InvocationScope) -> Value {
        let tid = &scope.caller.thread_id;
        let p = match self.kernel.store.thread(tid) {
            Ok(Some(p)) if p.thread.deleted_at.is_none() => p,
            Ok(_) => {
                return worktree_failure(
                    "thread_not_found",
                    format!("Thread '{tid}' was not found."),
                );
            }
            Err(_) => {
                return worktree_failure(
                    "operation_failed",
                    format!("Unable to read thread {tid}: The operation could not be completed."),
                );
            }
        };
        let project = match self.project(&p.thread.project_id.0) {
            Ok(p) => p,
            Err(_) => {
                return worktree_failure(
                    "project_not_found",
                    format!(
                        "Project '{}' was not found for thread '{tid}'.",
                        p.thread.project_id
                    ),
                );
            }
        };
        json!({"attached":p.thread.worktree_path.is_some(),"worktreePath":p.thread.worktree_path,"branch":p.thread.branch,
            "projectWorkspaceRoot":project["workspaceRoot"],"defaultStartFromOrigin":self.preferences().ok().map(|p|p["newWorktreesStartFromOrigin"].clone()).unwrap_or(json!(false))})
    }

    pub(crate) async fn worktree_list(
        &self,
        scope: &InvocationScope,
        input: Value,
    ) -> Result<Value, ToolError> {
        let caller = self.caller(scope, false)?;
        let project = self.project(&caller.project_id.0)?;
        let root = Path::new(
            caller
                .worktree_path
                .as_deref()
                .unwrap_or(project["workspaceRoot"].as_str().unwrap()),
        );
        let is_repo = self.repos.is_repo(root).await;
        if !is_repo {
            return Ok(
                json!({"refs":[],"isRepo":false,"hasPrimaryRemote":false,"nextCursor":null,"totalCount":0}),
            );
        }
        let refs = self
            .repos
            .orchestration_refs(root, input["includeMatchingRemoteRefs"] == true)
            .await
            .map_err(|_| unavailable())?;
        let query = input["query"].as_str().unwrap_or("").to_lowercase();
        let ref_kind = input["refKind"].as_str().unwrap_or("all");
        let rows: Vec<Value> = refs
            .into_iter()
            .filter(|r| {
                r["name"].as_str().unwrap().to_lowercase().contains(&query)
                    && (ref_kind == "all"
                        || (ref_kind == "local" && r["isRemote"] == false)
                        || (ref_kind == "remote" && r["isRemote"] == true))
            })
            .collect();
        let total = rows.len();
        let start = input["cursor"].as_u64().unwrap_or(0) as usize;
        let end = start.saturating_add(input["limit"].as_u64().unwrap_or(100) as usize);
        let has_primary = self
            .repos
            .orchestration_git(root, &["remote", "get-url", "origin"])
            .await
            .is_ok();
        Ok(
            json!({"refs":rows.into_iter().skip(start).take(end-start).collect::<Vec<_>>(),"isRepo":true,"hasPrimaryRemote":has_primary,"nextCursor":(end<total).then_some(end),"totalCount":total}),
        )
    }

    pub(crate) async fn handoff(&self, scope: &InvocationScope, input: Value) -> Value {
        let tid = scope.caller.thread_id.0.clone();
        if !lock(&self.in_flight).insert(tid.clone()) {
            return worktree_failure(
                "handoff_in_progress",
                format!("A worktree handoff is already in progress for thread '{tid}'."),
            );
        }
        let guard = FlightGuard {
            set: self.in_flight.clone(),
            key: tid.clone(),
        };
        // The owned task continues through CAS/continuation even if the client
        // disconnects, just as T3's uninterruptible post-checkout region does.
        let service = self.clone();
        match tokio::spawn(async move {
            let _guard = guard;
            service.perform_handoff(&tid, input).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => worktree_failure("operation_failed", "The operation could not be completed."),
        }
    }

    async fn perform_handoff(&self, tid: &str, input: Value) -> Value {
        let p = match self.kernel.store.thread(&ThreadId(tid.into())) {
            Ok(Some(p)) if p.thread.deleted_at.is_none() => p,
            _ => {
                return worktree_failure(
                    "thread_not_found",
                    format!("Thread '{tid}' was not found."),
                );
            }
        };
        if let Some(path) = p.thread.worktree_path.as_deref() {
            return worktree_failure(
                "already_in_worktree",
                format!("Thread '{tid}' is already attached to worktree '{path}'."),
            );
        }
        if p.thread.archived_at.is_some() {
            return worktree_failure(
                "invalid_request",
                format!("Thread '{tid}' is archived and cannot be handed off to a worktree."),
            );
        }
        let project = match self.project(&p.thread.project_id.0) {
            Ok(p) => p,
            Err(_) => {
                return worktree_failure(
                    "project_not_found",
                    format!(
                        "Project '{}' was not found for thread '{tid}'.",
                        p.thread.project_id
                    ),
                );
            }
        };
        if let Some(path) = input["path"]
            .as_str()
            .filter(|path| !Path::new(path).is_absolute())
        {
            return worktree_failure(
                "invalid_request",
                format!(
                    "path must be an absolute filesystem path, got '{path}'. A relative path would be created relative to the project workspace but stored verbatim as the thread's worktree binding."
                ),
            );
        }
        let root = Path::new(project["workspaceRoot"].as_str().unwrap());
        if !self.repos.is_repo(root).await {
            return worktree_failure(
                "invalid_request",
                format!(
                    "Project workspace '{}' is not a git repository.",
                    root.display()
                ),
            );
        }
        let branch = input["branch"].as_str().unwrap();
        let refs = match self.repos.orchestration_refs(root, true).await {
            Ok(refs) => refs,
            Err(_) => {
                return worktree_failure(
                    "operation_failed",
                    "Unable to list branches: The operation could not be completed.",
                );
            }
        };
        if let Some(existing) = refs
            .iter()
            .find(|r| r["name"] == branch && r["isRemote"] == false)
        {
            return worktree_failure(
                "invalid_request",
                format!(
                    "Branch '{branch}' already exists{}. Choose a different branch name, or delete the existing branch{} first.",
                    existing["worktreePath"]
                        .as_str()
                        .map(|p| format!(" and is checked out at '{p}'"))
                        .unwrap_or_default(),
                    if existing["worktreePath"].is_string() {
                        " and its worktree"
                    } else {
                        ""
                    }
                ),
            );
        }
        let base = match input["baseRef"].as_str() {
            Some(base) => base.to_owned(),
            None => match self.repos.current_branch(root).await {
                Ok(base) if base != "HEAD" => base,
                _ => {
                    return worktree_failure(
                        "invalid_request",
                        "Could not determine the current branch of the project workspace (detached HEAD?). Pass baseRef explicitly.",
                    );
                }
            },
        };
        let origin = input["startFromOrigin"].as_bool().unwrap_or(
            self.preferences()
                .ok()
                .is_some_and(|p| p["newWorktreesStartFromOrigin"] == true),
        );
        let operation_id = id();
        let destination = input["path"]
            .as_str()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| self.repos.orchestration_worktree_path(root, branch));
        if destination.symlink_metadata().is_ok() {
            return worktree_failure(
                "operation_failed",
                format!(
                    "Unable to create the worktree: Worktree destination already exists: {}",
                    destination.display()
                ),
            );
        }
        let path = destination.to_string_lossy().into_owned();
        let driver = self
            .registry
            .provider_instances
            .snapshot(&self.registry)
            .into_iter()
            .find(|i| i.provider_instance_id == p.thread.provider_instance_id)
            .map(|i| i.driver_kind.0)
            .unwrap_or("unknown".into());
        // Journal the exact resource and continuation before entering Git.
        // Recovery uses the same preparation path and never regenerates ids.
        let mut workflow = json!({"threadId":tid,"projectId":project["id"],"commandId":format!("handoff:{operation_id}"),"kind":"handoff",
            "status":"preparing","stage":"fetch","worktreePath":path,"branch":branch,"projectWorkspaceRoot":project["workspaceRoot"],
            "strategy":{"type":"worktree","baseRef":base,"branch":branch,"startFromOrigin":origin},"driver":driver,
            "continuationPrompt":input["continuationPrompt"],"runSetupScript":input["runSetupScript"],
            "createdWorktree":false,"checkoutIntent":true,"bound":false,"setup":null});
        if self
            .save("orchestration_launch_workflows", tid, &workflow)
            .is_err()
        {
            return worktree_failure("operation_failed", "The operation could not be completed.");
        }
        let start = if origin {
            if let Err(e) = self
                .repos
                .orchestration_git(root, &["fetch", "origin"])
                .await
            {
                return self.fail_handoff(
                    tid,
                    &mut workflow,
                    "operation_failed",
                    format!("Unable to fetch origin: {e}"),
                );
            }
            let remote = if base.starts_with("origin/") {
                base.clone()
            } else {
                format!("origin/{base}")
            };
            match self
                .repos
                .orchestration_git(
                    root,
                    &["rev-parse", "--verify", &format!("{remote}^{{commit}}")],
                )
                .await
            {
                Ok(commit) => commit.trim().into(),
                Err(e) => {
                    return self.fail_handoff(
                        tid,
                        &mut workflow,
                        "operation_failed",
                        format!("Unable to resolve the remote-tracking commit of '{base}': {e}"),
                    );
                }
            }
        } else {
            base.clone()
        };
        if !destination.is_absolute() {
            return worktree_failure(
                "invalid_request",
                format!(
                    "path must be an absolute filesystem path, got '{}'. A relative path would be created relative to the project workspace but stored verbatim as the thread's worktree binding.",
                    destination.display()
                ),
            );
        }
        workflow["resolvedBaseRef"] = json!(start);
        workflow["fetchCompleted"] = json!(true);
        if self.stage(tid, &mut workflow, "checkout").is_err() {
            return worktree_failure("operation_failed", "The operation could not be completed.");
        }
        if let Err(e) = self
            .repos
            .create_claimed_worktree(
                root,
                &destination,
                branch,
                &start,
                workflow["commandId"].as_str(),
            )
            .await
        {
            return self.fail_handoff(
                tid,
                &mut workflow,
                "operation_failed",
                format!("Unable to create the worktree: {e}"),
            );
        }
        workflow["createdWorktree"] = json!(true);
        workflow["bound"] = json!(true);
        workflow["stage"] = json!("setup-script");
        let binding = self
            .operation(
                tid,
                &format!("handoff:{operation_id}"),
                LaunchOperation::Bind {
                    expected: None,
                    path: Some(path.clone()),
                    branch: Some(branch.into()),
                    continuation: input["continuationPrompt"].as_str().map(str::to_owned),
                    driver: driver.clone(),
                    workflow: Some(workflow.clone()),
                },
            )
            .await;
        if let Err(error) = binding {
            // Read after uncertain response; NEVER clean a committed binding.
            if self
                .kernel
                .store
                .thread(&ThreadId(tid.into()))
                .ok()
                .flatten()
                .is_some_and(|p| p.thread.worktree_path.as_deref() != Some(&path))
            {
                let _ = self
                    .repos
                    .remove_created_worktree(root, &destination, branch)
                    .await;
                let _ = self.fail_handoff(
                    tid,
                    &mut workflow,
                    "operation_failed",
                    error.message.clone(),
                );
            }
            if error.message.contains("Workspace binding changed") {
                if let Ok(Some(p)) = self.kernel.store.thread(&ThreadId(tid.into())) {
                    if let Some(path) = p.thread.worktree_path {
                        return worktree_failure(
                            "already_in_worktree",
                            format!("Thread '{tid}' is already attached to worktree '{path}'."),
                        );
                    }
                }
            }
            return worktree_failure(
                "operation_failed",
                format!(
                    "Unable to re-point the thread at the worktree: {}",
                    error.message
                ),
            );
        }
        let _ = self.workspace.set_chat_cwd(tid, &path);
        let _ = self.workspace.set_chat_branch(tid, branch);
        let _ = self
            .repos
            .release_worktree_claim(root, &destination, workflow["commandId"].as_str().unwrap())
            .await;
        let continuation = if input["continuationPrompt"].is_string() {
            json!({"status":"scheduled","delivery":"queued"})
        } else {
            json!({"status":"skipped"})
        };
        let setup_blocking = input["runSetupScript"] != false
            && project["scripts"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|s| s["runOnWorktreeCreate"] == true)
                .is_some_and(|s| s["async"] == false);
        let setup = if input["runSetupScript"] == false {
            json!({"status":"skipped"})
        } else {
            match self
                .setup(tid, &project, &destination, &mut workflow, false)
                .await
            {
                Ok(_) => workflow["setupScript"].clone(),
                Err(e) => json!({"status":"failed","detail":e.message}),
            }
        };
        // Provider detach is after the atomic continuation + readiness hold.
        if self.intake.detach(tid).await.is_ok() {
            // Preserve an observer's setup completion, rather than overwriting
            // it with the pre-spawn snapshot.
            if let Ok(mut current) = self.workflow(tid) {
                current["detachCompleted"] = json!(true);
                let _ = self.save("orchestration_launch_workflows", tid, &current);
            }
        }
        if self.workflow(tid).is_ok_and(|w| {
            w["detachCompleted"] == true && (!setup_blocking || w["setup"]["status"] == "completed")
        }) {
            let _ = self
                .operation(
                    tid,
                    &format!("handoff:{operation_id}:release"),
                    LaunchOperation::Release,
                )
                .await;
        }
        let note = if continuation["status"] == "scheduled" {
            "Handoff recorded. Changing the workspace detaches this provider session, so the current turn ends shortly after this call; the queued continuation prompt then starts the next turn inside the worktree with the conversation preserved. The worktree is not removed automatically when the thread is deleted."
        } else {
            "Handoff recorded. Changing the workspace detaches this provider session, so the current turn ends shortly after this call; the conversation continues inside the worktree when the thread receives its next message. Pass continuationPrompt to resume automatically. The worktree is not removed automatically when the thread is deleted."
        };
        json!({"worktreePath":path,"branch":branch,"baseRef":base,"startedFromOrigin":origin,"setupScript":setup,"continuation":continuation,"note":note})
    }

    fn fail_handoff(
        &self,
        tid: &str,
        workflow: &mut Value,
        code: &str,
        detail: impl Into<String>,
    ) -> Value {
        let detail = detail.into();
        workflow["status"] = json!("failed");
        workflow["stage"] = json!("failed");
        workflow["error"] = json!(detail);
        let _ = self.save("orchestration_launch_workflows", tid, workflow);
        worktree_failure(code, detail)
    }
}

#[async_trait::async_trait]
impl crate::orchestration::launch_service::ScheduledThreadLaunch for HostLaunchService {
    async fn launch_scheduled(
        &self,
        input: crate::orchestration::launch_service::HostThreadLaunchRequest,
    ) -> Result<(), ToolError> {
        self.launch_host(
            json!({
                "projectId":input.project_id,"title":input.title,"modelSelection":input.model_selection,
                "runtimeMode":input.runtime_mode,"interactionMode":input.interaction_mode,
                "workspaceStrategy":input.workspace_strategy,"messageId":input.message_id,
                "scheduledTaskId":input.scheduled_task_id,"message":input.text,"attachments":[],
                "createdBy":input.created_by,"creationSource":input.creation_source
            }),
            input.command_id.0,
        ).await.map(|_| ())
    }
}

struct FlightGuard {
    set: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    key: String,
}
impl Drop for FlightGuard {
    fn drop(&mut self) {
        lock(&self.set).remove(&self.key);
    }
}
struct ShaName;
fn same_checkout(left: &Path, right: &Path) -> bool {
    std::fs::canonicalize(left)
        .ok()
        .zip(std::fs::canonicalize(right).ok())
        .is_some_and(|(left, right)| left == right)
}

impl ShaName {
    fn of(s: &str) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(s.as_bytes()))
    }
}
