use super::{HostLaunchService, ToolError, id, invalid, unavailable};
use crate::mcp::auth::InvocationScope;
use serde_json::{Value, json};
use std::path::Path;

impl HostLaunchService {
    /// Adopt existing owned spaces once, preserving their ids and checkout roots.
    pub(crate) fn projects(&self) -> Result<Vec<Value>, ToolError> {
        let mut rows = self
            .rows("orchestration_launch_projects")
            .map_err(|_| unavailable())?;
        for space in self
            .workspace
            .read_spaces()
            .map_err(|_| unavailable())?
            .into_iter()
            .filter(|s| s.device_id == self.workspace.device_id())
        {
            if rows.iter().any(|p| {
                p["id"] == space.id
                    || (p["deletedAt"].is_null() && p["workspaceRoot"] == space.path)
            }) {
                continue;
            }
            let project = json!({"id":space.id,"title":space.name.unwrap_or_else(|| Path::new(&space.path).file_name().unwrap_or_default().to_string_lossy().into_owned()),
                "workspaceRoot":space.path,"defaultModelSelection":null,"scripts":[],
                "createdAt":space.created_at,"updatedAt":space.created_at,"deletedAt":null});
            self.persist_project(&project)?;
            rows.push(project);
        }
        Ok(rows)
    }

    pub(crate) fn project(&self, project_id: &str) -> Result<Value, ToolError> {
        self.projects()?
            .into_iter()
            .find(|p| p["id"] == project_id && p["deletedAt"].is_null())
            .ok_or_else(|| invalid("The project was not found."))
    }

    pub(crate) fn persist_project(&self, project: &Value) -> Result<(), ToolError> {
        self.kernel.store.write(|tx| {
            tx.execute("INSERT INTO orchestration_launch_projects VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET root=excluded.root,payload=excluded.payload",
                rusqlite::params![project["id"].as_str(),project["workspaceRoot"].as_str(),project.to_string()])?;
            Ok(())
        }).map_err(|_| unavailable())
    }

