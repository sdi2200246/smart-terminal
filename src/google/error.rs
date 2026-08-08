use crate::core::error::ProviderError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoogleError {
    #[error("Token limit exceeded")]
    TokenLimit {
        #[source]
        source: anyhow::Error,
    },

    #[error("Invalid tool call from model")]
    InvalidToolCall {
        #[source]
        source: anyhow::Error,
    },

    #[error("Malformed model response")]
    MalformedResponse {
        #[source]
        source: anyhow::Error,
    },

    #[error("API request rejected")]
    Protocol {
        #[source]
        source: anyhow::Error,
    },

    #[error("HTTP transport error")]
    Http {
        #[source]
        source: anyhow::Error,
    },
}

impl From<GoogleError> for ProviderError {
    fn from(e: GoogleError) -> Self {
        match e {
            GoogleError::TokenLimit { source } => ProviderError::TokenLimit { source },
            GoogleError::InvalidToolCall { source } => ProviderError::InvalidToolCal { source },
            GoogleError::MalformedResponse { source } => ProviderError::MalformedResponse { source },
            other => ProviderError::Protocol {
                source: other.into(),
            },
        }
    }
}
