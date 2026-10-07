//! T3-native policy compilation. This is not a claim of an OS sandbox.
//! Unsupported combinations fail before spawning or sending a prompt.
use crate::HarnessError;
use serde_json::{Value, json};
use zeron_proto::{HarnessId, InteractionMode, RuntimeMode, SandboxLevel};

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledPolicy {
    pub runtime: RuntimeMode,
    pub interaction: InteractionMode,
    pub claude_permission_mode: &'static str,
    pub codex_approval: &'static str,
    pub codex_reviewer: &'static str,
    pub codex_sandbox: SandboxLevel,
    pub cursor_auto_review: bool,
    pub cursor_sandbox: bool,
}

pub fn compile(
    harness: HarnessId,
    runtime: RuntimeMode,
    interaction: InteractionMode,
) -> Result<CompiledPolicy, HarnessError> {
    let unsupported = |detail: &str| {
        HarnessError::Protocol(format!(
            "{harness:?} cannot enforce {runtime:?}/{interaction:?}: {detail}"
        ))
    };
    // Pi enforces Supervised and Auto-accept through its blocking `tool_call`
    // extension hook (pi/noches-policy.ts), verified against the live RPC
    // protocol. It has no native Auto classifier and no plan gate.
    if harness == HarnessId::Pi && runtime == RuntimeMode::Auto {
        return Err(unsupported("Pi has no native Auto classifier"));
    }
    if harness == HarnessId::Pi && interaction != InteractionMode::Default {
        return Err(unsupported("Pi has no native plan gate"));
    }
    if harness == HarnessId::Grok && runtime == RuntimeMode::AutoAcceptEdits {
        return Err(unsupported(
            "Grok supports Supervised, Auto, and Full access only",
        ));
    }
    if matches!(harness, HarnessId::Devin | HarnessId::Hermes) && runtime == RuntimeMode::Auto {
        return Err(unsupported("generic ACP has no native Auto classifier"));
    }
    if interaction == InteractionMode::Plan
        && matches!(
            harness,
            HarnessId::Grok
                | HarnessId::Devin
                | HarnessId::Hermes
                | HarnessId::Antigravity
                | HarnessId::Opencode
        )
    {
        return Err(unsupported(
            "this transport does not yet enforce the native plan gate",
        ));
    }
    let (approval, reviewer, sandbox) = match runtime {
        RuntimeMode::ApprovalRequired => ("untrusted", "user", SandboxLevel::ReadOnly),
        RuntimeMode::AutoAcceptEdits => ("on-request", "user", SandboxLevel::WorkspaceWrite),
        RuntimeMode::Auto => ("on-request", "auto_review", SandboxLevel::WorkspaceWrite),
        RuntimeMode::FullAccess => ("never", "user", SandboxLevel::DangerFullAccess),
    };
    Ok(CompiledPolicy {
        runtime,
        interaction,
        claude_permission_mode: if interaction == InteractionMode::Plan {
            "plan"
        } else {
            match runtime {
                RuntimeMode::ApprovalRequired => "default",
                RuntimeMode::AutoAcceptEdits => "acceptEdits",
                RuntimeMode::Auto => "auto",
                RuntimeMode::FullAccess => "bypassPermissions",
            }
        },
        codex_approval: approval,
        codex_reviewer: reviewer,
        codex_sandbox: sandbox,
        cursor_auto_review: runtime == RuntimeMode::ApprovalRequired,
        cursor_sandbox: runtime != RuntimeMode::FullAccess,
    })
}

pub fn grok_args(runtime: RuntimeMode) -> Vec<String> {
    let args: &[&str] = match runtime {
        RuntimeMode::FullAccess => &[
            "--no-auto-update",
            "agent",
            "--no-leader",
            "--always-approve",
            "stdio",
        ],
        RuntimeMode::Auto => &[
            "--no-auto-update",
            "--permission-mode",
            "auto",
            "agent",
            "--no-leader",
            "stdio",
        ],
        _ => &[
            "--no-auto-update",
            "--permission-mode",
            "default",
            "agent",
            "--no-leader",
            "stdio",
        ],
    };
    args.iter().map(|s| (*s).into()).collect()
}

