use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use zeron_proto as crate_types;

// The generated dispatcher refers to crate::orchestration, etc.
use crate_types::{orchestration, orchestration_mcp, provider_instance};
include!("t3_oracle/round_trips.rs");

fn round_trip<T: Serialize + DeserializeOwned>(value: &Value) {
    let decoded: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{}: {e}; input={value}", std::any::type_name::<T>()));
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert_eq!(encoded, *value, "{}", std::any::type_name::<T>());
    let again: T = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(again).unwrap(), encoded);
}

fn tools() -> Vec<Value> {
    serde_json::from_str(include_str!("t3_oracle/fixtures/tools.json")).unwrap()
}

fn digest(value: &Value) -> String {
    // serde_json's default map is ordered, matching the extractor's canonical JSON.
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}

fn tool_diff(actual: &[Value]) -> Vec<String> {
    let expected: BTreeMap<_, _> = orchestration_mcp::TOOL_CONTRACT_DIGESTS
        .iter()
        .copied()
        .collect();
    let found: BTreeMap<_, _> = actual
        .iter()
        .map(|v| (v["name"].as_str().unwrap(), digest(v)))
        .collect();
    expected
        .keys()
        .chain(found.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|name| found.get(name).map(String::as_str) != expected.get(name).copied())
        .map(str::to_owned)
        .collect()
}

#[test]
fn exact_inventory_is_72_with_52_core_and_20_later() {
    let tools = tools();
    assert_eq!(tools.len(), 72);
    assert_eq!(tools.iter().filter(|t| t["phase"] == "core").count(), 52);
    assert_eq!(tools.iter().filter(|t| t["phase"] == "later").count(), 20);
    let names: BTreeSet<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names.len(), 72);
    assert!(!names.contains("preview_screenshot"));
    for tool in tools {
        assert!(tool["description"].as_str().is_some());
        assert!(tool["inputSchema"].is_object());
        assert!(tool["resultSchema"].is_object());
        assert!(tool["errorSchema"].is_object());
        assert!(tool["rust"]["input"].as_str().is_some());
        assert!(tool["rust"]["result"].as_str().is_some());
        assert!(tool["rust"]["error"].as_str().is_some());
    }
}

#[test]
fn error_tags_include_declared_failures_and_framework_failure_families() {
    let tools = tools();
    let delegate = tools.iter().find(|t| t["name"] == "delegate_task").unwrap();
    assert!(
        delegate["declaredErrorTags"]
            .as_array()
            .unwrap()
            .contains(&json!("OrchestratorMcpFailure"))
    );
    for tag in ["AiError", "execution-denied", "execution-interrupted"] {
        assert!(
            delegate["errorTags"]
                .as_array()
                .unwrap()
                .contains(&json!(tag)),
            "{tag}"
        );
    }
    assert_eq!(delegate["refusalCodes"].as_array().unwrap().len(), 14);
    let pr = tools
        .iter()
        .find(|t| t["name"] == "watch_pull_request")
        .unwrap();
    assert!(
        pr["declaredErrorTags"]
            .as_array()
            .unwrap()
            .contains(&json!("PullRequestNotOpenError"))
    );
    let snapshot = tools
        .iter()
        .find(|t| t["name"] == "preview_snapshot")
        .unwrap();
    assert_eq!(
        snapshot["transport"]["fallbackErrorTag"],
        "PreviewSnapshotError"
    );
}

#[test]
fn source_tag_commit_and_copied_contract_sources_are_pinned() {
    let provenance: Value =
        serde_json::from_str(include_str!("t3_oracle/fixtures/provenance.json")).unwrap();
    assert_eq!(provenance["tag"], "v0.0.46-nightly.20261003.2632");
    assert_eq!(
        provenance["commit"],
        "f391794a35c604d57e166a3ab48d56fc6e4e469a"
    );
    assert_eq!(provenance["license"], "MIT");
    for (path, bytes) in [
        (
            "packages/contracts/src/orchestrationV2.ts",
            include_bytes!("t3_oracle/fixtures/orchestrationV2.ts").as_slice(),
        ),
        (
            "packages/contracts/src/orchestratorMcp.ts",
            include_bytes!("t3_oracle/fixtures/orchestratorMcp.ts").as_slice(),
        ),
        (
            "apps/server/src/mcp/McpHttpServer.ts",
            include_bytes!("t3_oracle/fixtures/mcp-framing.ts").as_slice(),
        ),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            provenance["sources"][path],
            "{path}"
        );
    }
}

