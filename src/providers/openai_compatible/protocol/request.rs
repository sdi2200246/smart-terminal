use super::message::Message;
use super::tool::Tool;
use crate::core::AgentSession;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl OpenAiRequest {
    pub fn structured(session: &AgentSession, schema: Value) -> Self {
        Self::structured_with_model(session, schema, "openai/gpt-oss-120b")
    }

    pub fn structured_with_model(
        session: &AgentSession,
        schema: Value,
        model: impl Into<String>,
    ) -> Self {
        let messages = session
            .events
            .iter()
            .flat_map(|e| Vec::<Message>::from(e))
            .collect();

        OpenAiRequest {
            model: model.into(),
            messages,
            temperature: 0.1,
            tools: vec![],
            tool_choice: None,
            response_format: Some(ResponseFormat::json_schema("output", schema)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ResponseFormat {
    pub r#type: String,
    pub json_schema: JsonSchemaSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
