use serde_json::Value;
use super::responce::ChatResponse;
use super::mcp::service::McpError;


#[derive(Debug)]
pub enum ModelOutcome {
    Tool {
        name: String,
        id: String,
        arguments: Value,
    },
}

impl TryFrom<&ChatResponse> for ModelOutcome {
    type Error = McpError;

    fn try_from(response: &ChatResponse) -> Result<Self, Self::Error> {

        let choice = response
            .choices
            .get(0)
            .ok_or(McpError::Protocol)?;

        let message = &choice.message;

        if let Some(tool_call) = message.tool_calls.get(0) {
            return Ok(ModelOutcome::Tool {
                name: tool_call.function.name.clone(),
                id: tool_call.id.clone(),
                arguments: tool_call.function.arguments(),
            });
        }
        Err(McpError::Protocol)
    }
}