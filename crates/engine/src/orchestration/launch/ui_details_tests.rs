use super::{Rig, json};

#[tokio::test]
async fn desktop_handoff_shares_binding_continuation_and_duplicate_refusal() {
    let rig = Rig::new().await;
    let request = serde_json::from_value(json!({
        "chatId":"parent", "input":{
            "branch":"feature/desktop", "startFromOrigin":false,
            "runSetupScript":false, "continuationPrompt":"Continue desktop work"
        }
    }))
    .unwrap();
    let reply = crate::orchestration::ui_details::handoff(&rig.service, request)
        .await
        .unwrap();
    let zeron_rpc::RpcReply::Value(value) = reply else {
        panic!("unary handoff")
    };
    assert_eq!(value["branch"], "feature/desktop");
    assert_eq!(value["continuation"]["status"], "scheduled");
    let projection = rig
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    assert_eq!(
        projection.thread.worktree_path.as_deref(),
        value["worktreePath"].as_str()
    );
    assert!(
        crate::orchestration::task::records(&projection, "message")
            .iter()
            .any(|m| m["text"] == "Continue desktop work")
    );
    let request =
        serde_json::from_value(json!({"chatId":"parent","input":{"branch":"second"}})).unwrap();
    let error = crate::orchestration::ui_details::handoff(&rig.service, request)
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("already attached to worktree"));
}
