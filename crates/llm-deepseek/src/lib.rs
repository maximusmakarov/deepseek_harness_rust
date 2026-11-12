#![forbid(unsafe_code)]
//! DeepSeek adapter speaking the Anthropic Messages wire format.
//!
//! Verified against `https://api.deepseek.com/anthropic` on 2026-11-12:
//! `POST /v1/messages`, SSE events `message_start` / `content_block_*` /
//! `message_delta` / `message_stop`, `thinking` blocks, cache usage fields.

use std::fmt;
use std::time::Duration;

use async_stream::stream;
use dshr_llm::{
    Capabilities, ChunkStream, ContentBlock, LlmError, LlmProvider, Request, Role, SseDecoder,
    SseEvent, StopReason, StreamChunk, Usage,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/anthropic";
const DEFAULT_MODEL: &str = "deepseek-flash";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Credential mode for the official endpoint.
#[derive(Clone)]
pub enum Auth {
    ApiKey(String),
    AccountToken(String),
}

impl fmt::Debug for Auth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKey(_) => formatter.write_str("ApiKey(<redacted>)"),
            Self::AccountToken(_) => formatter.write_str("AccountToken(<redacted>)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeepSeekConfig {
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub auth: Auth,
}

impl fmt::Debug for DeepSeekConfigDebug<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeepSeekConfig")
            .field("base_url", &self.0.base_url)
            .field("model", &self.0.model)
            .field("max_tokens", &self.0.max_tokens)
            .field("auth", &self.0.auth)
            .finish()
    }
}

struct DeepSeekConfigDebug<'a>(&'a DeepSeekConfig);

pub struct DeepSeekProvider {
    client: reqwest::Client,
    config: DeepSeekConfig,
}

impl fmt::Debug for DeepSeekProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&DeepSeekConfigDebug(&self.config), formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

impl DeepSeekProvider {
    pub fn from_env(model: Option<String>, max_tokens: Option<u32>) -> Result<Self, LlmError> {
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let model = model
            .or_else(|| std::env::var("DEEPSEEK_MODEL").ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        let max_tokens = max_tokens.unwrap_or(2048).max(1);

        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let account_token = std::env::var("DSH_DEEPSEEK_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let auth = if let Some(key) = api_key {
            Auth::ApiKey(key)
        } else if let Some(token) = account_token {
            Auth::AccountToken(token)
        } else {
            return Err(LlmError::MissingCredentials(
                "set DEEPSEEK_API_KEY or DSH_DEEPSEEK_TOKEN".to_owned(),
            ));
        };

        Self::new(DeepSeekConfig {
            base_url,
            model,
            max_tokens,
            auth,
        })
    }

    pub fn new(config: DeepSeekConfig) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|err| LlmError::Transport(err.to_string()))?;
        Ok(Self { client, config })
    }

    #[must_use]
    pub fn config(&self) -> &DeepSeekConfig {
        &self.config
    }

    /// Lists models through the same authentication as Messages.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let url = models_url(&self.config.base_url);
        let request = self.authorize(self.client.get(url));
        let response = request
            .send()
            .await
            .map_err(|err| LlmError::Transport(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::Http {
                status: status.as_u16(),
                body: truncate(&body, 2000),
            });
        }

        let value: Value = response
            .json()
            .await
            .map_err(|err| LlmError::Protocol(format!("invalid model listing: {err}")))?;
        let models = value["data"]
            .as_array()
            .ok_or_else(|| LlmError::Protocol("model listing has no data array".into()))?
            .iter()
            .filter_map(|entry| {
                let id = entry["id"].as_str()?;
                let name = entry["name"].as_str().unwrap_or(id);
                Some(ModelInfo {
                    id: id.to_owned(),
                    name: name.to_owned(),
                })
            })
            .collect();
        Ok(models)
    }

    async fn send_with_retry(
        &self,
        url: &str,
        body: &Value,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response, LlmError> {
        const MAX_ATTEMPTS: u32 = 3;
        let mut attempt = 1;
        loop {
            let result = self
                .authorize(self.client.post(url).json(body))
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let status = response.status();
                    let retryable = status.as_u16() == 429 || status.is_server_error();
                    if retryable && attempt < MAX_ATTEMPTS {
                        let delay = retry_delay(attempt, response.headers());
                        drop(response);
                        tokio::select! {
                            _ = cancel.cancelled() => return Err(LlmError::Cancelled),
                            _ = tokio::time::sleep(delay) => {}
                        }
                        attempt += 1;
                        continue;
                    }
                    let body = response.text().await.unwrap_or_default();
                    return Err(LlmError::Http {
                        status: status.as_u16(),
                        body: truncate(&body, 2000),
                    });
                }
                Err(err) if attempt < MAX_ATTEMPTS && (err.is_timeout() || err.is_connect()) => {
                    let delay = retry_delay(attempt, &reqwest::header::HeaderMap::new());
                    tokio::select! {
                        _ = cancel.cancelled() => return Err(LlmError::Cancelled),
                        _ = tokio::time::sleep(delay) => {}
                    }
                    attempt += 1;
                }
                Err(err) => return Err(LlmError::Transport(err.to_string())),
            }
        }
    }

    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth {
            Auth::ApiKey(key) => request.header("x-api-key", key),
            Auth::AccountToken(token) => request.header("x-dsh-auth-token", token),
        }
        .header("anthropic-version", ANTHROPIC_VERSION)
    }
}

