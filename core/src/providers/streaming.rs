//! where: iclaw/core/src/providers/streaming.rs | what: streaming provider types | why: runtimes need provider-agnostic chunk and error primitives

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub is_final: bool,
    pub token_count: usize,
}

impl StreamChunk {
    pub fn delta(text: impl Into<String>) -> Self {
        Self {
            delta: text.into(),
            is_final: false,
            token_count: 0,
        }
    }

    pub fn final_chunk() -> Self {
        Self {
            delta: String::new(),
            is_final: true,
            token_count: 0,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            delta: message.into(),
            is_final: true,
            token_count: 0,
        }
    }

    pub fn with_token_estimate(mut self) -> Self {
        self.token_count = self.delta.len().div_ceil(4);
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamOptions {
    pub enabled: bool,
    pub count_tokens: bool,
}

impl StreamOptions {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            count_tokens: false,
        }
    }

    pub fn with_token_count(mut self) -> Self {
        self.count_tokens = true;
        self
    }
}

pub type StreamResult<T> = std::result::Result<T, StreamError>;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("JSON parse error: {0}")]
    Json(serde_json::Error),
    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_is_non_zero_for_text() {
        let chunk = StreamChunk::delta("hello-world").with_token_estimate();
        assert!(chunk.token_count > 0);
        assert!(!chunk.is_final);
    }
}
