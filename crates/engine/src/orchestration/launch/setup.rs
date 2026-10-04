//! Opt-in trusted setup readiness. Interactive shell acceptance is NOT completion.
use super::{HostLaunchService, ToolError, id, lock, unavailable};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use zeron_proto::launch::SetupRun;
use zeron_proto::{ProjectAction, TerminalEvent};

impl HostLaunchService {
    pub(crate) async fn setup(
        &self,
        thread: &str,
        project: &Value,
        cwd: &std::path::Path,
        w: &mut Value,
        wait_blocking: bool,
    ) -> Result<bool, ToolError> {
        if self
            .kernel
            .store
            .thread(&zeron_proto::orchestration::ThreadId(thread.into()))
            .map_err(|_| unavailable())?
            .is_none_or(|p| p.thread.deleted_at.is_some())
        {
            return Err(super::invalid("The thread was not found."));
        }
        if let Some(setup) = w.get("setup").filter(|s| !s.is_null()) {
            return Ok(
                matches!(setup["status"].as_str(), Some("completed" | "continued"))
                    || setup["blocking"] == false,
            );
        }
        let root = std::path::Path::new(project["workspaceRoot"].as_str().unwrap());
        let Some(action) = self
            .actions
            .setup_action(project["id"].as_str().unwrap(), root)
            .map_err(|_| unavailable())?
        else {
            w["setupScript"] = json!({"status":"no-script"});
            return Ok(true);
        };
        // Only a saved script that explicitly opts out of async blocks.
        let script = project["scripts"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s["name"] == action.name);
        let blocking = script.is_some_and(|s| s["async"] == false);
        let timeout_ms = w["setupTimeoutMs"]
            .as_u64()
            .unwrap_or(300_000)
            .clamp(1, 3_600_000);
        let run_id = id();
        let mut setup = SetupRun {
            run_id: run_id.clone(),
            status: "pending".into(),
            blocking,
            timeout_ms,
            script_name: action.name.clone(),
            ..Default::default()
        };
        w["setup"] = serde_json::to_value(&setup).unwrap();
        self.save("orchestration_launch_workflows", thread, w)
            .map_err(|_| unavailable())?;
        // Subshell ensures shell initialization or a command's `exit` cannot
        // make PTY acceptance look like setup success.
        let shell = if cfg!(windows) {
            std::env::var("COMSPEC").unwrap_or("powershell.exe".into())
        } else {
            std::env::var("SHELL").unwrap_or_default()
        };
        let command = setup_command(&shell, &action.command);
        let tracked = ProjectAction { command, ..action };
        let run = match crate::project_actions::launch_project_setup_action(
            &self.terminals,
            &tracked,
            root,
            cwd,
            100,
            30,
        ) {
            Ok(run) => run,
            Err(error) => {
                setup.status = "failed".into();
                setup.detail = Some(error.to_string());
                w["setup"] = serde_json::to_value(&setup).unwrap();
                w["setupScript"] = json!({"status":"failed","detail":setup.detail});
                if blocking {
                    w["status"] = json!("blocked");
                }
                self.save("orchestration_launch_workflows", thread, w)
                    .map_err(|_| unavailable())?;
                return if blocking {
                    Ok(false)
                } else {
                    Err(unavailable())
                };
            }
        };
        setup.terminal_id = Some(run.terminal.id.clone());
        setup.status = "running".into();
        w["setup"] = serde_json::to_value(&setup).unwrap();
        w["setupScript"] =
            json!({"status":"started","scriptName":run.action_name,"terminalId":run.terminal.id});
        self.save("orchestration_launch_workflows", thread, w)
            .map_err(|_| unavailable())?;
        let cancel = CancellationToken::new();
        lock(&self.setup_cancels).insert(run_id.clone(), cancel.clone());
        let service = self.clone();
        let tid = thread.to_owned();
        let observe = async move {
            let mut setup = setup;
            let terminal = setup.terminal_id.clone().unwrap();
            let mut events = service
                .terminals
                .subscribe(&terminal, None)
                .map_err(|_| unavailable())?;
            let wait = async {
                while let Some(event) = events.recv().await {
                    if let TerminalEvent::Exit { exit_code, .. } = event {
                        return Some(exit_code);
                    }
                }
                None
            };
            tokio::select! {
                _=cancel.cancelled()=>{setup.status="cancelled".into(); let _=service.terminals.close(&terminal);}
                result=tokio::time::timeout(std::time::Duration::from_millis(timeout_ms),wait)=>{
                    match result {
                        Ok(Some(exit))=>{setup.exit_code=Some(exit); setup.status=if exit==0{"completed"}else{"failed"}.into();}
                        Ok(None)=>{setup.status="failed".into(); setup.detail=Some("Setup terminal closed without an exit code.".into());}
                        Err(_)=>{setup.status="timed_out".into(); let _=service.terminals.close(&terminal);}
                    }
                }
            }
            lock(&service.setup_cancels).remove(&setup.run_id);
            // CAS run id: a late previous attempt cannot overwrite retry/continue.
            let applied = service.kernel.store.write(|tx|{
                let raw:String=tx.query_row("SELECT payload FROM orchestration_launch_workflows WHERE thread_id=?1",[&tid],|r|r.get(0))?;
                let mut w:Value=serde_json::from_str(&raw)?;
                if w["setup"]["runId"]==setup.run_id && w["setup"]["status"]!="continued" {
                    w["setup"]=serde_json::to_value(&setup)?;
                    if blocking && setup.status!="completed" { w["status"]=json!("blocked"); }
                    tx.execute("UPDATE orchestration_launch_workflows SET payload=?2 WHERE thread_id=?1",rusqlite::params![tid,w.to_string()])?;
                    return Ok(true);
                }
                Ok(false)
            }).map_err(|_|unavailable())?;
            if applied && blocking && !wait_blocking && setup.status == "completed" {
                let w = service.workflow(&tid)?;
                if w["kind"] != "handoff" || w["detachCompleted"] == true {
                    service
                        .operation(
                            &tid,
                            &format!("{}:release", w["commandId"].as_str().unwrap()),
                            super::LaunchOperation::Release,
                        )
                        .await?;
                }
            }
            Ok::<_, ToolError>(setup.status == "completed")
        };
        if blocking && wait_blocking {
            observe.await
        } else {
            tokio::spawn(async move {
                let _ = observe.await;
            });
            Ok(true)
        }
    }

