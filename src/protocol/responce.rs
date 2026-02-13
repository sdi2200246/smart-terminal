use serde::{Deserialize};
use super::message::Message;

#[derive(Deserialize , Debug)]
pub enum ProtocolError{
    ToolCall
}

#[derive(Deserialize , Debug)]
pub struct ChatResponse {
     pub choices: Vec<Choice>,
}
impl ChatResponse{ 

    pub fn tool_call_name(&self) -> Result<&str, ProtocolError> {
        let choice = self.choices.get(0).ok_or(ProtocolError::ToolCall)?;

        let tool = choice.message.tool_calls.get(0)
            .ok_or(ProtocolError::ToolCall)?;
        Ok(&tool.name)
    }
    pub fn tool_call_id(&self)->&str{
        return  &self.choices[0].message.tool_calls[0].id;
    }
}


#[derive(Deserialize , Debug)]
pub struct Choice {
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub logprobs: Option<serde_json::Value>,
    pub message: Message,

} 