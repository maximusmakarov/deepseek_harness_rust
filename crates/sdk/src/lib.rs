#![forbid(unsafe_code)]
//! Typed HTTP/SSE client for the dshr server.

use std::time::Duration;

use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("http: {0}")]
    Http(String),

    #[error("status {status}: {body}")]
    Status { status: u16, body: String },

    #[error("json: {0}")]
    Json(String),
}

impl From<reqwest::Error> for SdkError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: String,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub turns: u64,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionView {
    pub session: SessionSummary,
    pub messages: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnAccepted {
    pub accepted: bool,
    pub session_id: String,
}

#[derive(Clone)]
pub struct Client {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Client {
    #[must_use]
    pub fn new(base: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.base, path))
            .bearer_auth(&self.token)
    }

    async fn json<T: for<'de> Deserialize<'de>>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, SdkError> {
        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(SdkError::Status {
                status: status.as_u16(),
                body,
            });
        }
        serde_json::from_str(&body).map_err(|error| SdkError::Json(error.to_string()))
    }

    pub async fn health(&self) -> Result<Value, SdkError> {
        self.json(self.request(reqwest::Method::GET, "/v1/health"))
            .await
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>, SdkError> {
        #[derive(Deserialize)]
        struct Response {
            sessions: Vec<SessionSummary>,
        }
        let response: Response = self
            .json(self.request(reqwest::Method::GET, "/v1/sessions"))
            .await?;
        Ok(response.sessions)
    }

    pub async fn create_session(&self) -> Result<SessionSummary, SdkError> {
        #[derive(Deserialize)]
        struct Response {
            session: SessionSummary,
        }
        let response: Response = self
            .json(
                self.request(reqwest::Method::POST, "/v1/sessions")
                    .json(&json!({})),
            )
            .await?;
        Ok(response.session)
    }

    pub async fn session(&self, id: &str) -> Result<SessionView, SdkError> {
        self.json(self.request(reqwest::Method::GET, &format!("/v1/sessions/{id}")))
            .await
    }

    pub async fn start_turn(&self, id: &str, prompt: &str) -> Result<TurnAccepted, SdkError> {
        self.json(
            self.request(reqwest::Method::POST, &format!("/v1/sessions/{id}/turns"))
                .json(&json!({"prompt": prompt})),
        )
        .await
    }

    pub async fn cancel(&self, id: &str) -> Result<(), SdkError> {
        let response = self
            .request(reqwest::Method::POST, &format!("/v1/sessions/{id}/cancel"))
            .json(&json!({}))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(SdkError::Status {
                status: response.status().as_u16(),
                body: response.text().await.unwrap_or_default(),
            })
        }
    }

    pub async fn approve(
        &self,
        session: &str,
        approval: &str,
        approved: bool,
    ) -> Result<(), SdkError> {
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("/v1/sessions/{session}/approvals/{approval}"),
            )
            .json(&json!({"approved": approved}))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(SdkError::Status {
                status: response.status().as_u16(),
                body: response.text().await.unwrap_or_default(),
            })
        }
    }

    pub fn events(
        &self,
        id: impl Into<String>,
    ) -> impl Stream<Item = Result<Value, SdkError>> + Send + 'static {
        let client = self.clone();
        let id = id.into();
        async_stream::stream! {
            let response = match client
                .request(reqwest::Method::GET, &format!("/v1/sessions/{id}/events"))
                .header(reqwest::header::ACCEPT, "text/event-stream")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    yield Err(SdkError::Http(error.to_string()));
                    return;
                }
            };
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                yield Err(SdkError::Status { status, body });
                return;
            }
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        yield Err(SdkError::Http(error.to_string()));
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(position) = buffer.find("\n\n") {
                    let event = buffer[..position].to_owned();
                    buffer.drain(..position + 2);
                    let data = event
                        .lines()
                        .filter_map(|line| line.strip_prefix("data:").map(str::trim))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if data.is_empty() {
                        continue;
                    }
                    yield serde_json::from_str(&data)
                        .map_err(|error| SdkError::Json(error.to_string()));
                }
            }
        }
    }
}
