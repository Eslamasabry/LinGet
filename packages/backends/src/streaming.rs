use std::fmt;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Represents a line of output from a streaming command
#[derive(Debug, Clone)]
pub struct StreamLine {
    /// The text content of the line
    pub text: String,
    /// The source stream (stdout or stderr)
    pub stream: StreamType,
    /// Whether this line indicates an error
    pub is_error: bool,
    /// Optional timestamp
    pub timestamp: Option<std::time::Instant>,
}

impl StreamLine {
    pub fn new(text: String, stream: StreamType, is_error: bool) -> Self {
        Self {
            text,
            stream,
            is_error,
            timestamp: Some(std::time::Instant::now()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    Stdout,
    Stderr,
}

impl fmt::Display for StreamLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stream = match self.stream {
            StreamType::Stdout => "stdout",
            StreamType::Stderr => "stderr",
        };
        write!(f, "[{}] {}", stream, self.text)
    }
}
