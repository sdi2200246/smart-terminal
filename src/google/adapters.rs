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
            ConversationEvent::ToolResult { name, result, id,thinking_state } => {
                Message::tool_responce(Some(result.clone()), name.clone() , thinking_state.clone())
            }
            ConversationEvent::ToolCall {
                name, arguments, id , thinking_state
            } => Message::tool_call(name.clone(), arguments.clone() , thinking_state.clone()),
        }
    }
}

pub fn to_google_model_string(model: ModelName) -> String {
    match model {
        ModelName::Gemini1_5Pro => "gemini-3.6-flash".into(),
        ModelName::Gemini1_5Flash => "gemini-3.6-flash".into(),
        ModelName::Gemini2_5Flash => "gemini-3.6-flash".into(),
        _ => "gemini-3.6-flash".into(),
    }
}

pub fn to_gemini_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "additionalProperties" || k == "$schema" || k == "title" {
                    continue;
                }

                if k == "type" {
                    match v {
                        Value::String(t) => {
                            let valid_types = ["string", "number", "integer", "boolean", "array", "object", "null"];
                            if valid_types.contains(&t.as_str()) {
                                out.insert(k.clone(), Value::String(t.to_uppercase()));
                            } else {
                                out.insert(k.clone(), v.clone());
                            }
                            continue;
                        }
                        Value::Array(arr) => {
                            if let Some(Value::String(t)) = arr.iter().find(|x| {
                                x.as_str().map_or(false, |s| s != "null")
                            }) {
                                out.insert(k.clone(), Value::String(t.to_uppercase()));
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                // Recursively process the rest of the tree
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



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_type_uppercasing() {
        let input = json!({
            "type": "string"
        });
        let expected = json!({
            "type": "STRING"
        });
        assert_eq!(to_gemini_schema(&input), expected);
    }

    #[test]
    fn test_non_standard_type_preserved() {
        // Custom string values (e.g., inside 'default' or user payloads) shouldn't be touched
        let input = json!({
            "type": "custom_user_value"
        });
        let expected = json!({
            "type": "custom_user_value"
        });
        assert_eq!(to_gemini_schema(&input), expected);
    }

    #[test]
    fn test_nullable_option_array_type() {
        // Handles schemars/JSON Schema representation for Option<T>: ["string", "null"] -> "STRING"
        let input = json!({
            "type": ["string", "null"]
        });
        let expected = json!({
            "type": "STRING"
        });
        assert_eq!(to_gemini_schema(&input), expected);
    }

    #[test]
    fn test_stripping_unsupported_gemini_keys() {
        let input = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "UserConfig",
            "type": "object",
            "additionalProperties": false
        });
        let expected = json!({
            "type": "OBJECT"
        });
        assert_eq!(to_gemini_schema(&input), expected);
    }

    #[test]
    fn test_nested_object_and_array_schema() {
        let input = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "ToolInput",
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "title": "File Path"
                },
                "max_lines": {
                    "type": ["integer", "null"]
                },
                "flags": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "additionalProperties": false
        });

        let expected = json!({
            "type": "OBJECT",
            "properties": {
                "file_path": {
                    "type": "STRING"
                },
                "max_lines": {
                    "type": "INTEGER"
                },
                "flags": {
                    "type": "ARRAY",
                    "items": {
                        "type": "STRING"
                    }
                }
            }
        });

        assert_eq!(to_gemini_schema(&input), expected);
    }
}