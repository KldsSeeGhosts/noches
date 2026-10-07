//! Pi's native session (its session file path) survives an idle process death:
//! the next dispatch resumes the stored session via `--session`, through the
//! real engine dispatch path.
#![cfg(unix)]
use std::{sync::Arc, time::Duration};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::{PiHarness, instance::InstanceLaunch};
use zeron_proto::{HarnessId, RunRequest, SandboxLevel};

fn request(prompt: &str, cwd: &std::path::Path) -> RunRequest {
    RunRequest {
        instance_id: None,
        prompt: prompt.into(),
        harness: None,
        model: None,
        reasoning: None,
        model_options: Default::default(),
        cwd: cwd.display().to_string(),
        sandbox: SandboxLevel::WorkspaceWrite,
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
        auto_approve: true,
        attachments: Vec::new(),
        worktree: None,
        resume: None,
    }
}

#[tokio::test]
async fn pi_idle_crash_next_dispatch_resumes_the_stored_session_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../harness/tests/fixtures/fake-pi.py");
    std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o755)).unwrap();
    let sessions = dir.path().join("pi-sessions");
    let launch = InstanceLaunch::new(
        [
            ("FAKE_PI_SESSIONS", sessions.display().to_string()),
            // Where real Pi keeps sessions; resume only trusts files under it.
            ("PI_CODING_AGENT_SESSION_DIR", sessions.display().to_string()),
            ("PI_CODING_AGENT_DIR", dir.path().join("agent").display().to_string()),
            ("FAKE_PI_LOG", dir.path().join("pi.log").display().to_string()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
        Vec::new(),
    );
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(
        PiHarness::new()
            .with_executable(fixture)
            .with_instance_launch(launch)
            .with_graces(Duration::from_millis(50), Duration::from_millis(100)),
    ));
    let core = EngineCore::assemble(dir.path(), Arc::new(registry), HarnessId::Pi, None).unwrap();
    // Startup discovery probes the fixture's models and then account auth. Let
    // it finish so it cannot race the first dispatch, then mark the fixture as
    // a signed-in Pi so the test never depends on this host's Pi login.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !core
            .registry
            .provider_instances
            .snapshot(&core.registry)
            .iter()
            .any(|p| p.harness_id == Some(HarnessId::Pi) && p.models.len() >= 2)
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("startup provider discovery");
    core.registry.provider_instances.set_authentication(
        HarnessId::Pi,
        zeron_engine::provider_instances::Authentication::Authenticated,
    );
    let chat = "pi-idle-crash";
    let handle = core.doc_host.open(chat).unwrap();
    for (prompt, expected) in [
        ("idle-crash", "reply:idle-crash"),
        // The fixture answers NOT-RESUMED unless it was launched with --session.
        ("require-resume", "reply:require-resume"),
    ] {
        core.sessions
            .dispatch(chat, HarnessId::Pi, request(prompt, dir.path()), None)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let entries = handle.doc().read_entries().unwrap();
                if entries.iter().any(|entry| {
                    entry.parts.iter().any(|part| {
                        matches!(part, zeron_doc::MessagePart::Text { text, .. }
                            if text.trim_end() == expected)
                    })
                }) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{prompt}: no `{expected}` (resume not honored?)"));
        // Let the fixture exit and the driver remove its live mailbox.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    // One session file served both turns.
    assert_eq!(std::fs::read_dir(&sessions).unwrap().count(), 1);
}
