//! Wire-level policy/approval/question fixtures without installed providers.
#![cfg(unix)]

use futures::StreamExt;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use zeron_harness::{
    AcpHarness, CancellationToken, ClaudeHarness, CodexHarness, Harness, PermissionReceiver,
    RunControls,
};
use zeron_proto::{
    AgentEvent, DoneStatus, InteractionMode, PermissionDecision, RunRequest, RuntimeMode,
    UserInputAnswer,
};

fn fixture() -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policy-agent.py");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn roundtrip(
    harness: &dyn Harness,
    mode: RuntimeMode,
    decision: PermissionDecision,
    plan: bool,
) -> Value {
    let request: RunRequest = serde_json::from_value(json!({
        "prompt":"policy", "model":"fixture", "cwd":"/tmp", "sandbox":"workspace-write",
        "resume":null, "reasoning":null, "runtimeMode":mode,
        "interactionMode":if plan {InteractionMode::Plan} else {InteractionMode::Default}
    }))
    .unwrap();
    let seen = Arc::new(Mutex::new(0));
    let seen_permission = seen.clone();
    let unknown_seen = Arc::new(Mutex::new(0));
    let unknown_permission = unknown_seen.clone();
    let (_steer, steering) = mpsc::channel(1);
    let controls = RunControls {
        browser: None,
        computer_use_socket: None,
        steering,
        interrupt: CancellationToken::new(),
        request_permission: Box::new(move |request| {
            let unknown = request.description == "Future approval";
            if unknown {
                *unknown_permission.lock().unwrap() += 1;
                assert!(
                    request
                        .options
                        .iter()
                        .all(|o| o.decision == PermissionDecision::Decline)
                );
            } else {
                *seen_permission.lock().unwrap() += 1;
            }
            assert!(
                !request
                    .options
                    .iter()
                    .any(|o| o.decision == PermissionDecision::AcceptAlways)
            );
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(
                request
                    .options
                    .into_iter()
                    .find(|o| {
                        o.decision
                            == if unknown {
                                PermissionDecision::Decline
                            } else {
                                decision
                            }
                    })
                    .unwrap_or_default(),
            );
            PermissionReceiver::new(rx, || {})
        }),
        request_input: Box::new(|questions| {
            assert!(questions.iter().all(|q| q.question != "Future approval"));
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(
                questions
                    .into_iter()
                    .map(|q| UserInputAnswer {
                        question_id: q.id,
                        labels: vec!["B".into()],
                    })
                    .collect(),
            );
            rx
        }),
    };
    let mut stream = harness.run(request, controls).await.unwrap();
    let events = tokio::time::timeout(Duration::from_secs(10), async {
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            let event = event.unwrap();
            let done = matches!(event, AgentEvent::Done { .. });
            events.push(event);
            if done {
                break;
            }
        }
        events
    })
    .await
    .unwrap_or_else(|err| panic!("{:?} {mode:?} {decision:?}: {err}", harness.id()));
    assert!(
        matches!(
            events.last(),
            Some(AgentEvent::Done {
                status: DoneStatus::Completed,
                ..
            })
        ),
        "{events:?}"
    );
    assert_eq!(
        *seen.lock().unwrap(),
        if mode == RuntimeMode::FullAccess
            && matches!(
                harness.id(),
                zeron_proto::HarnessId::Grok
                    | zeron_proto::HarnessId::Antigravity
                    | zeron_proto::HarnessId::Hermes
            )
        {
            0
        } else {
            1
        }
    );
    assert_eq!(
        *unknown_seen.lock().unwrap(),
        if matches!(
            harness.id(),
            zeron_proto::HarnessId::Grok
                | zeron_proto::HarnessId::Antigravity
                | zeron_proto::HarnessId::Hermes
                | zeron_proto::HarnessId::Devin
        ) {
            1
        } else {
            0
        }
    );
    let text = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::Done {
                result: Some(text), ..
            } => Some(text),
            AgentEvent::TextDelta { text } => Some(text),
            _ => None,
        })
        .unwrap();
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
async fn claude_flags_and_session_scope_roundtrip() {
    for mode in RuntimeMode::ALL {
        for decision in [
            PermissionDecision::Accept,
            PermissionDecision::AcceptForSession,
            PermissionDecision::Decline,
            PermissionDecision::Cancel,
        ] {
            let value = roundtrip(
                &ClaudeHarness::new().with_executable(fixture()),
                mode,
                decision,
                false,
            )
            .await;
            let args = value["args"].as_array().unwrap();
            let expected = zeron_harness::policy::compile(
                zeron_proto::HarnessId::ClaudeCode,
                mode,
                InteractionMode::Default,
            )
            .unwrap()
            .claude_permission_mode;
            assert!(args.contains(&json!(expected)));
            let approval = &value["approval"]["response"]["response"];
            assert_eq!(
                approval["behavior"],
                if matches!(
                    decision,
                    PermissionDecision::Accept | PermissionDecision::AcceptForSession
                ) {
                    "allow"
                } else {
                    "deny"
                }
            );
            if decision == PermissionDecision::AcceptForSession {
                assert_eq!(approval["updatedPermissions"][0]["destination"], "session");
            }
            assert_eq!(
                value["question"]["response"]["response"]["updatedInput"]["answers"]["Pick"],
                "B"
            );
        }
    }
}

