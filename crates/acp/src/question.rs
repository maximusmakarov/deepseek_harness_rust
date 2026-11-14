//! ACP elicitation: ask the editor client a question from tools.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use dshr_tools::UserPrompt;
use serde_json::json;
use tokio::sync::Mutex;

use crate::ClientRpc;

pub struct AcpQuestionClient {
    rpc: Mutex<Option<Arc<dyn ClientRpc>>>,
    enabled: AtomicBool,
}

impl AcpQuestionClient {
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

    pub async fn ask(
        &self,
        session_id: &str,
        question: &str,
        options: &[String],
    ) -> Option<String> {
        if !self.enabled.load(Ordering::SeqCst) {
            return None;
        }
        let rpc = self.rpc.lock().await.clone()?;
        let property = if options.is_empty() {
            json!({"type": "string", "title": question})
        } else {
            json!({"type": "string", "title": question, "enum": options})
        };
        let result = rpc
            .request(
                "elicitation/create",
                json!({
                    "sessionId": session_id,
                    "message": question,
                    "requestedSchema": {
                        "type": "object",
                        "properties": {"answer": property},
                        "required": ["answer"]
                    }
                }),
            )
            .await
            .ok()?;
        if result["action"].as_str() != Some("accept") {
            return None;
        }
        result["content"]["answer"].as_str().map(ToOwned::to_owned)
    }
}

impl Default for AcpQuestionClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AcpPrompt {
    pub client: Arc<AcpQuestionClient>,
    pub session_id: String,
}

#[async_trait]
impl UserPrompt for AcpPrompt {
    async fn ask(&self, question: &str, options: &[String]) -> Option<String> {
        self.client.ask(&self.session_id, question, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::Mutex as StdMutex;

    struct MockRpc {
        requests: StdMutex<Vec<(String, Value)>>,
        response: Value,
    }

    #[async_trait]
    impl ClientRpc for MockRpc {
        async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
            self.requests
                .lock()
                .expect("requests")
                .push((method.to_owned(), params));
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn asks_through_elicitation() {
        let rpc = Arc::new(MockRpc {
            requests: StdMutex::new(Vec::new()),
            response: json!({"action": "accept", "content": {"answer": "yes"}}),
        });
        let client = AcpQuestionClient::new();
        client.attach(rpc.clone(), true).await;
        let answer = client
            .ask("s1", "Continue?", &["yes".into(), "no".into()])
            .await;
        assert_eq!(answer.as_deref(), Some("yes"));
        let requests = rpc.requests.lock().expect("requests");
        assert_eq!(requests[0].0, "elicitation/create");
        assert_eq!(requests[0].1["sessionId"], "s1");
        assert_eq!(
            requests[0].1["requestedSchema"]["properties"]["answer"]["enum"][0],
            "yes"
        );
    }

    #[tokio::test]
    async fn declines_without_capability() {
        let rpc = Arc::new(MockRpc {
            requests: StdMutex::new(Vec::new()),
            response: json!({"action": "accept", "content": {"answer": "yes"}}),
        });
        let client = AcpQuestionClient::new();
        client.attach(rpc.clone(), false).await;
        assert!(client.ask("s1", "Continue?", &[]).await.is_none());
        assert!(rpc.requests.lock().expect("requests").is_empty());
    }
}
