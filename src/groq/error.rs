use thiserror::Error;
use crate::interfaces::error::ProviderError;

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


impl From<GroqError> for ProviderError {
    fn from(e: GroqError) -> Self {
        match e {
            GroqError::TokenLimit { source }        => ProviderError::TokenLimit { source },
            GroqError::InvalidToolCall { source }   => ProviderError::InvalidToolCal { source },
            GroqError::MalformedResponse { source } => ProviderError::MalformedResponse { source },
            other                               => ProviderError::Protocol { source: other.into() },
        }
    }
}