#[test]
fn refusal_branches_and_upstream_replay_seeds_keep_source_provenance() {
    let provenance: Value =
        serde_json::from_str(include_str!("t3_oracle/fixtures/provenance.json")).unwrap();
    for corpus in [
        include_str!("t3_oracle/fixtures/refusal-sources.json"),
        include_str!("t3_oracle/fixtures/scenario-sources.json"),
    ] {
        let entries: Vec<Value> = serde_json::from_str(corpus).unwrap();
        assert!(entries.len() >= 14);
        for entry in entries {
            let path = entry["source"].as_str().unwrap();
            let hash = format!(
                "{:x}",
                Sha256::digest(entry["text"].as_str().unwrap().as_bytes())
            );
            assert_eq!(hash, entry["sha256"], "{path}");
            assert_eq!(hash, provenance["sources"][path], "{path}");
        }
    }
}

#[test]
fn nonfinite_json_numbers_are_named_strings_not_null() {
    for value in [
        json!(1.5),
        json!("Infinity"),
        json!("-Infinity"),
        json!("NaN"),
    ] {
        round_trip::<orchestration::JsonNumber>(&value);
    }
    for (number, encoded) in [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
    ] {
        assert_eq!(
            serde_json::to_value(orchestration::JsonNumber::Finite(number)).unwrap(),
            json!(encoded)
        );
    }
}

#[test]
fn schema_diff_locks_fields_defaults_annotations_and_error_tags() {
    assert_eq!(tool_diff(&tools()), Vec::<String>::new());
    let contracts: Value =
        serde_json::from_str(include_str!("t3_oracle/fixtures/contracts.json")).unwrap();
    let actual = contracts["definitions"].as_object().unwrap();
    assert_eq!(
        actual.len(),
        orchestration_mcp::DOMAIN_CONTRACT_DIGESTS.len()
    );
    for (name, expected) in orchestration_mcp::DOMAIN_CONTRACT_DIGESTS {
        assert_eq!(&digest(&actual[*name]), expected, "{name}");
    }
}

#[test]
fn generated_rust_is_the_reviewed_translation_not_a_stale_schema_copy() {
    let hashes: BTreeMap<String, String> =
        serde_json::from_str(include_str!("t3_oracle/fixtures/generated-artifacts.json")).unwrap();
    for (path, bytes) in [
        (
            "crates/proto/src/orchestration.rs",
            include_bytes!("../src/orchestration.rs").as_slice(),
        ),
        (
            "crates/proto/src/orchestration_mcp.rs",
            include_bytes!("../src/orchestration_mcp.rs").as_slice(),
        ),
        (
            "crates/proto/src/provider_instance.rs",
            include_bytes!("../src/provider_instance.rs").as_slice(),
        ),
        (
            "crates/proto/tests/t3_oracle/round_trips.rs",
            include_bytes!("t3_oracle/round_trips.rs").as_slice(),
        ),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            hashes[path],
            "{path}: regenerate/review the translation"
        );
    }
    assert_eq!(orchestration_mcp::pinned_tool_inventory().len(), 72);
}

#[test]
fn schema_diff_detects_every_requested_drift_category() {
    let original = tools();
    let mut removed = original.clone();
    removed.pop();
    assert!(!tool_diff(&removed).is_empty());
    let mut added = original.clone();
    let mut extra = original[0].clone();
    extra["name"] = json!("invented_tool");
    added.push(extra);
    assert!(!tool_diff(&added).is_empty());
    for path in [
        vec!["inputSchema", "properties", "newField"],
        vec!["inputSchema", "default"],
        vec!["resultSchema", "newField"],
        vec!["errorSchema", "newTag"],
        vec!["annotations", "readOnlyHint"],
        vec!["refusalCodes"],
    ] {
        let mut changed = original.clone();
        let mut field = &mut changed[0];
        for key in path {
            if !field.is_object() {
                *field = json!({});
            }
            field = &mut field[key];
        }
        *field = json!("intentional-drift");
        assert!(!tool_diff(&changed).is_empty());
    }
}

#[test]
fn all_extracted_contracts_and_tool_variants_round_trip() {
    let cases: BTreeMap<String, Vec<Value>> =
        serde_json::from_str(include_str!("t3_oracle/fixtures/serde-cases.json")).unwrap();
    assert_eq!(
        cases.len(),
        orchestration_mcp::DOMAIN_CONTRACT_DIGESTS.len()
    );
    let mut failures = Vec::new();
    for (name, values) in cases {
        for value in values {
            if std::panic::catch_unwind(|| check_case(&name, &value)).is_err() {
                failures.push(name.clone());
            }
        }
    }
    assert!(
        failures.is_empty(),
        "oracle round-trip failures: {failures:?}"
    );
}

