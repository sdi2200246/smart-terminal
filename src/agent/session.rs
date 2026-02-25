use crate::agent::tools::capability::ToolFunction;
use serde_json::Value;

#[derive(Debug , PartialEq)]
pub enum ConversationEvent {
    System(String),
    User(String),
    ToolCall {
        name: String,
        arguments: Value,
        id: String,
    },
    ToolResult {
        name: String,
        result: String,
        id: String,
    },
}

#[derive(Debug)]
pub struct AgentSession{
    pub events:Vec<ConversationEvent>,
    pub available_tools:Vec<ToolFunction>,
    pub steps:usize,
}
