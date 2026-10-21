use std::fmt::{Display, Formatter};

/// The object file cannot be executed (malformed or references unknown functions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError(pub String);

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid object file: {}", self.0)
    }
}

impl std::error::Error for LoadError {}

/// An exception that was not handled by any `Попытка`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
    /// Source file name from the debug info.
    pub source: Option<String>,
    pub line: Option<u32>,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (&self.source, self.line) {
            (Some(source), Some(line)) => write!(f, "{source}:{line}: {}", self.message),
            (None, Some(line)) => write!(f, "line {line}: {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for RuntimeError {}
