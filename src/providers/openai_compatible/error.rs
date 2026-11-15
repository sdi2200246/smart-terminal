use crate::core::ProviderError;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenAiError {
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

impl From<OpenAiError> for ProviderError {
    fn from(e: OpenAiError) -> Self {
        match e {
            OpenAiError::TokenLimit { .. } => ProviderError::TokenLimit {
                source: anyhow::anyhow!(e),
            },
            OpenAiError::InvalidToolCall { .. } => ProviderError::InvalidToolCall {
                source: anyhow::anyhow!(e),
            },
            OpenAiError::MalformedResponse { .. } => ProviderError::MalformedResponse {
                source: anyhow::anyhow!(e),
            },
            OpenAiError::UnexpectedOutput { .. } => ProviderError::MalformedResponse {
                source: anyhow::anyhow!(e),
            },
            _ => ProviderError::Protocol {
                source: anyhow::anyhow!(e),
            },
        }
    }
}
