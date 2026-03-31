use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Token limit exceeded")]
    TokenLimit{
        #[source]
        source: anyhow::Error,
    },

    #[error("Invalid tool call from model")]
    InvalidToolCal{
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
}

#[derive(Debug, Error)]
pub enum InternalError{
    #[error("Tool execution failed")]
    Tool {
        #[source]
        source: anyhow::Error,
    },

    #[error("Session initialization failed")]
    SessionInit {
        #[source]
        source: anyhow::Error,
    },

}