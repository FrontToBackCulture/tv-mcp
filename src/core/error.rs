// Typed error for Tauri commands
//
// Replaces `Result<T, String>` with `Result<T, CommandError>`.
// Implements Serialize (required by Tauri v2) and From<X> for common error types
// so `?` works without manual `.map_err(|e| format!(...))`.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Network(String),

    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },

    #[error("{0}")]
    Parse(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Config(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Internal(String),
}

// Tauri v2 requires the error type to implement Serialize.
// We serialize as a structured object so the frontend can inspect error codes.
impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let (code, message) = match self {
            CommandError::Network(msg) => ("network", msg.as_str()),
            CommandError::Http { status, body } => {
                // For Http, we format inline since we can't return a reference to a temp
                let msg = format!("HTTP {}: {}", status, body);
                let mut s = serializer.serialize_struct("CommandError", 2)?;
                s.serialize_field("code", "http")?;
                s.serialize_field("message", &msg)?;
                return s.end();
            }
            CommandError::Parse(msg) => ("parse", msg.as_str()),
            CommandError::NotFound(msg) => ("not_found", msg.as_str()),
            CommandError::Config(msg) => ("config", msg.as_str()),
            CommandError::Io(msg) => ("io", msg.as_str()),
            CommandError::Internal(msg) => ("internal", msg.as_str()),
        };
        let mut s = serializer.serialize_struct("CommandError", 2)?;
        s.serialize_field("code", code)?;
        s.serialize_field("message", message)?;
        s.end()
    }
}

// -- From impls for `?` ergonomics --

impl From<reqwest::Error> for CommandError {
    fn from(e: reqwest::Error) -> Self {
        CommandError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(e: serde_json::Error) -> Self {
        CommandError::Parse(e.to_string())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::Io(e.to_string())
    }
}

/// Convert from legacy String errors (allows incremental migration)
impl From<String> for CommandError {
    fn from(s: String) -> Self {
        CommandError::Internal(s)
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        CommandError::Internal(s.to_string())
    }
}

/// Allow CommandError to convert back to String for incremental migration.
/// Functions still returning Result<T, String> can use `?` on CmdResult values.
impl From<CommandError> for String {
    fn from(e: CommandError) -> Self {
        e.to_string()
    }
}

/// Type alias for commands — drop-in replacement for Result<T, String>
pub type CmdResult<T> = Result<T, CommandError>;
