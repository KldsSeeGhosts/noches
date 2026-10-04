//! End-to-end: a GUI/service host resolves shell-only CLIs AND supplies their
//! provider credentials, without needing a key on the mobile companion.
//!
//! This file must stay a single test: it mutates process env (SHELL/PATH/HOME)
//! and warms the process-global login-shell snapshot cache, so it needs its
//! own test binary with no parallel siblings.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use zeron_harness::{AcpHarness, Harness as _};

fn write_executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[tokio::test]
async fn gui_host_resolves_clis_and_launches_with_shell_provider_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let shell_bin = dir.path().join("shell-bin");
    std::fs::create_dir(&shell_bin).unwrap();
    write_executable(&shell_bin.join("devin"), "#!/bin/sh\nexit 0\n");
    write_executable(&shell_bin.join("hermes"), "#!/bin/sh\nexit 0\n");
    write_executable(&shell_bin.join("pi-acp"), "#!/bin/sh\nexit 0\n");
    write_executable(&shell_bin.join("claude"), "#!/bin/sh\nexit 0\n");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-codex.sh");
    write_executable(
        &shell_bin.join("codex"),
        &format!(
            "#!/bin/sh\n[ \"$CPA_API_KEY\" = test-only-shell-key ] || exit 12\nexec /bin/sh '{}' \"$@\"\n",
            fixture.display().to_string().replace('\'', "'\\''")
        ),
    );

    // A $SHELL whose init shapes PATH — the shape resolution must survive.
    let fake_shell = dir.path().join("fake-shell");
    write_executable(
        &fake_shell,
        &format!(
            "#!/bin/sh\nPATH=\"{}:/usr/bin:/bin\"; export PATH\n\
             export CPA_API_KEY=test-only-shell-key\n\
             while [ \"$#\" -gt 0 ]; do\n\
               if [ \"$1\" = \"-c\" ]; then shift; exec /bin/sh -c \"$1\"; fi\n\
               shift\n\
             done\nexit 1\n",
            shell_bin.display()
        ),
    );

    // A GUI/service-launch environment: minimal PATH, no CLIs reachable, HOME
    // pointed away from any real install dirs.
    // SAFETY: single-test binary — nothing else reads env concurrently.
    unsafe {
        std::env::set_var("SHELL", &fake_shell);
        std::env::set_var("HOME", dir.path());
        std::env::set_var("PATH", "/usr/bin:/bin");
        std::env::remove_var("DEVIN_EXECUTABLE");
        std::env::remove_var("HERMES_EXECUTABLE");
        std::env::remove_var("PI_ACP_EXECUTABLE");
        std::env::remove_var("CLAUDE_CODE_EXECUTABLE");
        std::env::remove_var("CODEX_EXECUTABLE");
        std::env::remove_var("CPA_API_KEY");
        std::env::remove_var("ZERON_NO_LOGIN_SHELL");
    }

    let snapshot = zeron_harness::shell_env::login_shell_path().expect("snapshot captured");
    let snapshot = snapshot.to_string_lossy();
    assert!(
        snapshot.starts_with(&format!("{}:", shell_bin.display())),
        "snapshot should carry the shell-shaped PATH, got: {snapshot}"
    );

    // The agent binaries are only reachable through the snapshot; the
    // launch program (not the npx fallback) must be the shell-PATH binary,
    // proving resolution consulted the login-shell snapshot.
    // Native drivers consult the same snapshot for the agent CLI itself.
    assert!(
        zeron_harness::ClaudeHarness::new().installed(),
        "claude resolves via login-shell PATH"
    );
    let devin = AcpHarness::devin()
        .launch_program()
        .expect("devin resolves via login-shell PATH");
    assert_eq!(devin, shell_bin.join("devin"), "{devin:?}");
    let hermes = AcpHarness::hermes()
        .launch_program()
        .expect("hermes resolves via login-shell PATH");
    assert_eq!(hermes, shell_bin.join("hermes"), "{hermes:?}");
    let pi = AcpHarness::pi()
        .launch_program()
        .expect("pi-acp resolves via login-shell PATH");
    assert_eq!(pi, shell_bin.join("pi-acp"), "{pi:?}");

    // The actual Codex child refuses to start without the shell-only CPA key.
    // Successful skills/model probes prove the shared environment reaches both
    // discovery paths, rather than returning the fallback catalog.
    let codex = zeron_harness::CodexHarness::new();
    assert!(
        codex
            .commands()
            .await
            .unwrap()
            .iter()
            .any(|c| c.name == "bare")
    );
    assert_eq!(codex.models().await.unwrap().len(), 3);

    use futures::StreamExt;
    use zeron_proto::{AgentEvent, DoneStatus, RunRequest, SandboxLevel};
    for title in [false, true] {
        let request = RunRequest {
            instance_id: None,
            prompt: if title {
                "scenario:title"
            } else {
                "scenario:publication"
            }
            .into(),
            harness: None,
            model: None,
            reasoning: None,
            model_options: serde_json::Map::new(),
            cwd: String::new(),
            sandbox: SandboxLevel::WorkspaceWrite,
            runtime_mode: Default::default(),
            interaction_mode: Default::default(),
            auto_approve: true,
            attachments: Vec::new(),
            worktree: None,
            resume: None,
        };
        let (steering, receiver) = tokio::sync::mpsc::channel(1);
        let controls = zeron_harness::RunControls {
            mcp: Default::default(),
            browser: None,
            request_permission: zeron_harness::refuse_permissions(),
            request_input: Box::new(|_| panic!("fixture requires no input")),
            steering: receiver,
            interrupt: zeron_harness::CancellationToken::new(),
            computer_use_socket: None,
        };
        let stream = if title {
            codex.run_title(request, controls).await
        } else {
            codex.run(request, controls).await
        }
        .unwrap();
        let events = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            stream.collect::<Vec<_>>(),
        )
        .await
        .unwrap();
        drop(steering);
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                ..
            })
        )));
    }

    // Login uses the same environment before applying its isolated CODEX_HOME.
    let login = zeron_harness::codex::login_command(&dir.path().join("login")).unwrap();
    assert!(login.as_std().get_envs().any(|(key, value)| {
        key == "CPA_API_KEY" && value == Some(std::ffi::OsStr::new("test-only-shell-key"))
    }));

    // Verify precedence at a real child boundary, not just in command metadata.
    for (value, check) in [
        (
            Some("explicit-child-key"),
            "[ \"$CPA_API_KEY\" = explicit-child-key ]",
        ),
        (None, "[ \"${CPA_API_KEY+x}\" != x ]"),
    ] {
        let mut command = zeron_harness::process::Command::new("/bin/sh");
        if let Some(value) = value {
            command.env("CPA_API_KEY", value);
        } else {
            command.env_remove("CPA_API_KEY");
        }
        zeron_harness::compose_child_environment(&mut command, Path::new("/bin/sh"));
        assert!(
            command
                .arg("-c")
                .arg(check)
                .status()
                .await
                .unwrap()
                .success()
        );
    }
    assert!(
        std::env::var_os("CPA_API_KEY").is_none(),
        "host environment is never mutated"
    );
}
