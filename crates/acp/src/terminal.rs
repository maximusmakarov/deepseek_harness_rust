use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tokio::sync::Mutex;

use crate::ClientRpc;

pub struct AcpTerminalClient {
    rpc: Mutex<Option<Arc<dyn ClientRpc>>>,
    enabled: AtomicBool,
}

impl AcpTerminalClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rpc: Mutex::new(None),
            enabled: AtomicBool::new(false),
        }
    }

    pub async fn attach(&self, rpc: Arc<dyn ClientRpc>, enabled: bool) {
        *self.rpc.lock().await = Some(rpc);
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub async fn run(
        &self,
        session_id: &str,
        command: &str,
        cwd: &str,
    ) -> Result<Option<(String, i32)>, String> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let Some(rpc) = self.rpc.lock().await.clone() else {
            return Ok(None);
        };
        let shell = if cfg!(windows) {
            ("cmd", vec!["/C".to_owned(), command.to_owned()])
        } else {
            ("sh", vec!["-c".to_owned(), command.to_owned()])
        };
        let created = rpc
            .request(
                "terminal/create",
                json!({
                    "sessionId": session_id,
                    "command": shell.0,
                    "args": shell.1,
                    "cwd": cwd,
                    "outputByteLimit": 1_048_576
                }),
            )
            .await?;
        let terminal_id = created["terminalId"]
            .as_str()
            .ok_or_else(|| "terminal/create returned no terminalId".to_owned())?
            .to_owned();
        let status = rpc
            .request(
                "terminal/wait_for_exit",
                json!({"sessionId": session_id, "terminalId": terminal_id}),
            )
            .await;
        let output = rpc
            .request(
                "terminal/output",
                json!({"sessionId": session_id, "terminalId": terminal_id}),
            )
            .await;
        let _ = rpc
            .request(
                "terminal/release",
                json!({"sessionId": session_id, "terminalId": terminal_id}),
            )
            .await;
        let status = status?;
        let output = output?;
        Ok(Some((
            output["output"].as_str().unwrap_or_default().to_owned(),
            status["exitCode"].as_i64().unwrap_or(1) as i32,
        )))
    }
}

impl Default for AcpTerminalClient {
    fn default() -> Self {
        Self::new()
    }
}
