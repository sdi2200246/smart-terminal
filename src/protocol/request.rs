use serde::{Serialize};

use super::message::Message;
use super::tool::Tool;
use super::agent::session::AgentSession;

#[derive(Serialize , Debug)]
pub struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<String>,
    temperature :f32,
}

impl From<&AgentSession> for ChatRequest {

    fn from(mcp_session:&AgentSession)->ChatRequest{
        ChatRequest { 
            model:mcp_session.model.clone(),
            messages:mcp_session.messages.clone(),
            tools:Some(mcp_session.tools.clone()),
            tool_choice:Some("required".to_string()),
            temperature:0.7
        }
    }
}