#[async_trait::async_trait]
impl LlmProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: true,
            vision: true,
            reasoning: true,
        }
    }

    async fn stream(
        &self,
        request: Request,
        cancel: CancellationToken,
    ) -> Result<ChunkStream, LlmError> {
        let body = request_body(&request, self.config.max_tokens);
        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));
        let response = self.send_with_retry(&url, &body, &cancel).await?;
        let mut bytes = response.bytes_stream();
        let mut decoder = SseDecoder::new();

        let stream = stream! {
            'outer: loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        yield Err(LlmError::Cancelled);
                        break 'outer;
                    }
                    item = bytes.next() => {
                        match item {
                            None => {
                                for event in decoder.finish() {
                                    match map_event(event) {
                                        Ok(chunks) => for chunk in chunks { yield Ok(chunk); },
                                        Err(err) => { yield Err(err); break 'outer; }
                                    }
                                }
                                break 'outer;
                            }
                            Some(Err(err)) => {
                                yield Err(LlmError::Transport(err.to_string()));
                                break 'outer;
                            }
                            Some(Ok(chunk)) => {
                                for event in decoder.push(&chunk) {
                                    match map_event(event) {
                                        Ok(chunks) => for chunk in chunks { yield Ok(chunk); },
                                        Err(err) => { yield Err(err); break 'outer; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

fn retry_delay(attempt: u32, headers: &reqwest::header::HeaderMap) -> Duration {
    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let millis = retry_after.map_or(250 * 2u64.pow(attempt - 1), |seconds| seconds * 1000);
    Duration::from_millis(millis.min(5000))
}

fn models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let root = base.strip_suffix("/anthropic").unwrap_or(base);
    format!("{root}/models")
}

fn request_body(request: &Request, default_max_tokens: u32) -> Value {
    let mut messages = Vec::new();
    for message in &request.messages {
        let role = match message.role {
            Role::System => continue,
            Role::User | Role::Tool => "user",
            Role::Assistant => "assistant",
        };
        let content: Vec<Value> = message.content.iter().filter_map(block_to_wire).collect();
        if content.is_empty() {
            continue;
        }
        messages.push(json!({ "role": role, "content": content }));
    }

    let mut body = json!({
        "model": request.model,
        "max_tokens": if request.max_tokens == 0 { default_max_tokens } else { request.max_tokens },
        "stream": true,
        "messages": messages,
    });

    if let Some(system) = &request.system
        && !system.trim().is_empty()
    {
        body["system"] = json!(system);
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    if !request.stop_sequences.is_empty() {
        body["stop_sequences"] = json!(request.stop_sequences);
    }
    if !request.tools.is_empty() {
        body["tools"] = json!(
            request
                .tools
                .iter()
                .map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                }))
                .collect::<Vec<_>>()
        );
    }

    body
}

fn block_to_wire(block: &ContentBlock) -> Option<Value> {
    match block {
        ContentBlock::Text { text } if !text.is_empty() => {
            Some(json!({"type": "text", "text": text}))
        }
        ContentBlock::Text { .. } => None,
        // Thinking history is not replayed: the official endpoint expects
        // provider-signed thinking blocks, which we do not retain.
        ContentBlock::Thinking { .. } => None,
        ContentBlock::ToolUse { id, name, input } => Some(json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": input,
        })),
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => Some(json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
            "is_error": is_error,
        })),
        ContentBlock::Image { media_type, data } => Some(json!({
            "type": "image",
            "source": {"type": "base64", "media_type": media_type, "data": data},
        })),
    }
}