    pub(crate) async fn project_call(
        &self,
        scope: &InvocationScope,
        name: &str,
        input: Value,
    ) -> Result<Value, ToolError> {
        if matches!(name, "t3_project_list" | "t3_project_read") {
            self.caller(scope, false)?;
        } else {
            self.require_full(
                scope,
                "Project changes require a live full-access/default calling thread.",
            )?;
        }
        match name {
            "t3_project_list" => {
                let rows: Vec<_> = self
                    .projects()?
                    .into_iter()
                    .filter(|p| p["deletedAt"].is_null())
                    .collect();
                let start = input["cursor"].as_u64().unwrap_or(0) as usize;
                let end = start.saturating_add(input["limit"].as_u64().unwrap_or(20) as usize);
                Ok(
                    json!({"projects":rows.iter().skip(start).take(end-start).collect::<Vec<_>>(),"nextCursor":(end<rows.len()).then_some(end)}),
                )
            }
            "t3_project_read" => self.project(input["projectId"].as_str().unwrap()),
            "t3_project_create" => self.create_project(input).await,
            "t3_project_update" => {
                let mut project = self.project(input["projectId"].as_str().unwrap())?;
                if let Some(root) = input["workspaceRoot"].as_str() {
                    let root = std::fs::canonicalize(root).map_err(|_| unavailable())?;
                    if !root.is_dir() {
                        return Err(unavailable());
                    }
                    let root = root.to_string_lossy().into_owned();
                    if self.projects()?.iter().any(|p| {
                        p["workspaceRoot"] == root
                            && p["id"] != project["id"]
                            && p["deletedAt"].is_null()
                    }) {
                        return Err(invalid("The workspace is already registered to a project."));
                    }
                    project["workspaceRoot"] = json!(root);
                }
                for key in [
                    "title",
                    "defaultModelSelection",
                    "autoPull",
                    "projectIcon",
                    "faviconPath",
                    "defaultThreadEnvMode",
                    "scripts",
                ] {
                    if let Some(value) = input.get(key) {
                        project[key] = value.clone();
                    }
                }
                project["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());
                if input.get("scripts").is_some() || input.get("workspaceRoot").is_some() {
                    self.actions
                        .replace_orchestration_scripts(
                            project["id"].as_str().unwrap(),
                            Path::new(project["workspaceRoot"].as_str().unwrap()),
                            project["scripts"].as_array().unwrap(),
                        )
                        .map_err(|_| unavailable())?;
                }
                self.persist_project(&project)?;
                // The space is the existing discovery/navigation authority.
                let mut space = self
                    .workspace
                    .space(project["id"].as_str().unwrap())
                    .map_err(|_| unavailable())?
                    .ok_or_else(unavailable)?;
                space.path = project["workspaceRoot"].as_str().unwrap().into();
                space.name = project["title"].as_str().map(str::to_owned);
                self.workspace
                    .import_space_row(&space)
                    .map_err(|_| unavailable())?;
                Ok(project)
            }
            "t3_project_delete" => {
                let mut project = self.project(input["projectId"].as_str().unwrap())?;
                let pid = project["id"].as_str().unwrap().to_owned();
                let nonempty = self
                    .workspace
                    .read_chats()
                    .map_err(|_| unavailable())?
                    .iter()
                    .any(|c| c.space_id.as_deref() == Some(&pid))
                    || self
                        .kernel
                        .store
                        .launch_threads()
                        .map_err(|_| unavailable())?
                        .iter()
                        .any(|p| p.thread.project_id.0 == pid && p.thread.deleted_at.is_none());
                if nonempty && input["force"] != true {
                    return Err(invalid(
                        "The project is not empty; force=true is required to delete it.",
                    ));
                }
                // Soft-delete V2 records, then the existing registry cascade.
                for p in self
                    .kernel
                    .store
                    .launch_threads()
                    .map_err(|_| unavailable())?
                    .into_iter()
                    .filter(|p| p.thread.project_id.0 == pid && p.thread.deleted_at.is_none())
                {
                    self.operation(&p.thread.id.0, &id(), super::LaunchOperation::Delete)
                        .await?;
                    let _ = self.intake.detach(&p.thread.id.0).await;
                }
                self.workspace
                    .delete_space(&pid)
                    .map_err(|_| unavailable())?;
                project["deletedAt"] = json!(chrono::Utc::now().to_rfc3339());
                project["updatedAt"] = project["deletedAt"].clone();
                self.persist_project(&project)?;
                Ok(project)
            }
            "t3_project_clone" => {
                let destination = input["destinationPath"].as_str().unwrap();
                if Path::new(destination).symlink_metadata().is_ok() {
                    return Err(ToolError::new(
                        super::Code::OrchestrationError,
                        format!("Destination path already exists: {destination}"),
                    ));
                }
                let url = if let (Some(provider), Some(repo)) =
                    (input["provider"].as_str(), input["repository"].as_str())
                {
                    let host = match provider {
                        "github" => "github.com",
                        "gitlab" => "gitlab.com",
                        "bitbucket" => "bitbucket.org",
                        "unknown" => {
                            return Err(ToolError::new(
                                super::Code::OrchestrationError,
                                "Choose a source control provider before continuing.",
                            ));
                        }
                        _ => {
                            return Err(ToolError::new(
                                super::Code::OrchestrationError,
                                format!("No {provider} source control provider is registered."),
                            ));
                        }
                    };
                    if input["protocol"] == "https" {
                        format!("https://{host}/{repo}.git")
                    } else {
                        format!("git@{host}:{repo}.git")
                    }
                } else if let Some(remote) = input["remoteUrl"].as_str() {
                    remote.trim().to_owned()
                } else {
                    return Err(ToolError::new(
                        super::Code::OrchestrationError,
                        "Enter a repository path or clone URL before cloning.",
                    ));
                };
                self.repos
                    .clone_to(&url, Path::new(destination))
                    .await
                    .map_err(|e| {
                        ToolError::new(
                            super::Code::OrchestrationError,
                            redact_url_credentials(&e.to_string()),
                        )
                    })?;
                Ok(json!({"cwd":destination,"remoteUrl":redact_remote_url(&url),"repository":null}))
            }
            _ => Err(unavailable()),
        }
    }

    pub(crate) async fn create_project(&self, input: Value) -> Result<Value, ToolError> {
        let title = input["title"].as_str().unwrap();
        let managed = !input.get("workspaceRoot").is_some();
        if managed
            && [
                "scripts",
                "createWorkspaceRootIfMissing",
                "defaultModelSelection",
            ]
            .iter()
            .any(|k| input.get(k).is_some())
        {
            return Err(invalid(
                "A project started from its title takes only a title; set scripts or defaultModelSelection afterwards with t3_project_update.",
            ));
        }
        let mut commit_error = None;
        let root = if let Some(root) = input["workspaceRoot"].as_str() {
            if input["createWorkspaceRootIfMissing"] == true {
                std::fs::create_dir_all(root).map_err(|_| unavailable())?;
            }
            std::fs::canonicalize(root).map_err(|_| unavailable())?
        } else {
            let parent = self.data_dir.join("projects");
            std::fs::create_dir_all(&parent).map_err(|_| unavailable())?;
            let slug = project_folder_name(title);
            let mut claimed = None;
            for n in 1..=100 {
                let path = parent.join(if n == 1 {
                    slug.clone()
                } else {
                    format!("{slug}-{n}")
                });
                match std::fs::create_dir(&path) {
                    Ok(()) => {
                        claimed = Some(path);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(_) => {
                        return Err(ToolError::new(
                            super::Code::OrchestrationError,
                            "Failed to create the project folder.",
                        ));
                    }
                }
            }
            let path = claimed.ok_or_else(|| {
                ToolError::new(
                    super::Code::OrchestrationError,
                    "Failed to create the project folder.",
                )
            })?;
            let scaffold = self.scaffold(&path, title).await;
            match scaffold {
                Ok(error) => commit_error = error,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&path);
                    return Err(error);
                }
            }
            path
        };
        if !root.is_dir() {
            return Err(unavailable());
        }
        let root = root.to_string_lossy().into_owned();
        if self
            .projects()?
            .iter()
            .any(|p| p["workspaceRoot"] == root && p["deletedAt"].is_null())
        {
            return Err(invalid("The workspace is already registered to a project."));
        }
        let pid = id();
        let time = chrono::Utc::now().to_rfc3339();
        // T3's create ignores defaultModelSelection; only update records it.
        let mut project = json!({"id":pid,"title":title,"workspaceRoot":root,"defaultModelSelection":null,
            "scripts":input.get("scripts").cloned().unwrap_or(json!([])),"createdAt":time,"updatedAt":time,"deletedAt":null});
        self.persist_project(&project)?;
        if input.get("scripts").is_some() {
            self.actions
                .replace_orchestration_scripts(
                    &pid,
                    Path::new(&root),
                    project["scripts"].as_array().unwrap(),
                )
                .map_err(|_| unavailable())?;
        }
        self.workspace
            .create_space(
                &pid,
                self.workspace.device_id(),
                &root,
                Some(title.into()),
                self.repos.is_repo(Path::new(&root)).await,
            )
            .map_err(|_| unavailable())?;
        if let Some(error) = commit_error {
            project["commitError"] = json!(error);
        }
        Ok(project)
    }

