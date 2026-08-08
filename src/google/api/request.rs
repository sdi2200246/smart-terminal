use super::message::Message;
use super::tool::Tool;
// use crate::core::session::AgentSession;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub model:String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Message>,
    pub contents: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<Value>,
}

// impl GeminiRequest {
//     pub fn structured(session: &AgentSession, schema: Value) -> Self {
//         let contents = session.events.iter().map(Message::from).collect();
//         GeminiRequest {
//             system_instruction: None,
//             contents,
//             generation_config: Some(GenerationConfig {
//                 temperature: Some(0.1),
//                 response_mime_type: Some("application/json".into()),
//                 response_schema: Some(schema),
//             }),
//             tools: vec![],
//         }
//     }
// }