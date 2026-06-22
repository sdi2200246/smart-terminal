use crate::core::error::InternalError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("[ERROR]Invallid arguments:{source}]")]
    ArgumentsParsing {
        #[source]
        source: anyhow::Error,
    },

    #[error("[Error][Tool execution failed: {source}]")]
    ToolExecution {
        #[source]
        source: anyhow::Error,
    },
}

impl From<ToolError> for InternalError {
    fn from(e: ToolError) -> InternalError {
        match e {
            ToolError::ArgumentsParsing { source } => InternalError::Tool { source },
            ToolError::ToolExecution { source } => InternalError::Tool { source },
        }
    }
}