    pub async fn setup_control(
        &self,
        params: zeron_proto::launch::SetupControlParams,
    ) -> Result<(), ToolError> {
        if self
            .kernel
            .store
            .thread(&zeron_proto::orchestration::ThreadId(
                params.chat_id.clone(),
            ))
            .map_err(|_| unavailable())?
            .is_none_or(|p| p.thread.deleted_at.is_some())
        {
            return Err(super::invalid("The thread was not found."));
        }
        let mut w = self.workflow(&params.chat_id)?;
        if w["setup"]["runId"] != params.run_id {
            return Err(super::invalid("The setup run is no longer current."));
        }
        match params.action.as_str() {
            "cancel" => {
                if let Some(cancel) = lock(&self.setup_cancels).get(&params.run_id) {
                    cancel.cancel();
                }
            }
            "continue" => {
                if let Some(cancel) = lock(&self.setup_cancels).get(&params.run_id) {
                    cancel.cancel();
                }
                w["setup"]["status"] = json!("continued");
                w["status"] = json!("preparing");
                self.save("orchestration_launch_workflows", &params.chat_id, &w)
                    .map_err(|_| unavailable())?;
                self.spawn_preparation(params.chat_id);
            }
            "retry" => {
                if matches!(w["setup"]["status"].as_str(), Some("pending" | "running")) {
                    return Err(super::invalid("The setup run is still running."));
                }
                w["setup"] = Value::Null;
                w["status"] = json!("preparing");
                self.save("orchestration_launch_workflows", &params.chat_id, &w)
                    .map_err(|_| unavailable())?;
                self.spawn_preparation(params.chat_id);
            }
            _ => return Err(super::invalid("Unknown setup control action.")),
        }
        Ok(())
    }
}

fn setup_command(shell: &str, command: &str) -> String {
    let shell = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase();
    match shell.trim_end_matches(".exe") {
        "fish" => format!("begin\n{command}\nend\nexit $status\n"),
        "cmd" => format!("(\r\n{command}\r\n)\r\nexit %errorlevel%\r\n"),
        "powershell" | "pwsh" => format!(
            "try {{\n  & {{\n{command}\n  }}\n  if (-not $?) {{ exit 1 }}\n  if ($null -ne $LASTEXITCODE) {{ exit $LASTEXITCODE }}\n  exit 0\n}} catch {{ Write-Error $_; exit 1 }}\n"
        ),
        _ => format!("(\n{command}\n)\nexit $?\n"),
    }
}

#[cfg(test)]
#[test]
fn setup_uses_the_owning_shell_exit_status() {
    assert!(setup_command("/bin/zsh", "false").ends_with("exit $?\n"));
    assert!(setup_command("/usr/bin/fish", "false").ends_with("exit $status\n"));
    assert!(setup_command("C:\\Windows\\cmd.exe", "echo test").ends_with("exit %errorlevel%\r\n"));
    assert!(setup_command("pwsh.exe", "exit 2").contains("exit $LASTEXITCODE"));
}
