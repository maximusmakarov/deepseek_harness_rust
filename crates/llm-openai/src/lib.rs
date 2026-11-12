#![forbid(unsafe_code)]
//! OpenAI-compatible chat-completions adapter (RouterAI and similar gateways).

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

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "deepseek-chat";

#[derive(Clone)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub api_key: String,
}

impl fmt::Debug for OpenAiConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiConfig")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

pub struct OpenAiProvider {
    client: reqwest::Client,
    config: OpenAiConfig,
}

impl fmt::Debug for OpenAiProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.config, formatter)
    }
}

impl OpenAiProvider {
    pub fn from_env(model: Option<String>, max_tokens: Option<u32>) -> Result<Self, LlmError> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let model = model
            .or_else(|| std::env::var("OPENAI_MODEL").ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| std::env::var("ROUTERAI_API_KEY").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                LlmError::MissingCredentials("set OPENAI_API_KEY or ROUTERAI_API_KEY".to_owned())
            })?;

        Self::new(OpenAiConfig {
            base_url,
            model,
            max_tokens: max_tokens.unwrap_or(2048).max(1),
            api_key,
        })
    }

    pub fn new(config: OpenAiConfig) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|err| LlmError::Transport(err.to_string()))?;
        Ok(Self { client, config })
    }

    #[must_use]
    pub fn config(&self) -> &OpenAiConfig {
        &self.config
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
                .client
                .post(url)
                .bearer_auth(&self.config.api_key)
                .json(body)
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
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai-compatible"
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
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
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

fn request_body(request: &Request, default_max_tokens: u32) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = &request.system
        && !system.trim().is_empty()
    {
        messages.push(json!({"role": "system", "content": system}));
    }

    for message in &request.messages {
        match message.role {
            Role::System => {
                let text = text_of(&message.content);
                if !text.is_empty() {
                    messages.push(json!({"role": "system", "content": text}));
                }
            }
            Role::Tool => {
                for block in &message.content {
                    if let ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = block
                    {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": content,
                        }));
                    }
                }
            }
            Role::Assistant => {
                let text = text_of(&message.content);
                let calls: Vec<Value> = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::ToolUse { id, name, input } => Some(json!({
                            "id": id,
                            "type": "function",
                            "function": {"name": name, "arguments": input.to_string()},
                        })),
                        _ => None,
                    })
                    .collect();
                let mut entry = json!({"role": "assistant"});
                if !text.is_empty() {
                    entry["content"] = json!(text);
                }
                if !calls.is_empty() {
                    entry["tool_calls"] = json!(calls);
                }
                messages.push(entry);
            }
            Role::User => {
                let parts: Vec<Value> = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } if !text.is_empty() => {
                            Some(json!({"type": "text", "text": text}))
                        }
                        ContentBlock::Image { media_type, data } => Some(json!({
                            "type": "image_url",
                            "image_url": {"url": format!("data:{media_type};base64,{data}")},
                        })),
                        _ => None,
                    })
                    .collect();
                if !parts.is_empty() {
                    messages.push(json!({"role": "user", "content": parts}));
                }
            }
        }
    }

    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
        "max_tokens": if request.max_tokens == 0 { default_max_tokens } else { request.max_tokens },
        "stream_options": {"include_usage": true},
    });
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    if !request.stop_sequences.is_empty() {
        body["stop"] = json!(request.stop_sequences);
    }
    if !request.tools.is_empty() {
        body["tools"] = json!(
            request
                .tools
                .iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }
                }))
                .collect::<Vec<_>>()
        );
    }

    body
}

