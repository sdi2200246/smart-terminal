use serde_json::Value;
use super::error::AgentError;
pub enum AgentResponse {
    Success(Value),
    Error(AgentError),
}