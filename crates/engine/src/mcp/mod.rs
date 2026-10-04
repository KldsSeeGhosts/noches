//! Engine-owned `t3-code`, localhost streamable HTTP (2025-06-18).
//! Authentication precedes all protocol processing, including discovery.
pub mod auth;
pub mod codec;
mod service_stub;
#[cfg(test)]
mod tests;
pub mod toolkit;

use std::convert::Infallible;
use std::sync::{Arc, Weak};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;
use zeron_harness::mcp::{McpServerEntry, SessionMcpContext};

use crate::HarnessRegistry;
use crate::orchestration::service::OrchestratorService;
use auth::{CredentialRegistry, InvocationScope};
use toolkit::Toolkit;

pub const SERVER_NAME: &str = "t3-code";
pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const INSTRUCTIONS: &str = include_str!("instructions.md");

pub struct McpServer {
    pub credentials: Arc<CredentialRegistry>,
    pub toolkit: Arc<Toolkit>,
    endpoint: OnceCell<String>,
    shutdown: CancellationToken,
}

impl McpServer {
    pub fn new(registry: Arc<HarnessRegistry>) -> Self {
        Self {
            credentials: Arc::default(),
            toolkit: Arc::new(Toolkit::new(registry)),
            endpoint: OnceCell::new(),
            shutdown: CancellationToken::new(),
        }
    }

    pub fn set_service(&self, service: Arc<dyn OrchestratorService>) {
        self.toolkit.set_service(service);
    }

    pub async fn endpoint(self: &Arc<Self>) -> std::io::Result<&str> {
        self.endpoint
            .get_or_try_init(|| async {
                let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
                let endpoint = format!("http://127.0.0.1:{}/mcp", listener.local_addr()?.port());
                let weak = Arc::downgrade(self);
                let shutdown = self.shutdown.clone();
                tokio::spawn(serve(listener, weak, shutdown));
                Ok::<_, std::io::Error>(endpoint)
            })
            .await
            .map(String::as_str)
    }

    /// Uses the foundation's registration seam and compare-owned revoker.
    /// A failed/replaced registration revokes this unique credential only.
    pub async fn register(
        self: &Arc<Self>,
        sessions: &crate::SessionsEngine,
        session: &str,
        scope: InvocationScope,
    ) -> Result<SessionMcpContext, zeron_harness::HarnessError> {
        let endpoint = self.endpoint().await?.to_owned();
        let read_only = scope.caller.interaction_mode == zeron_proto::InteractionMode::Plan
            || scope.caller.runtime_mode == zeron_proto::RuntimeMode::ApprovalRequired;
        let credential = self.credentials.issue(scope)?;
        let mut entry = McpServerEntry::http(
            SERVER_NAME,
            endpoint,
            [("Authorization".into(), credential.authorization)].into(),
        );
        entry.allowed_tools = if read_only {
            self.toolkit
                .tools()
                .iter()
                .filter(|descriptor| descriptor["annotations"]["readOnlyHint"] == true)
                .filter_map(|descriptor| descriptor["name"].as_str())
                .map(|name| format!("mcp__t3-code__{name}"))
                .collect()
        } else {
            vec!["mcp__t3-code__*".into()]
        };
        let context = sessions.register_session_mcp(session, vec![entry], INSTRUCTIONS.into());
        match context {
            Ok(context) => {
                let registry = self.credentials.clone();
                context.on_revoke(move || registry.revoke_session(&credential.session_id));
                Ok(context)
            }
            Err(error) => {
                self.credentials.revoke_session(&credential.session_id);
                Err(error)
            }
        }
    }

