//! LLM-generated session titles (DSH-compatible first-prompt title).

use dshr_llm::{LlmProvider, Message, Request, StreamChunk};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

const SYSTEM: &str = "Create a concise title for an AI coding-assistant session from the supplied human messages.\nReturn only the title on one line, in plain text of the natural language, with no quotes, prefix, explanation, Markdown, XML, or terminal control codes. No code is allowed.\nUse the language of the messages.\nAim for about 5 words in non-CJK languages or 10 CJK characters.\nIf the messages give little to name, still return a short best-effort title.";

/// Asks the provider for a short session title. Returns `None` for an empty
/// answer; transport errors are propagated.
pub async fn generate_title(
    provider: &dyn LlmProvider,
    prompt: &str,
) -> Result<Option<String>, dshr_llm::LlmError> {
    let request = Request {
        model: provider.model().to_owned(),
        system: Some(SYSTEM.to_owned()),
        messages: vec![Message::user(prompt)],
        max_tokens: 24,
        temperature: Some(0.2),
        tools: Vec::new(),
        stop_sequences: vec!["\n".to_owned()],
    };
    let mut stream = provider.stream(request, CancellationToken::new()).await?;
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk? {
            StreamChunk::TextDelta { text: delta } => text.push_str(&delta),
            StreamChunk::Done { .. } => break,
            _ => {}
        }
    }
    let title = sanitize(&text);
    Ok((!title.is_empty()).then_some(title))
}

fn sanitize(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    let line = line.trim_matches(['"', '\'', '`', '#', '*', ' ']).trim();
    line.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_llm::StopReason;
    use dshr_llm_mock::ScriptedProvider;

    #[tokio::test]
    async fn generates_and_sanitizes_a_title() {
        let provider = ScriptedProvider::new(
            "test-model",
            vec![vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::TextDelta {
                    text: "\"ACP audio and MCP\"".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]],
        );
        let title = generate_title(&provider, "continue the ACP project")
            .await
            .expect("title")
            .expect("some");
        assert_eq!(title, "ACP audio and MCP");
    }

    #[tokio::test]
    async fn empty_answer_is_none() {
        let provider = ScriptedProvider::new(
            "test-model",
            vec![vec![StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            }]],
        );
        assert!(
            generate_title(&provider, "hi")
                .await
                .expect("title")
                .is_none()
        );
    }
}
