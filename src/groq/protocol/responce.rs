use serde::{Deserialize};
use super::message::Message;

#[derive(Deserialize , Debug)]
pub struct GroqResponse {
    pub choices: Vec<Choice>,
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