use thiserror::Error;
use crate::interfaces::error::InternalError;

#[derive(Debug, Error)]
pub enum ToolError{

 #[error("Invallid arguments.")]
    ArgumentsParsing{
        #[source]
        source: anyhow::Error,
    },

    #[error("Tool execution failed.")]
    ToolExecution{
        #[source]
        source: anyhow::Error,
    },
}

impl From<ToolError> for InternalError{

    fn from(e:ToolError)->InternalError{
        match e {
            ToolError::ArgumentsParsing { source } => InternalError::Tool{source},
            ToolError::ToolExecution { source } => InternalError::Tool {source}
        }
    }
}