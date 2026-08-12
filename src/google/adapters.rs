use super::api::message::Message;
use super::api::request::{GeminiRequest, GenerationConfig};
use super::api::tool::{FunctionDeclaration, Tool};
use crate::core::llm_client::AgentRequest;
use crate::core::model::ModelName;
use crate::core::session::ConversationEvent;
use serde_json::Value;

impl From<&ConversationEvent> for Message {
    fn from(event: &ConversationEvent) -> Message {
        match event {
            ConversationEvent::System(message) => Message::user(Some(message.clone())),
            ConversationEvent::User(message) => Message::user(Some(message.clone())),
            ConversationEvent::ToolResult { name, result, .. } => {
                Message::tool_responce(Some(result.clone()), name.clone())
            }
            ConversationEvent::ToolCall {
                name, arguments, ..
            } => Message::tool_call(name.clone(), arguments.clone()),
        }
    }
}

pub fn to_google_model_string(model: ModelName) -> String {
    match model {
        ModelName::Gemini1_5Pro => "gemini-pro-latest".into(),
        ModelName::Gemini1_5Flash => "gemini-flash-latest".into(),
        ModelName::Gemini2_5Flash => "gemini-flash-latest".into(),
        _ => "gemini-flash-latest".into(),
    }
}

pub fn to_gemini_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "additionalProperties" || k == "$schema" {
                    continue;
                }
                if k == "type" {
                    if let Value::String(t) = v {
                        out.insert(k.clone(), Value::String(t.to_uppercase()));
                        continue;
                    }
                }
                out.insert(k.clone(), to_gemini_schema(v));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(to_gemini_schema).collect()),
        other => other.clone(),
    }
}

impl From<&AgentRequest<'_>> for GeminiRequest {
    fn from(request: &AgentRequest<'_>) -> Self {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for event in request.session.events.iter() {
            match event {
                ConversationEvent::System(msg) if system_instruction.is_none() => {
                    system_instruction = Some(Message::system(Some(msg.clone())));
                }
                other => contents.push(Message::from(other)),
            }
        }

        let function_declarations: Vec<FunctionDeclaration> = request
            .tools_metadata
            .iter()
            .map(|t| FunctionDeclaration {
                name: t.name.clone(),
                description: Some(t.description.clone()),
                parameters: to_gemini_schema(&t.parameters),
            })
            .collect();

        GeminiRequest {
            model: to_google_model_string(request.model.get_name()),
            system_instruction,
            contents,
            tools: vec![Tool {
                function_declarations,
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(request.model.get_temp()),
                response_mime_type: None,
                response_schema: None,
            }),
        }
    }
}