fn text_of(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Maps one `data:` payload to normalized chunks.
pub fn map_event(event: SseEvent) -> Result<Vec<StreamChunk>, LlmError> {
    let data = event.data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(data)
        .map_err(|err| LlmError::Protocol(format!("invalid SSE payload: {err}")))?;

    let mut chunks = Vec::new();

    if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
        let usage = Usage {
            input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
            output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
            cache_creation_input_tokens: usage["prompt_tokens_details"]["cache_creation_tokens"]
                .as_u64()
                .unwrap_or(0),
            cache_read_input_tokens: usage["prompt_tokens_details"]["cached_tokens"]
                .as_u64()
                .unwrap_or(0),
        };
        if usage != Usage::default() {
            chunks.push(StreamChunk::Usage { usage });
        }
    }

    let Some(choice) = value["choices"]
        .as_array()
        .and_then(|choices| choices.first())
    else {
        return Ok(chunks);
    };
    let delta = &choice["delta"];

    if let Some(text) = delta["content"].as_str()
        && !text.is_empty()
    {
        chunks.push(StreamChunk::TextDelta {
            text: text.to_owned(),
        });
    }
    if let Some(text) = delta["reasoning_content"].as_str()
        && !text.is_empty()
    {
        chunks.push(StreamChunk::ThinkingDelta {
            text: text.to_owned(),
        });
    }

    if let Some(calls) = delta["tool_calls"].as_array() {
        for call in calls {
            let index = call["index"].as_u64().unwrap_or(0) as u32;
            let function = &call["function"];
            if let Some(id) = call["id"].as_str()
                && !id.is_empty()
            {
                chunks.push(StreamChunk::ToolUseStart {
                    index,
                    id: id.to_owned(),
                    name: function["name"].as_str().unwrap_or_default().to_owned(),
                });
            }
            if let Some(arguments) = function["arguments"].as_str()
                && !arguments.is_empty()
            {
                chunks.push(StreamChunk::ToolInputDelta {
                    index,
                    json: arguments.to_owned(),
                });
            }
        }
    }

    if let Some(reason) = choice["finish_reason"].as_str() {
        chunks.push(StreamChunk::Done {
            stop_reason: stop_reason(reason),
        });
    }

    Ok(chunks)
}

fn retry_delay(attempt: u32, headers: &reqwest::header::HeaderMap) -> Duration {
    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let millis = retry_after.map_or(250 * 2u64.pow(attempt - 1), |seconds| seconds * 1000);
    Duration::from_millis(millis.min(5000))
}

fn stop_reason(value: &str) -> StopReason {
    match value {
        "stop" => StopReason::EndTurn,
        "tool_calls" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
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

    fn request() -> Request {
        Request {
            model: "deepseek-v4-flash-latest".into(),
            system: Some("be brief".into()),
            messages: vec![Message::user("hi")],
            max_tokens: 32,
            temperature: Some(0.1),
            tools: vec![ToolDef {
                name: "read".into(),
                description: "read file".into(),
                input_schema: json!({"type": "object"}),
            }],
            stop_sequences: vec!["STOP".into()],
        }
    }

    #[test]
    fn builds_openai_request_body() {
        let body = request_body(&request(), 64);
        assert_eq!(body["model"], "deepseek-v4-flash-latest");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["stream_options"]["include_usage"], true);
    }

    #[test]
    fn maps_text_and_finish_reason() {
        let chunks = map_event(SseEvent {
            event: None,
            data: json!({
                "choices": [{"delta": {"content": "pong"}, "finish_reason": null}]
            })
            .to_string(),
        })
        .expect("map");
        assert_eq!(
            chunks,
            vec![StreamChunk::TextDelta {
                text: "pong".into()
            }]
        );

        let chunks = map_event(SseEvent {
            event: None,
            data: json!({
                "choices": [{"delta": {}, "finish_reason": "stop"}]
            })
            .to_string(),
        })
        .expect("map");
        assert_eq!(
            chunks,
            vec![StreamChunk::Done {
                stop_reason: StopReason::EndTurn
            }]
        );
    }

    #[test]
    fn maps_tool_call_deltas() {
        let chunks = map_event(SseEvent {
            event: None,
            data: json!({
                "choices": [{"delta": {"tool_calls": [{
                    "index": 0,
                    "id": "call_1",
                    "function": {"name": "read", "arguments": "{\"path\":\"a\"}"}
                }]}, "finish_reason": null}]
            })
            .to_string(),
        })
        .expect("map");
        assert!(matches!(chunks[0], StreamChunk::ToolUseStart { .. }));
        assert!(matches!(chunks[1], StreamChunk::ToolInputDelta { .. }));
    }

    #[test]
    fn ignores_done_marker() {
        let chunks = map_event(SseEvent {
            event: None,
            data: "[DONE]".into(),
        })
        .expect("map");
        assert!(chunks.is_empty());
    }

    #[test]
    fn assembles_from_openai_events() {
        let mut assembler = BlockAssembler::new();
        for data in [
            json!({"choices":[{"delta":{"content":"ok"},"finish_reason":null}]}),
            json!({"choices":[{"delta":{},"finish_reason":"stop"}]}),
        ] {
            for chunk in map_event(SseEvent {
                event: None,
                data: data.to_string(),
            })
            .expect("map")
            {
                assembler.push(chunk);
            }
        }
        let output = assembler.finish().expect("assemble");
        assert_eq!(output.content, vec![ContentBlock::text("ok")]);
    }
}
