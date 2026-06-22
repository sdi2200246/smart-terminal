use crate::core::error::{InternalError, ProviderError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Infrastructure error")]
    Internal(#[from] InternalError),

    #[error("Domain error")]
    Provider(#[from] ProviderError),

    #[error("Agent exhausted all steps without reaching a final answer")]
    StepsExhausted,

    #[error("Contract schema is invalid: {0}")]
    InvalidContract(String),

    #[error("Final answer does not satisfy the expected schema")]
    ScheemaViolation,
}
