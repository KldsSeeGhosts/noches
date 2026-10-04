use super::*;
use crate::session_lifecycle::{NativeForkRequest, SessionLifecycle};

fn fork_body(protocol: Protocol, before: Option<&str>) -> Value {
    let mut body = json!({});
    if let Some(before) = before {
        body[if protocol == Protocol::V2 {
            "before"
        } else {
            "messageID"
        }] = json!(before);
    }
    body
}

#[async_trait]
impl SessionLifecycle for OpencodeHarness {
    fn can_fork_from_turn(&self) -> bool {
        true
    }
    async fn fork_thread(&self, request: NativeForkRequest) -> Result<String, HarnessError> {
        if request.source_turn_id.is_none() {
            return Err(HarnessError::Protocol(
                "OpenCode fork boundary turn was not found".into(),
            ));
        }
        if !request
            .source_thread_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(HarnessError::Protocol(
                "Invalid OpenCode native session ID".into(),
            ));
        }
        let mut server = self.server(Some(&request.cwd)).await?;
        let result = async {
            // Copying native history while it is changing is not a stable fork.
            if server
                .session_running(&request.source_thread_id, Some(&request.cwd))
                .await?
            {
                return Err(HarnessError::Protocol(format!(
                    "Cannot fork OpenCode thread {} while a turn is active",
                    request.source_thread_id
                )));
            }
            let protocol = server.protocol().await;
            let path = format!(
                "{}/session/{}/fork",
                if protocol == Protocol::V2 { "/api" } else { "" },
                request.source_thread_id
            );
            // No retries: a 5xx or dropped response may follow native acceptance.
            let forked = unwrap_data(
                server
                    .post_json(
                        &path,
                        Some(&request.cwd),
                        &fork_body(protocol, request.source_next_turn_id.as_deref()),
                    )
                    .await?,
            );
            forked["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    HarnessError::Protocol(
                        "OpenCode session.fork returned no native session ID".into(),
                    )
                })
        }
        .await;
        server.shutdown(self.kill_grace).await;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t3_version_specific_before_boundary_shapes() {
        assert_eq!(
            fork_body(Protocol::V1, Some("msg_next")),
            json!({"messageID":"msg_next"})
        );
        assert_eq!(
            fork_body(Protocol::V2, Some("msg_next")),
            json!({"before":"msg_next"})
        );
        assert_eq!(fork_body(Protocol::V2, None), json!({}));
    }
}