/// Maps one SSE event to normalized chunks.
pub fn map_event(event: SseEvent) -> Result<Vec<StreamChunk>, LlmError> {
    if event.data.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(&event.data)
        .map_err(|err| LlmError::Protocol(format!("invalid SSE payload: {err}")))?;

    match value["type"].as_str().unwrap_or_default() {
        "message_start" => {
            let message = &value["message"];
            let model = message["model"].as_str().unwrap_or_default().to_owned();
            let usage = usage_from(&message["usage"]);
            Ok(vec![
                StreamChunk::Start { model },
                StreamChunk::Usage { usage },
            ])
        }
        "content_block_start" => {
            let index = value["index"].as_u64().unwrap_or(0) as u32;
            let block = &value["content_block"];
            if block["type"].as_str() == Some("tool_use") {
                Ok(vec![StreamChunk::ToolUseStart {
                    index,
                    id: block["id"].as_str().unwrap_or_default().to_owned(),
                    name: block["name"].as_str().unwrap_or_default().to_owned(),
                }])
            } else {
                Ok(Vec::new())
            }
        }
        "content_block_delta" => {
            let index = value["index"].as_u64().unwrap_or(0) as u32;
            let delta = &value["delta"];
            match delta["type"].as_str().unwrap_or_default() {
                "text_delta" => Ok(vec![StreamChunk::TextDelta {
                    text: delta["text"].as_str().unwrap_or_default().to_owned(),
                }]),
                "thinking_delta" => Ok(vec![StreamChunk::ThinkingDelta {
                    text: delta["thinking"].as_str().unwrap_or_default().to_owned(),
                }]),
                "input_json_delta" => Ok(vec![StreamChunk::ToolInputDelta {
                    index,
                    json: delta["partial_json"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                }]),
                _ => Ok(Vec::new()),
            }
        }
        "message_delta" => {
            let mut chunks = Vec::new();
            let usage = usage_from(&value["usage"]);
            if usage != Usage::default() {
                chunks.push(StreamChunk::Usage { usage });
            }
            if let Some(reason) = value["delta"]["stop_reason"].as_str() {
                chunks.push(StreamChunk::Done {
                    stop_reason: stop_reason(reason),
                });
            }
            Ok(chunks)
        }
        "message_stop" | "content_block_stop" | "ping" => Ok(Vec::new()),
        "error" => {
            let message = value["error"]["message"]
                .as_str()
                .unwrap_or("provider error");
            Err(LlmError::Protocol(message.to_owned()))
        }
        other => Err(LlmError::Protocol(format!("unknown SSE event '{other}'"))),
    }
}

fn usage_from(value: &Value) -> Usage {
    Usage {
        input_tokens: value["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: value["output_tokens"].as_u64().unwrap_or(0),
        cache_creation_input_tokens: value["cache_creation_input_tokens"].as_u64().unwrap_or(0),
        cache_read_input_tokens: value["cache_read_input_tokens"].as_u64().unwrap_or(0),
    }
}

fn stop_reason(value: &str) -> StopReason {
    match value {
        "end_turn" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        other => StopReason::Other(other.to_owned()),
    }
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_owned();
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_llm::{BlockAssembler, Message, ToolDef};

    fn config() -> DeepSeekConfig {
        DeepSeekConfig {
            base_url: DEFAULT_BASE_URL.into(),
            model: DEFAULT_MODEL.into(),
            max_tokens: 64,
            auth: Auth::ApiKey("test".into()),
        }
    }

    #[test]
    fn builds_anthropic_request_body() {
        let request = Request {
            model: "deepseek-flash".into(),
            system: Some("be brief".into()),
            messages: vec![Message::user("hi")],
            max_tokens: 32,
            temperature: Some(0.2),
            tools: vec![ToolDef {
                name: "read".into(),
                description: "read file".into(),
                input_schema: json!({"type": "object"}),
            }],
            stop_sequences: vec!["STOP".into()],
        };
        let body = request_body(&request, 64);
        assert_eq!(body["model"], "deepseek-flash");
        assert_eq!(body["max_tokens"], 32);
        assert_eq!(body["system"], "be brief");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["tools"][0]["name"], "read");
    }

    #[test]
    fn maps_text_stream_events() {
        let start = map_event(SseEvent {
            event: Some("message_start".into()),
            data: json!({
                "type": "message_start",
                "message": {"model": "deepseek-flash", "usage": {"input_tokens": 11}}
            })
            .to_string(),
        })
        .expect("start");
        assert!(matches!(start[0], StreamChunk::Start { .. }));

        let delta = map_event(SseEvent {
            event: Some("content_block_delta".into()),
            data: json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "pong"}
            })
            .to_string(),
        })
        .expect("delta");
        assert_eq!(
            delta,
            vec![StreamChunk::TextDelta {
                text: "pong".into()
            }]
        );
    }

    #[test]
    fn maps_tool_use_deltas() {
        let start = map_event(SseEvent {
            event: Some("content_block_start".into()),
            data: json!({
                "type": "content_block_start",
                "index": 1,
                "content_block": {"type": "tool_use", "id": "call_1", "name": "read"}
            })
            .to_string(),
        })
        .expect("start");
        assert_eq!(
            start,
            vec![StreamChunk::ToolUseStart {
                index: 1,
                id: "call_1".into(),
                name: "read".into(),
            }]
        );

        let delta = map_event(SseEvent {
            event: Some("content_block_delta".into()),
            data: json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {"type": "input_json_delta", "partial_json": "{\"path\":"}
            })
            .to_string(),
        })
        .expect("delta");
        assert_eq!(
            delta,
            vec![StreamChunk::ToolInputDelta {
                index: 1,
                json: "{\"path\":".into(),
            }]
        );
    }

    #[test]
    fn maps_stop_reason_and_usage() {
        let chunks = map_event(SseEvent {
            event: Some("message_delta".into()),
            data: json!({
                "type": "message_delta",
                "delta": {"stop_reason": "tool_use"},
                "usage": {"output_tokens": 7}
            })
            .to_string(),
        })
        .expect("delta");
        assert!(chunks.iter().any(|chunk| matches!(
            chunk,
            StreamChunk::Done {
                stop_reason: StopReason::ToolUse
            }
        )));
        assert!(chunks.iter().any(
            |chunk| matches!(chunk, StreamChunk::Usage { usage } if usage.output_tokens == 7)
        ));
    }

    #[test]
    fn assembles_end_to_end_from_events() {
        let mut assembler = BlockAssembler::new();
        for event in [
            json!({"type":"message_start","message":{"model":"deepseek-flash","usage":{"input_tokens":3}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text"}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}),
        ] {
            for chunk in map_event(SseEvent {
                event: None,
                data: event.to_string(),
            })
            .expect("map")
            {
                assembler.push(chunk);
            }
        }
        let output = assembler.finish().expect("assemble");
        assert_eq!(output.content, vec![ContentBlock::text("ok")]);
        assert_eq!(output.stop_reason, Some(StopReason::EndTurn));
    }

    #[test]
    fn models_url_strips_anthropic_suffix() {
        assert_eq!(
            models_url("https://api.deepseek.com/anthropic"),
            "https://api.deepseek.com/models"
        );
    }

    #[test]
    fn caps_max_tokens_at_configured_default() {
        let request = Request {
            model: "deepseek-flash".into(),
            messages: vec![Message::user("hi")],
            max_tokens: 0,
            ..Request::default()
        };
        assert_eq!(request_body(&request, 64)["max_tokens"], 64);
    }

    #[test]
    fn debug_redacts_credentials() {
        let provider = DeepSeekProvider::new(config()).expect("provider");
        let text = format!("{provider:?}");
        assert!(!text.contains("test"), "{text}");
        assert!(text.contains("<redacted>"));
    }

    #[tokio::test]
    async fn streams_over_http_mock() {
        use dshr_llm_mock::MockServer;
        use futures_util::StreamExt;

        let body = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"deepseek-flash\",\"usage\":{\"input_tokens\":3}}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"pong\"}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

        let server = MockServer::start(body).await.expect("server");
        let provider = DeepSeekProvider::new(DeepSeekConfig {
            base_url: server.base_url(),
            model: "deepseek-flash".into(),
            max_tokens: 32,
            auth: Auth::ApiKey("test".into()),
        })
        .expect("provider");

        let request = Request {
            model: "deepseek-flash".into(),
            messages: vec![Message::user("hi")],
            max_tokens: 32,
            ..Request::default()
        };
        let mut stream = provider
            .stream(request, CancellationToken::new())
            .await
            .expect("stream");
        let mut text = String::new();
        let mut done = false;
        while let Some(item) = stream.next().await {
            match item.expect("chunk") {
                StreamChunk::TextDelta { text: delta } => text.push_str(&delta),
                StreamChunk::Done { .. } => done = true,
                _ => {}
            }
        }
        assert_eq!(text, "pong");
        assert!(done);
        server.abort();
    }

    #[test]
    #[ignore = "requires DEEPSEEK_API_KEY or DSH_DEEPSEEK_TOKEN"]
    fn live_smoke() {
        // Run with: cargo test -p dshr-llm-deepseek -- --ignored live_smoke --nocapture
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let provider = DeepSeekProvider::from_env(None, Some(64)).expect("credentials");
            let models = provider.list_models().await.expect("models");
            assert!(!models.is_empty(), "model listing is empty");

            let request = Request {
                model: provider.model().to_owned(),
                messages: vec![Message::user("Reply with the single word: pong")],
                max_tokens: 16,
                ..Request::default()
            };
            let mut chunks = provider
                .stream(request, CancellationToken::new())
                .await
                .expect("stream");
            let mut text = String::new();
            while let Some(chunk) = chunks.next().await {
                match chunk.expect("chunk") {
                    StreamChunk::TextDelta { text: delta } => text.push_str(&delta),
                    StreamChunk::Done { .. } => break,
                    _ => {}
                }
            }
            assert!(!text.trim().is_empty(), "no text in live response");
        });
    }
}