    async fn scaffold(&self, path: &Path, title: &str) -> Result<Option<String>, ToolError> {
        let branch = self
            .repos
            .orchestration_git(path, &["config", "--get", "init.defaultBranch"])
            .await
            .unwrap_or("main".into());
        self.repos
            .orchestration_git(
                path,
                &["init", &format!("--initial-branch={}", branch.trim())],
            )
            .await
            .map_err(|_| unavailable())?;
        std::fs::write(path.join("README.md"),format!("<img src=\"assets/icon.svg\" width=\"64\" height=\"64\" alt=\"\">\n\n# {title}\n\nCreated in [T3 Code](https://t3.codes).\n")).map_err(|_| unavailable())?;
        std::fs::create_dir(path.join("assets")).map_err(|_| unavailable())?;
        let initial: String = title
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .take(2)
            .filter_map(|w| w.chars().next())
            .flat_map(char::to_uppercase)
            .collect();
        let initial = if initial.is_empty() {
            title.trim().chars().next().unwrap_or('?').to_string()
        } else {
            initial
        };
        let font_size = if initial.encode_utf16().count() > 1 {
            26
        } else {
            32
        };
        let initial = initial
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let colors = [
            "#dc2626", "#ea580c", "#d97706", "#16a34a", "#059669", "#0d9488", "#0891b2", "#0284c7",
            "#2563eb", "#4f46e5", "#7c3aed", "#9333ea", "#c026d3", "#db2777", "#e11d48",
        ];
        let hash = title
            .chars()
            .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32));
        std::fs::write(path.join("assets/icon.svg"),format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">\n  <rect width=\"64\" height=\"64\" rx=\"14\" fill=\"{}\"/>\n  <text x=\"32\" y=\"32\" dy=\"0.35em\" text-anchor=\"middle\" font-family=\"ui-sans-serif, system-ui, -apple-system, sans-serif\" font-size=\"{font_size}\" font-weight=\"600\" fill=\"#ffffff\">{initial}</text>\n</svg>\n",colors[hash as usize%colors.len()])).map_err(|_| unavailable())?;
        self.repos
            .orchestration_git(
                path,
                &["add", "--force", "--", "README.md", "assets/icon.svg"],
            )
            .await
            .map_err(|_| unavailable())?;
        Ok(self.repos.orchestration_git(path, &["commit","--message","Initial commit"]).await.err().map(|e| {
            let error=e.to_string();
            let lower = error.to_lowercase();
            if lower.contains("identity unknown") || lower.contains("tell me who you are") || lower.contains("no email was given") || lower.contains("no name was given") {
                "Git has no name or email on this machine. Set user.name and user.email, then commit.".into()
            } else { error.lines().map(str::trim).filter(|s|!s.is_empty()).next_back().unwrap_or("Git could not make the first commit.").into() }
        }))
    }
}

