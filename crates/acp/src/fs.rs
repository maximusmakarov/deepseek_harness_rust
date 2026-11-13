use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncWrite, BufWriter};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};

use crate::send;

#[async_trait]
pub trait ClientRpc: Send + Sync {
    async fn request(&self, method: &str, params: Value) -> Result<Value, String>;
}

pub struct AcpRpc<W> {
    pub(crate) writer: Arc<Mutex<BufWriter<W>>>,
    pub(crate) pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    pub(crate) next_id: Arc<AtomicU64>,
}

#[async_trait]
impl<W> ClientRpc for AcpRpc<W>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);
        if send(
            &self.writer,
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        )
        .await
        .is_err()
        {
            self.pending.lock().await.remove(&id);
            return Err("failed to send ACP request".into());
        }
        match timeout(Duration::from_secs(300), receiver).await {
            Ok(Ok(response)) => {
                if let Some(error) = response.get("error") {
                    Err(error["message"]
                        .as_str()
                        .unwrap_or("ACP request failed")
                        .to_owned())
                } else {
                    Ok(response["result"].clone())
                }
            }
            _ => {
                self.pending.lock().await.remove(&id);
                Err("ACP request timed out".into())
            }
        }
    }
}

pub struct AcpFsClient {
    rpc: Mutex<Option<Arc<dyn ClientRpc>>>,
    read: AtomicBool,
    write: AtomicBool,
}

impl AcpFsClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rpc: Mutex::new(None),
            read: AtomicBool::new(false),
            write: AtomicBool::new(false),
        }
    }

    pub async fn attach(&self, rpc: Arc<dyn ClientRpc>, read: bool, write: bool) {
        *self.rpc.lock().await = Some(rpc);
        self.read.store(read, Ordering::SeqCst);
        self.write.store(write, Ordering::SeqCst);
    }

    pub async fn read_file(&self, path: &str) -> Result<Option<String>, String> {
        if !self.read.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let Some(rpc) = self.rpc.lock().await.clone() else {
            return Ok(None);
        };
        let result = rpc
            .request("fs/read_text_file", json!({"path": path}))
            .await?;
        Ok(result["content"].as_str().map(ToOwned::to_owned))
    }

    pub async fn write_file(&self, path: &str, content: &str) -> Result<bool, String> {
        if !self.write.load(Ordering::SeqCst) {
            return Ok(false);
        }
        let Some(rpc) = self.rpc.lock().await.clone() else {
            return Ok(false);
        };
        rpc.request(
            "fs/write_text_file",
            json!({"path": path, "content": content}),
        )
        .await?;
        Ok(true)
    }
}

impl Default for AcpFsClient {
    fn default() -> Self {
        Self::new()
    }
}
