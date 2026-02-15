use serde::{Serialize , Deserialize};
use serde_json::Value;
use super::service::AgentError;

#[derive(Serialize , Deserialize , PartialEq)]
pub enum AgentResponse {
    Success(Value),
    Error(AgentError),
}