#[tokio::test]
async fn claude_permission_answer_cannot_exit_plan_mode() {
    let value = roundtrip(
        &ClaudeHarness::new().with_executable(fixture()),
        RuntimeMode::FullAccess,
        PermissionDecision::AcceptForSession,
        true,
    )
    .await;
    assert_eq!(
        value["planExit"]["response"]["response"]["behavior"],
        "deny"
    );
    assert!(value["args"].as_array().unwrap().contains(&json!("plan")));
}

#[tokio::test]
async fn codex_policy_and_permission_scope_roundtrip() {
    for mode in RuntimeMode::ALL {
        for decision in [
            PermissionDecision::Accept,
            PermissionDecision::AcceptForSession,
            PermissionDecision::Decline,
        ] {
            let value = roundtrip(
                &CodexHarness::new().with_executable(fixture()),
                mode,
                decision,
                true,
            )
            .await;
            let p = zeron_harness::policy::compile(
                zeron_proto::HarnessId::Codex,
                mode,
                InteractionMode::Plan,
            )
            .unwrap();
            assert_eq!(value["start"]["approvalPolicy"], p.codex_approval);
            assert_eq!(value["turn"]["approvalsReviewer"], p.codex_reviewer);
            assert_eq!(value["turn"]["collaborationMode"]["mode"], "plan");
            assert_eq!(
                value["approval"]["result"]["scope"],
                if decision == PermissionDecision::AcceptForSession {
                    "session"
                } else {
                    "turn"
                }
            );
            assert_eq!(
                value["approval"]["result"]["permissions"],
                if decision == PermissionDecision::Decline {
                    json!({})
                } else {
                    json!({"network":{"enabled":true}})
                }
            );
            assert_eq!(
                value["question"]["result"]["answers"]["q"]["answers"],
                json!(["B"])
            );
        }
    }
}

#[tokio::test]
async fn acp_native_mode_and_permission_question_roundtrips() {
    for harness in [
        AcpHarness::grok(),
        AcpHarness::antigravity(),
        AcpHarness::hermes(),
        AcpHarness::devin(),
    ] {
        let harness = harness.with_executable(fixture());
        for decision in [
            PermissionDecision::Accept,
            PermissionDecision::AcceptForSession,
            PermissionDecision::Decline,
            PermissionDecision::Cancel,
        ] {
            let value = roundtrip(&harness, RuntimeMode::ApprovalRequired, decision, false).await;
            let outcome = &value["approval"]["result"]["outcome"];
            if decision == PermissionDecision::Cancel {
                assert_eq!(outcome["outcome"], "cancelled");
            } else {
                assert_eq!(
                    outcome["optionId"],
                    if decision == PermissionDecision::Decline {
                        "no"
                    } else {
                        "once"
                    }
                );
            }
            assert_eq!(value["question"]["result"]["outcome"]["optionId"], "b");
            assert_eq!(
                value["unknownApproval"]["result"]["outcome"]["outcome"],
                "cancelled"
            );
            if harness.id() == zeron_proto::HarnessId::Grok {
                assert_eq!(value["init"]["_meta"]["clientType"], "extension");
                assert!(
                    value["args"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("default"))
                );
                for alias in value["aliases"].as_array().unwrap() {
                    assert_eq!(
                        alias["result"],
                        json!({"outcome":"accepted","answers":{"Pick":["B"]}})
                    );
                }
            } else {
                assert_eq!(value["mode"], "default");
            }
        }
    }
}

#[tokio::test]
async fn acp_full_access_still_refuses_unknown_permission_kinds() {
    let value = roundtrip(
        &AcpHarness::hermes().with_executable(fixture()),
        RuntimeMode::FullAccess,
        PermissionDecision::Accept,
        false,
    )
    .await;
    assert_eq!(
        value["unknownApproval"]["result"]["outcome"]["outcome"],
        "cancelled"
    );
    assert_eq!(value["question"]["result"]["outcome"]["optionId"], "b");
}

#[tokio::test]
async fn pi_restricted_modes_refuse_before_launch() {
    for mode in [
        RuntimeMode::ApprovalRequired,
        RuntimeMode::AutoAcceptEdits,
        RuntimeMode::Auto,
    ] {
        let request: RunRequest = serde_json::from_value(json!({
            "prompt":"policy", "model":null, "reasoning":null, "cwd":"/tmp",
            "sandbox":"workspace-write", "resume":null, "runtimeMode":mode
        }))
        .unwrap();
        let (_, steering) = mpsc::channel(1);
        let result = AcpHarness::pi()
            .with_executable("/not/a/provider")
            .run(
                request,
                RunControls {
                    browser: None,
                    computer_use_socket: None,
                    steering,
                    interrupt: CancellationToken::new(),
                    request_permission: zeron_harness::refuse_permissions(),
                    request_input: Box::new(|_| oneshot::channel().1),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(zeron_harness::HarnessError::Protocol(_))
        ));
    }
}