#[test]
fn omitted_null_and_value_are_distinct_and_required_null_is_required() {
    use orchestration_mcp::T3QueueReorderInput;
    for value in [
        json!({"queuedRunId":"queued","beforeRunId":null}),
        json!({"threadId":"parent","queuedRunId":"queued","beforeRunId":"other"}),
    ] {
        round_trip::<T3QueueReorderInput>(&value);
    }
    assert!(
        serde_json::from_value::<T3QueueReorderInput>(json!({"queuedRunId":"queued"})).is_err()
    );
    assert!(
        serde_json::from_value::<T3QueueReorderInput>(
            json!({"threadId":null,"queuedRunId":"queued","beforeRunId":null})
        )
        .is_err()
    );
    for value in [
        json!({"projectId":"p"}),
        json!({"projectId":"p","faviconPath":null}),
        json!({"projectId":"p","faviconPath":"icon.png"}),
    ] {
        round_trip::<orchestration_mcp::T3ProjectUpdateInput>(&value);
    }
}

#[test]
fn modes_use_exact_t3_tags_and_ids_are_not_uuids() {
    for mode in [
        "approval-required",
        "auto-accept-edits",
        "auto",
        "full-access",
    ] {
        round_trip::<orchestration::RuntimeMode>(&json!(mode));
    }
    for mode in ["default", "plan"] {
        round_trip::<orchestration::ProviderInteractionMode>(&json!(mode));
    }
    round_trip::<orchestration::NodeId>(&json!("node:delegated-task:command%3Amcp%3Aexample"));
    round_trip::<provider_instance::ProviderDriverKind>(&json!("futureDriver"));
}

#[test]
fn orchestration_and_runtime_policy_share_authoritative_types() {
    for policy in zeron_proto::RuntimeMode::ALL {
        let contract: orchestration::RuntimeMode = policy;
        assert_eq!(contract, policy);
        assert!(contract.permits(zeron_proto::RuntimeMode::ApprovalRequired));
    }
    let plan: orchestration::ProviderInteractionMode = zeron_proto::InteractionMode::Plan;
    let alias: orchestration::InteractionMode = plan;
    assert!(!alias.permits(zeron_proto::InteractionMode::Default));
    let decision: orchestration::ProviderApprovalDecision =
        zeron_proto::PermissionDecision::AcceptForSession;
    assert_eq!(serde_json::to_value(decision).unwrap(), "acceptForSession");
}

#[test]
fn upstream_provider_environment_defaults_are_not_invented() {
    let decoded: provider_instance::ProviderInstanceEnvironmentVariable =
        serde_json::from_value(json!({"name":"EXAMPLE"})).unwrap();
    assert_eq!(
        serde_json::to_value(decoded).unwrap(),
        json!({"name":"EXAMPLE","value":"","sensitive":false})
    );
}

fn compare_codec<T: Serialize + DeserializeOwned>(case: &Value) {
    let decoded = serde_json::from_value::<T>(case["input"].clone());
    if case["accepted"] == false {
        assert!(
            decoded.is_err(),
            "{} unexpectedly accepted {}",
            case["name"],
            case["input"]
        );
    } else {
        let value = decoded.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            case["encoded"],
            "{}",
            case["name"]
        );
    }
}

#[test]
fn real_pinned_effect_codec_traces_match_rust_compatibility_codecs() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("t3_oracle/fixtures/codec-cases.json")).unwrap();
    for case in cases {
        match case["name"].as_str().unwrap() {
            "ModelSelection" => compare_codec::<provider_instance::ModelSelection>(&case),
            "ProviderOptionSelections" => {
                compare_codec::<provider_instance::ProviderOptionSelections>(&case)
            }
            "ProviderInstanceEnvironmentVariable" => {
                compare_codec::<provider_instance::ProviderInstanceEnvironmentVariable>(&case)
            }
            "OrchestratorMcpTargetOptions" => {
                compare_codec::<orchestration_mcp::OrchestratorMcpTargetOptions>(&case)
            }
            "OrchestratorMcpScheduleTaskInput" => {
                compare_codec::<orchestration_mcp::OrchestratorMcpScheduleTaskInput>(&case)
            }
            "ScheduledTaskSchedule" => compare_codec::<orchestration::ScheduledTaskSchedule>(&case),
            "ScheduledTaskUpsertSchedule" => {
                compare_codec::<orchestration::ScheduledTaskUpsertSchedule>(&case)
            }
            "OrchestrationV2NotificationSource" => {
                compare_codec::<orchestration::OrchestrationV2NotificationSource>(&case)
            }
            "OrchestrationV2PendingBackgroundTask" => {
                compare_codec::<orchestration::OrchestrationV2PendingBackgroundTask>(&case)
            }
            other => panic!("unmapped codec trace: {other}"),
        }
    }
}
