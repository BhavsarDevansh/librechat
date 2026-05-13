//! Custom SSE (Server-Sent Events) parser for streaming LLM responses.
//!
//! Provides [`SseParser`] which buffers incoming text chunks and yields
//! complete SSE events. Handles line splitting, multi-line `data:` fields,
//! and event boundaries.

/// A single parsed SSE event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The event type (defaults to `"message"`).
    pub event_type: String,
    /// The concatenated data payload.
    pub data: String,
}

/// SSE parser that buffers incoming text and emits complete events.
///
/// Feed raw chunks from the HTTP response body via [`SseParser::feed`].
/// The parser splits on line boundaries, accumulates `data:` lines, and
/// returns a [`Vec<SseEvent>`] every time a blank line signals the end
/// of an event.
#[derive(Debug, Clone, Default)]
pub struct SseParser {
    buffer: String,
    pending_event_type: String,
    pending_data: String,
}

impl SseParser {
    /// Create a new empty SSE parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a raw text chunk into the parser.
    ///
    /// Returns all complete SSE events found in the buffer after processing
    /// the new chunk. Incomplete lines are retained in the internal buffer
    /// until the next call or until the stream ends.
    pub fn feed(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer.drain(..=pos).collect::<String>();
            let line = line.strip_suffix('\n').unwrap_or(&line);
            let line = line.strip_suffix('\r').unwrap_or(line);

            if line.is_empty() {
                if !self.pending_data.is_empty() || !self.pending_event_type.is_empty() {
                    events.push(SseEvent {
                        event_type: if self.pending_event_type.is_empty() {
                            "message".to_string()
                        } else {
                            std::mem::take(&mut self.pending_event_type)
                        },
                        data: std::mem::take(&mut self.pending_data),
                    });
                }
                continue;
            }

            if let Some(comment) = line.strip_prefix(':') {
                let _ = comment;
                continue;
            }

            if let Some(data) = line.strip_prefix("data") {
                let data = data.strip_prefix(':').unwrap_or(data);
                let data = data.strip_prefix(' ').unwrap_or(data);
                if !self.pending_data.is_empty() {
                    self.pending_data.push('\n');
                }
                self.pending_data.push_str(data);
                continue;
            }

            if let Some(event) = line.strip_prefix("event") {
                let event = event.strip_prefix(':').unwrap_or(event);
                let event = event.strip_prefix(' ').unwrap_or(event);
                self.pending_event_type = event.to_string();
                continue;
            }
        }

        events
    }

    /// Drain any remaining event data as a final event.
    ///
    /// Call this when the underlying stream has ended to ensure the last
    /// event is returned even if the server omitted the trailing blank line.
    pub fn finalize(mut self) -> Option<SseEvent> {
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            let line = line.strip_suffix('\r').unwrap_or(&line);

            if let Some(data) = line.strip_prefix("data: ") {
                if !self.pending_data.is_empty() {
                    self.pending_data.push('\n');
                }
                self.pending_data.push_str(data);
            } else if let Some(event) = line.strip_prefix("event: ") {
                self.pending_event_type = event.to_string();
            }
        }

        if !self.pending_data.is_empty() || !self.pending_event_type.is_empty() {
            Some(SseEvent {
                event_type: if self.pending_event_type.is_empty() {
                    "message".to_string()
                } else {
                    self.pending_event_type
                },
                data: self.pending_data,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_data_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[0].event_type, "message");
    }

    #[test]
    fn test_parse_done_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: [DONE]\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "[DONE]");
    }

    #[test]
    fn test_parse_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: first\n\ndata: second\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_parse_across_chunk_boundaries() {
        let mut parser = SseParser::new();
        let first = parser.feed("data: hel");
        assert!(first.is_empty());

        let second = parser.feed("lo\n\n");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].data, "hello");
    }

    #[test]
    fn test_parse_multiline_data() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: line1\ndata: line2\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn test_ignores_comments() {
        let mut parser = SseParser::new();
        let events = parser.feed(": comment\ndata: real\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "real");
    }

    #[test]
    fn test_custom_event_type() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: error\ndata: oops\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "error");
        assert_eq!(events[0].data, "oops");
    }

    #[test]
    fn test_crlf_line_endings() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: hello\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_finalize_returns_pending_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: no trailing newline");
        assert!(events.is_empty());

        let final_event = parser.finalize();
        assert!(final_event.is_some());
        assert_eq!(final_event.unwrap().data, "no trailing newline");
    }

    #[test]
    fn test_finalize_returns_none_when_empty() {
        let parser = SseParser::new();
        assert!(parser.finalize().is_none());
    }

    #[test]
    fn test_json_data_parsing() {
        let mut parser = SseParser::new();
        let json = r#"{"id":"1","model":"test","choices":[]}"#;
        let events = parser.feed(&format!("data: {json}\n\n"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, json);
    }

    #[test]
    fn test_data_without_space() {
        let mut parser = SseParser::new();
        let events = parser.feed("data:hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_event_without_space() {
        let mut parser = SseParser::new();
        let events = parser.feed("event:error\ndata:oops\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "error");
        assert_eq!(events[0].data, "oops");
    }
}
