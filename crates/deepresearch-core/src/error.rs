use std::{fmt, path::PathBuf};

use thiserror::Error;

/// Core error type for DeepResearch.
#[derive(Debug, Error)]
pub enum DeepResearchError {
    #[error("configuration error: {0}")]
    InvalidConfiguration(String),
    #[error("missing environment variable: {0}")]
    MissingSecret(String),
    #[error("I/O error while reading {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl DeepResearchError {
    pub fn config_io(path: PathBuf, source: std::io::Error) -> Self {
        Self::ConfigIo { path, source }
    }
}

/// Error representing a task failure within the orchestration graph.
#[derive(Debug, Clone)]
pub struct TaskError {
    pub reason: String,
    pub retryable: bool,
}

impl TaskError {
    pub fn new(reason: impl Into<String>, retryable: bool) -> Self {
        Self {
            reason: reason.into(),
            retryable,
        }
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let retry = if self.retryable {
            "retryable"
        } else {
            "terminal"
        };
        write!(f, "{retry} task failure: {}", self.reason)
    }
}

impl std::error::Error for TaskError {}
