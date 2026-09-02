use super::message::Message;
use super::tool::Tool;
use crate::google::adapters::to_gemini_schema;
use crate::core::session::{AgentSession , ConversationEvent};

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    #[serde(skip)]
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

impl GeminiRequest {
    pub fn structured(session: &AgentSession, schema: Value, model: String) -> Self {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for event in session.events.iter() {
            match event {
                ConversationEvent::System(msg) if system_instruction.is_none() => {
                    system_instruction = Some(Message::system(Some(msg.clone())));
                }
                other => contents.push(Message::from(other)),
            }
        }

        GeminiRequest {
            model,
            system_instruction,
            contents,
            tools: vec![],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.1),
                response_mime_type: Some("application/json".into()),
                response_schema: Some(to_gemini_schema(&schema)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::AgentSession;
    use serde_json::json;

    fn session_with(events: Vec<ConversationEvent>) -> AgentSession {
        AgentSession {
            events,
            steps: 5,
            final_answer: None,
        }
    }

    #[test]
    fn leading_system_becomes_system_instruction() {
        let session = session_with(vec![
            ConversationEvent::System("You are a helpful assistant.".into()),
            ConversationEvent::User("git sta".into()),
        ]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-flash-latest".into(),
        );

        let sys = req.system_instruction.expect("system_instruction present");
        assert_eq!(sys.parts[0].text, Some("You are a helpful assistant.".into()));
        assert_eq!(req.contents.len(), 1);
        assert_eq!(req.contents[0].role.as_deref(), Some("user"));
    }

    #[test]
    fn only_first_system_event_becomes_instruction() {
        // Matches current (non-refactored) behavior: only the FIRST System
        // event is promoted; any later System event falls into `contents`.
        let session = session_with(vec![
            ConversationEvent::System("first".into()),
            ConversationEvent::System("second".into()),
            ConversationEvent::User("hi".into()),
        ]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-flash-latest".into(),
        );

        let sys = req.system_instruction.expect("system_instruction present");
        assert_eq!(sys.parts[0].text, Some("first".into()));

        // "second" got demoted into contents via Message::from(System(..))
        assert_eq!(req.contents.len(), 2);
    }

    #[test]
    fn no_system_event_leaves_instruction_none() {
        let session = session_with(vec![ConversationEvent::User("hi".into())]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-flash-latest".into(),
        );

        assert!(req.system_instruction.is_none());
        assert_eq!(req.contents.len(), 1);
    }

    #[test]
    fn tools_is_always_empty() {
        let session = session_with(vec![ConversationEvent::User("hi".into())]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-flash-latest".into(),
        );

        assert!(req.tools.is_empty());
    }

    #[test]
    fn generation_config_forces_json_mode() {
        let session = session_with(vec![ConversationEvent::User("hi".into())]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-flash-latest".into(),
        );

        let config = req.generation_config.expect("config present");
        assert_eq!(config.temperature, Some(0.1));
        assert_eq!(config.response_mime_type, Some("application/json".into()));
        assert!(config.response_schema.is_some());
    }

    #[test]
    fn schema_is_converted_to_gemini_dialect() {
        let session = session_with(vec![ConversationEvent::User("hi".into())]);

        // Groq/schemars-style input: lowercase type, additionalProperties present.
        let schema = json!({
            "type": "object",
            "properties": { "cmd": { "type": "string" } },
            "additionalProperties": false
        });

        let req = GeminiRequest::structured(&session, schema, "gemini-flash-latest".into());
        let converted = req.generation_config.unwrap().response_schema.unwrap();

        assert_eq!(converted.get("type"), Some(&json!("OBJECT")));
        assert_eq!(
            converted["properties"]["cmd"].get("type"),
            Some(&json!("STRING"))
        );
        assert!(converted.get("additionalProperties").is_none());
    }

    #[test]
    fn model_is_carried_but_not_serialized() {
        let session = session_with(vec![ConversationEvent::User("hi".into())]);

        let req = GeminiRequest::structured(
            &session,
            json!({"type": "object"}),
            "gemini-pro-latest".into(),
        );
        assert_eq!(req.model, "gemini-pro-latest");

        // Confirms #[serde(skip)] on `model` — it must never hit the wire.
        let serialized = serde_json::to_value(&req).unwrap();
        assert!(serialized.get("model").is_none());
    }
}