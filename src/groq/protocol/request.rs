use serde::{Serialize};
use super::message::Message;
use super::tool::{Tool};

#[derive(Serialize , Debug)]
pub struct GroqRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub tool_choice: String,
    pub temperature :f32,
}