/// Policy-critical config selection, not best-effort model-option matching.
/// Never leave a configured bypass enabled for a supervised run.
pub fn acp_mode_set(
    harness: HarnessId,
    runtime: RuntimeMode,
    session: &Value,
) -> Result<Option<(String, Value)>, HarnessError> {
    if harness == HarnessId::Grok {
        return Ok(None);
    }
    let candidates: &[&str] = match (harness, runtime) {
        (HarnessId::Antigravity, RuntimeMode::FullAccess) => &["yolo"],
        (HarnessId::Antigravity, RuntimeMode::AutoAcceptEdits) => &["auto_edit"],
        (HarnessId::Antigravity, _) => &["default"],
        (_, RuntimeMode::FullAccess) => &[
            "bypassPermissions",
            "bypass_permissions",
            "bypass",
            "yolo",
            "agent-full-access",
            "danger-full-access",
            "full-access",
        ],
        (_, RuntimeMode::AutoAcceptEdits) => &["acceptEdits", "auto_edit", "auto-accept-edits"],
        _ => &["default", "ask", "approval-required"],
    };
    let mode = session
        .get("configOptions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|o| o["category"] == "mode" && o["type"] == "select");
    if let Some(native) = candidates.iter().find(|candidate| {
        session
            .pointer("/modes/availableModes")
            .and_then(Value::as_array)
            .is_some_and(|modes| {
                modes
                    .iter()
                    .any(|mode| mode["id"].as_str() == Some(**candidate))
            })
    }) {
        return Ok(Some(("session/set_mode".into(), json!({"modeId":native}))));
    }
    if let Some(mode) = mode {
        let selected = candidates.iter().find(|candidate| {
            mode["options"].as_array().is_some_and(|options| {
                options
                    .iter()
                    .any(|o| o["value"].as_str() == Some(**candidate))
            })
        });
        if let (Some(id), Some(selected)) = (mode["id"].as_str(), selected) {
            return Ok(Some((
                "session/set_config_option".into(),
                json!({"configId": id,"value": selected}),
            )));
        }
    }
    // Legacy unrestricted generic ACP may have no selectable mode at all.
    // This is a request boundary, not an OS sandbox guarantee.
    if mode.is_none() && runtime == RuntimeMode::FullAccess && harness != HarnessId::Antigravity {
        return Ok(None);
    }
    Err(HarnessError::Protocol(format!(
        "{harness:?} did not advertise an enforceable {runtime:?} permission mode"
    )))
}

/// Exact T3 defaults without the optional explicit sandbox overrides.
pub fn opencode_rules(runtime: RuntimeMode, v2: bool) -> Value {
    let rule = |name: &str, action: &str| {
        if v2 {
            json!({"action":name,"resource":"*","effect":action})
        } else {
            json!({"permission":name,"pattern":"*","action":action})
        }
    };
    if runtime == RuntimeMode::FullAccess {
        return json!([rule("*", "allow")]);
    }
    if v2 {
        return json!([
            rule("shell", "ask"),
            rule(
                "edit",
                if runtime == RuntimeMode::AutoAcceptEdits {
                    "allow"
                } else {
                    "ask"
                }
            ),
            rule("external_directory", "ask")
        ]);
    }
    let restricted = [
        "bash",
        "edit",
        "webfetch",
        "websearch",
        "codesearch",
        "external_directory",
        "doom_loop",
    ];
    let mut rules = vec![rule("*", "deny")];
    rules.extend(restricted.iter().map(|s| rule(s, "deny")));
    rules.push(rule("*", "ask"));
    rules.extend(restricted.iter().map(|s| rule(s, "ask")));
    rules.extend(
        [
            "question",
            "read",
            "glob",
            "grep",
            "lsp",
            "todowrite",
            "task",
            "skill",
        ]
        .iter()
        .map(|s| rule(s, "allow")),
    );
    for (pattern, action) in [
        ("*.env", "ask"),
        ("*.env.*", "ask"),
        ("*.env.example", "allow"),
    ] {
        rules.push(json!({"permission":"read","pattern":pattern,"action":action}));
    }
    if runtime == RuntimeMode::AutoAcceptEdits {
        rules.push(rule("edit", "allow"));
    }
    Value::Array(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_policy_fixtures() {
        for harness in [
            HarnessId::ClaudeCode,
            HarnessId::Codex,
            HarnessId::Cursor,
            HarnessId::Opencode,
            HarnessId::Antigravity,
        ] {
            for mode in RuntimeMode::ALL {
                assert!(compile(harness, mode, InteractionMode::Default).is_ok());
            }
        }
        for harness in [
            HarnessId::Pi,
            HarnessId::Grok,
            HarnessId::Devin,
            HarnessId::Hermes,
        ] {
            assert!(compile(harness, RuntimeMode::FullAccess, InteractionMode::Default).is_ok());
            assert!(compile(harness, RuntimeMode::FullAccess, InteractionMode::Plan).is_err());
        }
        assert!(
            compile(
                HarnessId::Pi,
                RuntimeMode::ApprovalRequired,
                InteractionMode::Default
            )
            .is_ok()
        );
        assert!(
            compile(
                HarnessId::Grok,
                RuntimeMode::AutoAcceptEdits,
                InteractionMode::Default
            )
            .is_err()
        );
        let auto = compile(
            HarnessId::Codex,
            RuntimeMode::Auto,
            InteractionMode::Default,
        )
        .unwrap();
        assert_eq!(
            (auto.codex_approval, auto.codex_reviewer, auto.codex_sandbox),
            ("on-request", "auto_review", SandboxLevel::WorkspaceWrite)
        );
        assert_eq!(
            compile(
                HarnessId::ClaudeCode,
                RuntimeMode::Auto,
                InteractionMode::Plan
            )
            .unwrap()
            .claude_permission_mode,
            "plan"
        );
        assert!(
            acp_mode_set(HarnessId::Hermes, RuntimeMode::ApprovalRequired, &json!({})).is_err()
        );
        assert!(acp_mode_set(HarnessId::Antigravity, RuntimeMode::Auto, &json!({"configOptions":[{"id":"permission","type":"select","category":"mode","options":[{"value":"yolo"}]}]})).is_err());
        assert_eq!(
            opencode_rules(RuntimeMode::FullAccess, true),
            json!([{"action":"*","resource":"*","effect":"allow"}])
        );
        assert!(grok_args(RuntimeMode::ApprovalRequired).contains(&"default".into()));
        assert!(grok_args(RuntimeMode::FullAccess).contains(&"--no-leader".into()));
    }

    #[test]
    fn claude_cli_policy_fixture() {
        for (mode, expected) in RuntimeMode::ALL.into_iter().zip([
            "default",
            "acceptEdits",
            "auto",
            "bypassPermissions",
        ]) {
            assert_eq!(
                compile(HarnessId::ClaudeCode, mode, InteractionMode::Default)
                    .unwrap()
                    .claude_permission_mode,
                expected
            );
            assert_eq!(
                compile(HarnessId::ClaudeCode, mode, InteractionMode::Plan)
                    .unwrap()
                    .claude_permission_mode,
                "plan"
            );
        }
    }
    #[test]
    fn codex_app_server_policy_fixture() {
        let expected = [
            ("untrusted", "user", SandboxLevel::ReadOnly),
            ("on-request", "user", SandboxLevel::WorkspaceWrite),
            ("on-request", "auto_review", SandboxLevel::WorkspaceWrite),
            ("never", "user", SandboxLevel::DangerFullAccess),
        ];
        for (mode, expected) in RuntimeMode::ALL.into_iter().zip(expected) {
            let p = compile(HarnessId::Codex, mode, InteractionMode::Default).unwrap();
            assert_eq!(
                (p.codex_approval, p.codex_reviewer, p.codex_sandbox),
                expected
            );
        }
    }
    #[test]
    fn cursor_sdk_policy_fixture() {
        for (mode, (review, sandbox)) in RuntimeMode::ALL.into_iter().zip([
            (true, true),
            (false, true),
            (false, true),
            (false, false),
        ]) {
            let p = compile(HarnessId::Cursor, mode, InteractionMode::Plan).unwrap();
            assert_eq!((p.cursor_auto_review, p.cursor_sandbox), (review, sandbox));
        }
    }
    #[test]
    fn acp_antigravity_policy_fixture() {
        let session =
            json!({"modes":{"availableModes":[{"id":"default"},{"id":"auto_edit"},{"id":"yolo"}]}});
        for (mode, expected) in
            RuntimeMode::ALL
                .into_iter()
                .zip(["default", "auto_edit", "default", "yolo"])
        {
            assert_eq!(
                acp_mode_set(HarnessId::Antigravity, mode, &session).unwrap(),
                Some(("session/set_mode".into(), json!({"modeId":expected})))
            );
        }
        assert!(acp_mode_set(HarnessId::Antigravity, RuntimeMode::FullAccess, &json!({})).is_err());
    }
    #[test]
    fn acp_grok_and_generic_policy_fixtures() {
        assert_eq!(
            grok_args(RuntimeMode::Auto),
            [
                "--no-auto-update",
                "--permission-mode",
                "auto",
                "agent",
                "--no-leader",
                "stdio"
            ]
        );
        for h in [HarnessId::Devin, HarnessId::Hermes] {
            let session = json!({"configOptions":[{"id":"mode","type":"select","category":"mode","currentValue":"bypass","options":[{"value":"default"},{"value":"acceptEdits"},{"value":"bypass"}]}]});
            for (m, value) in [
                (RuntimeMode::ApprovalRequired, "default"),
                (RuntimeMode::AutoAcceptEdits, "acceptEdits"),
                (RuntimeMode::FullAccess, "bypass"),
            ] {
                assert_eq!(
                    acp_mode_set(h, m, &session).unwrap(),
                    Some((
                        "session/set_config_option".into(),
                        json!({"configId":"mode","value":value})
                    ))
                );
            }
            assert!(compile(h, RuntimeMode::Auto, InteractionMode::Default).is_err());
        }
    }
    #[test]
    fn opencode_native_rules_fixture() {
        assert_eq!(
            opencode_rules(RuntimeMode::AutoAcceptEdits, true),
            json!([
                {"action":"shell","resource":"*","effect":"ask"},
                {"action":"edit","resource":"*","effect":"allow"},
                {"action":"external_directory","resource":"*","effect":"ask"}
            ])
        );
        let v1 = opencode_rules(RuntimeMode::ApprovalRequired, false);
        assert_eq!(
            v1[0],
            json!({"permission":"*","pattern":"*","action":"deny"})
        );
        assert!(
            v1.as_array()
                .unwrap()
                .iter()
                .any(|r| r["permission"] == "bash" && r["action"] == "ask")
        );
        assert!(
            !v1.as_array()
                .unwrap()
                .iter()
                .any(|r| r["permission"] == "edit" && r["action"] == "allow")
        );
        assert!(
            compile(
                HarnessId::Opencode,
                RuntimeMode::FullAccess,
                InteractionMode::Plan
            )
            .is_err()
        );
    }
    #[test]
    fn native_pi_enforces_supervised_and_auto_accept_but_invents_no_classifier_or_plan_gate() {
        for mode in RuntimeMode::ALL {
            assert_eq!(
                compile(HarnessId::Pi, mode, InteractionMode::Default).is_ok(),
                mode != RuntimeMode::Auto,
                "{mode:?}"
            );
            assert!(compile(HarnessId::Pi, mode, InteractionMode::Plan).is_err());
        }
    }
}
