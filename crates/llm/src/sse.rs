/// One Server-Sent Event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

/// Incremental SSE decoder.
///
/// Accepts arbitrary byte chunks (including splits inside a UTF-8 sequence or
/// inside a line) and emits events on blank lines. `ping` and comment lines are
/// surfaced as events only if the provider sent them; adapters decide what to
/// ignore.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
    event: Option<String>,
    data: String,
}

impl SseDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();

        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=position).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = String::from_utf8_lossy(&line).into_owned();
            self.process_line(&line, &mut events);
        }

        events
    }

    /// Flushes a trailing line and event that were not terminated by a newline
    /// or a blank line.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let line = String::from_utf8_lossy(&self.buffer).into_owned();
            self.buffer.clear();
            self.process_line(&line, &mut events);
        }
        if !self.data.is_empty() || self.event.is_some() {
            events.push(SseEvent {
                event: self.event.take(),
                data: std::mem::take(&mut self.data),
            });
        }
        events
    }

    fn process_line(&mut self, line: &str, events: &mut Vec<SseEvent>) {
        if line.is_empty() {
            if !self.data.is_empty() || self.event.is_some() {
                events.push(SseEvent {
                    event: self.event.take(),
                    data: std::mem::take(&mut self.data),
                });
            }
            return;
        }
        if line.starts_with(':') {
            return;
        }

        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut decoder = SseDecoder::new();
        let mut events = Vec::new();
        for chunk in chunks {
            events.extend(decoder.push(chunk));
        }
        events.extend(decoder.finish());
        events
    }

    #[test]
    fn parses_event_and_data() {
        let events = decode(&[b"event: message_start\ndata: {\"a\":1}\n\n"]);
        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("message_start".into()),
                data: "{\"a\":1}".into(),
            }]
        );
    }

    #[test]
    fn handles_chunk_boundaries_inside_utf8() {
        let text = "event: ping\ndata: привет\n\n";
        let bytes = text.as_bytes();
        let split = text.find('п').expect("cyrillic") + 1;
        let events = decode(&[&bytes[..split], &bytes[split..]]);
        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("ping".into()),
                data: "привет".into(),
            }]
        );
    }

    #[test]
    fn joins_multiline_data() {
        let events = decode(&[b"data: a\ndata: b\n\n"]);
        assert_eq!(events[0].data, "a\nb");
        assert_eq!(events[0].event, None);
    }

    #[test]
    fn ignores_comments_and_handles_crlf() {
        let events = decode(&[b": keep-alive\r\nevent: x\r\ndata: 1\r\n\r\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("x"));
        assert_eq!(events[0].data, "1");
    }

    #[test]
    fn flushes_unterminated_event() {
        let events = decode(&[b"data: tail"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "tail");
    }
}