    pub fn shutdown(&self) {
        self.credentials.revoke_all();
        self.shutdown.cancel();
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn serve(listener: TcpListener, server: Weak<McpServer>, shutdown: CancellationToken) {
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            accepted = listener.accept() => {
                let Ok((socket, peer)) = accepted else { break };
                if !peer.ip().is_loopback() || connections.len() >= 128 { continue; }
                let server = server.clone();
                let shutdown = shutdown.clone();
                connections.spawn(async move {
                    let service = hyper::service::service_fn(move |request| {
                        let server = server.clone();
                        async move {
                            let response = match server.upgrade() {
                                Some(server) => server.handle_http(request).await,
                                None => response(StatusCode::SERVICE_UNAVAILABLE, json!({})),
                            };
                            Ok::<_, Infallible>(response)
                        }
                    });
                    tokio::select! {
                        _ = shutdown.cancelled() => {},
                        _ = hyper::server::conn::http1::Builder::new()
                            .max_buf_size(32 * 1024)
                            .serve_connection(TokioIo::new(socket), service) => {}
                    }
                });
            }
        }
    }
    connections.abort_all();
}

impl McpServer {
    async fn handle_http(&self, request: Request<Incoming>) -> Response<Full<Bytes>> {
        let authorization = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let Some(scope) = self.credentials.resolve(authorization) else {
            return unauthorized();
        };
        if let Some(session) = request.headers().get("mcp-session-id")
            && session.to_str().ok() != Some(scope.caller.session_id.as_str())
        {
            return unauthorized();
        }
        if request.uri().path() != "/mcp" {
            return response(StatusCode::NOT_FOUND, json!({}));
        }
        if let Some(version) = request.headers().get("mcp-protocol-version")
            && version.to_str().ok() != Some(PROTOCOL_VERSION)
        {
            return response(
                StatusCode::BAD_REQUEST,
                codec::rpc_error(Value::Null, -32600, "Unsupported protocol version"),
            );
        }
        // No web origin (or forwarded host) can mint a caller scope.
        if let Some(origin) = request.headers().get("origin") {
            let allowed = origin
                .to_str()
                .ok()
                .and_then(|v| reqwest::Url::parse(v).ok())
                .is_some_and(|u| matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")));
            if !allowed {
                return response(StatusCode::FORBIDDEN, json!({}));
            }
        }
        match *request.method() {
            hyper::Method::DELETE => return empty(StatusCode::NO_CONTENT),
            hyper::Method::GET => return empty(StatusCode::METHOD_NOT_ALLOWED),
            hyper::Method::POST => {}
            _ => return empty(StatusCode::METHOD_NOT_ALLOWED),
        }
        let body = tokio::time::timeout(
            Duration::from_secs(30),
            Limited::new(request.into_body(), 8 * 1024 * 1024).collect(),
        )
        .await;
        let bytes = match body {
            Ok(Ok(body)) => body.to_bytes(),
            _ => return response(StatusCode::PAYLOAD_TOO_LARGE, json!({})),
        };
        let message: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return response(
                    StatusCode::BAD_REQUEST,
                    codec::rpc_error(Value::Null, -32700, "Parse error"),
                );
            }
        };
        if !message.is_object() {
            return response(
                StatusCode::BAD_REQUEST,
                codec::rpc_error(Value::Null, -32600, "Invalid Request"),
            );
        }
        let initialize = message["method"] == "initialize";
        let session = scope.caller.session_id.clone();
        let Some(result) = self.toolkit.request(scope, message).await else {
            return empty(StatusCode::ACCEPTED);
        };
        let mut response = response(StatusCode::OK, result);
        if initialize {
            response
                .headers_mut()
                .insert("mcp-session-id", session.parse().expect("UUID header"));
        }
        response
    }
}

fn empty(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::new()))
        .expect("response")
}

fn response(status: StatusCode, value: Value) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(value.to_string())))
        .expect("response")
}

fn unauthorized() -> Response<Full<Bytes>> {
    let mut response = response(
        StatusCode::UNAUTHORIZED,
        json!({
            "error":"invalid_mcp_credential",
            "message":"A valid provider-scoped MCP bearer credential is required."
        }),
    );
    response
        .headers_mut()
        .insert("www-authenticate", "Bearer".parse().expect("static header"));
    response
}
