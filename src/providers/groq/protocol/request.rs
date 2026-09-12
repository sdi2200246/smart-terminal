use super::message::Message;
use super::tool::Tool;
use crate::core::session::AgentSession;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug, Default)]
pub struct GroqRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl GroqRequest {
    pub fn structured(session: &AgentSession, schema: Value) -> Self {
        // Use flat_map to unpack the Vec<Message> returned by the adapter
        let messages = session
            .events
            .iter()
            .flat_map(|e| Vec::<Message>::from(e))
            .collect();

        GroqRequest {
            model: "openai/gpt-oss-120b".into(),
            messages,
            temperature: 0.1,
            tools: vec![],
            tool_choice: None,
            response_format: Some(ResponseFormat::json_schema("output", schema)),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ResponseFormat {
    pub r#type: String,
    pub json_schema: JsonSchemaSpec,
}

#[derive(Serialize, Debug)]
pub struct JsonSchemaSpec {
    pub name: String,
    pub strict: bool,
    pub schema: Value,
}

impl ResponseFormat {
    pub fn json_schema(name: impl Into<String>, schema: Value) -> Self {
        ResponseFormat {
            r#type: "json_schema".into(),
            json_schema: JsonSchemaSpec {
                name: name.into(),
                strict: true,
                schema,
            },
        }
    }
}