pub(crate) fn folder_words(title: &str) -> String {
    let value = title
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join("-");
    let value: String = value.chars().take(48).collect();
    value.trim_end_matches('-').into()
}

pub(crate) fn project_folder_name(title: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let normalized: String = title
        .nfkd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .flat_map(char::to_lowercase)
        .collect();
    let slug = normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug: String = slug.chars().take(64).collect();
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        return "project".into();
    }
    let reserved = matches!(slug, "con" | "prn" | "aux" | "nul")
        || ["com", "lpt"].iter().any(|prefix| {
            slug.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
            })
        });
    if reserved {
        format!("{slug}-project")
    } else {
        slug.into()
    }
}

fn redact_remote_url(value: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(value) else {
        return value.into();
    };
    if url.username().is_empty() && url.password().is_none() && url.query().is_none() {
        return value.into();
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.into()
}

fn redact_url_credentials(value: &str) -> String {
    static USERINFO: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b([a-z][a-z0-9+.-]*://)[^\s/]+@").unwrap()
    });
    USERINFO.replace_all(value, "$1").into_owned()
}

#[cfg(test)]
#[test]
fn clone_urls_and_errors_never_publish_url_credentials() {
    assert_eq!(
        redact_remote_url("https://user:secret@example.test/repo.git?token=secret"),
        "https://example.test/repo.git"
    );
    assert_eq!(
        redact_url_credentials("fatal: https://user:secret@example.test/repo.git failed"),
        "fatal: https://example.test/repo.git failed"
    );
}
