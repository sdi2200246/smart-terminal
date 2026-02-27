use thiserror::Error;
use crate::contracts::error::DomainError;

#[derive(Debug, Error)]
pub enum GroqError {
    #[error("Token limit exceeded")]
    TokenLimit{
        #[source]
        source: anyhow::Error,
    },

    #[error("Invalid tool call from model")]
    InvalidToolCall{
        #[source]
        source: anyhow::Error,
    },

    #[error("Malformed model response")]
    MalformedResponse {
        #[source]
        source: anyhow::Error,
    },

    #[error("API request rejected")]
    Protocol{
        #[source]
        source: anyhow::Error,
    },

    #[error("HTTP transport error")]
    Http{
        #[source]
        source: anyhow::Error,
    },
}


impl From<GroqError> for DomainError {
    fn from(e: GroqError) -> Self {
        match e {
            GroqError::TokenLimit { source }        => DomainError::TokenLimit { source },
            GroqError::InvalidToolCall { source }   => DomainError::InvalidToolCal { source },
            GroqError::MalformedResponse { source } => DomainError::MalformedResponse { source },
            other                               => DomainError::Protocol { source: other.into() },
        }
    }
}