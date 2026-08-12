use crate::core::error::ProviderError;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GroqError {
    #[error("Request timed out")]
    Timeout {
        #[source]
        source: reqwest::Error,
    },

    #[error("Network transport error: {source}")]
    Network {
        #[source]
        source: reqwest::Error,
    },

    #[error("Token limit exceeded: {body}")]
    TokenLimit { body: String },

    #[error("Invalid tool call from model: {body}")]
    InvalidToolCall { body: String },

    #[error("Malformed model response: {source}")]
    MalformedResponse {
        #[source]
        source: reqwest::Error,
    },

    #[error("Unexpected model output: {body}")]
    UnexpectedOutput { body: String },

    #[error("API request rejected (Status {status}): {body}")]
    Protocol { status: StatusCode, body: String },
}

impl From<GroqError> for ProviderError {
    fn from(e: GroqError) -> Self {
        match e {
            GroqError::TokenLimit { .. } => ProviderError::TokenLimit {
                source: anyhow::anyhow!(e),
            },
            GroqError::InvalidToolCall { .. } => ProviderError::InvalidToolCall {
                source: anyhow::anyhow!(e),
            },
            GroqError::MalformedResponse { .. } => ProviderError::MalformedResponse {
                source: anyhow::anyhow!(e),
            },
            _ => ProviderError::Protocol {
                source: anyhow::anyhow!(e),
            },
        }
    }
}
