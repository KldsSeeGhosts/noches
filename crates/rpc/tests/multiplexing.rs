use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use zeron_rpc::{RpcClient, RpcError, RpcReply, RpcService};

struct Service(Arc<AtomicUsize>);
struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
#[async_trait]
impl RpcService for Service {
    async fn handle(&self, method: &str, params: Value) -> Result<RpcReply, RpcError> {
        match method {
            "Flood" => Ok(RpcReply::Stream(
                futures::stream::iter((0..1000).map(|i| json!(i))).boxed(),
            )),
            "Echo" => Ok(RpcReply::Value(params)),
            "Wait" => {
                self.0.fetch_add(1, Ordering::SeqCst);
                let _active = Active(self.0.clone());
                std::future::pending::<()>().await;
                unreachable!()
            }
            _ => Err(RpcError::UnknownMethod(method.into())),
        }
    }
}

async fn wait_for(active: &AtomicUsize, count: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while active.load(Ordering::SeqCst) != count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("request lifecycle did not settle");
}

#[tokio::test]
async fn stalled_stream_does_not_block_unary_and_ends_instead_of_losing_deltas() {
    let client = zeron_rpc::memory_client(Arc::new(Service(Arc::default())));
    let mut stream = client.subscribe_scoped("Flood", Value::Null).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), client.call("Echo", json!(42)))
            .await
            .unwrap()
            .unwrap(),
        json!(42)
    );
    let mut values = Vec::new();
    while let Some(value) = stream.recv().await {
        values.push(value);
    }
    assert_eq!(values, (0..256).map(|i| json!(i)).collect::<Vec<_>>());
}

#[tokio::test]
async fn eof_rejects_new_calls_even_when_outbound_is_retained() {
    let (out, _requests) = tokio::sync::mpsc::channel(16);
    let (input, inbound) = tokio::sync::mpsc::channel(16);
    let client = RpcClient::new(out, inbound);
    drop(input);
    tokio::task::yield_now().await;
    for _ in 0..20 {
        let result = tokio::time::timeout(Duration::from_secs(1), client.call("Echo", Value::Null))
            .await
            .expect("closed request hung");
        assert!(matches!(result, Err(RpcError::Closed)));
    }
}

#[tokio::test]
async fn cancelling_a_unary_call_cancels_owned_server_work() {
    let active = Arc::new(AtomicUsize::new(0));
    let client = Arc::new(zeron_rpc::memory_client(Arc::new(Service(active.clone()))));
    let call_client = client.clone();
    let call = tokio::spawn(async move { call_client.call("Wait", Value::Null).await });
    wait_for(&active, 1).await;
    call.abort();
    let _ = call.await;
    wait_for(&active, 0).await;
    assert_eq!(
        client.call("Echo", json!("alive")).await.unwrap(),
        json!("alive")
    );
}

#[tokio::test]
async fn duplicate_live_ids_retire_all_requests_without_orphans() {
    let active = Arc::new(AtomicUsize::new(0));
    let (input, inbound) = tokio::sync::mpsc::channel(16);
    let (output, _replies) = tokio::sync::mpsc::channel(16);
    let server = tokio::spawn(zeron_rpc::serve_connection(
        Arc::new(Service(active.clone())),
        output,
        inbound,
    ));
    let request = json!({"id":7,"method":"Wait","params":{}}).to_string();
    input.send(request.clone()).await.unwrap();
    wait_for(&active, 1).await;
    input.send(request).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
    wait_for(&active, 0).await;
}
