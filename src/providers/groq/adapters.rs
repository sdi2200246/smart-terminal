use crate::core::{AgentRequest, ModelName};
use crate::providers::openai_compatible::{build_openai_request, OpenAiRequest};

pub fn to_groq_model_string(model: ModelName) -> String {
    match model {
        ModelName::GptOss120B => "openai/gpt-oss-120b".into(),
        ModelName::Llma3p18B => "llama-3.1-8b-instant".into(),
        _ => "openai/gpt-oss-120b".into(),
    }
}

impl From<&AgentRequest<'_>> for OpenAiRequest {
    fn from(request: &AgentRequest<'_>) -> Self {
        build_openai_request(request, to_groq_model_string(request.model.get_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm_client::AgentRequest;
    use crate::core::model::{Model, ModelName};
    use crate::core::session::{CoreAgentSession, Scratchpad};
    use serde_json::json;

    #[test]
    fn test_system_event_mapping() {
        let event = ConversationEvent::System("System prompt".into());
        let messages: Vec<Message> = (&event).into();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content, Some("System prompt".into()));
    }

    #[test]
    fn test_user_event_mapping() {
        let event = ConversationEvent::User("Hello".into());
        let messages: Vec<Message> = (&event).into();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, Some("Hello".into()));
    }

    #[test]
    fn test_tool_calls_event_mapping() {
        let calls = vec![
            CoreToolCall::new("get_weather", json!({"location": "Athens"}), "call_123"),
            CoreToolCall::new("get_time", json!({}), "call_456"),
        ];
        let event = ConversationEvent::ToolCalls(calls);
        let messages: Vec<Message> = (&event).into();

        assert_eq!(messages.len(), 1);

        let msg = &messages[0];
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.tool_calls.len(), 2);

        assert_eq!(msg.tool_calls[0].id, "call_123");
        assert_eq!(msg.tool_calls[0].function.name, "get_weather");
        assert_eq!(
            msg.tool_calls[0].function.arguments.as_deref(),
            Some("{\"location\":\"Athens\"}")
        );

        assert_eq!(msg.tool_calls[1].id, "call_456");
        assert_eq!(msg.tool_calls[1].function.name, "get_time");
        assert_eq!(msg.tool_calls[1].function.arguments.as_deref(), Some("{}"));
    }

    #[test]
    fn test_tool_results_event_mapping() {
        let results = vec![
            CoreToolResult::new("get_weather", "Sunny, 25C", "call_123"),
            CoreToolResult::new("get_time", "12:00 PM", "call_456"),
        ];
        let event = ConversationEvent::ToolResults(results);
        let messages: Vec<Message> = (&event).into();

        assert_eq!(messages.len(), 2);

        assert_eq!(messages[0].role, "tool");
        assert_eq!(messages[0].tool_call_id, Some("call_123".into()));
        assert_eq!(messages[0].name, Some("get_weather".into()));
        assert_eq!(messages[0].content, Some("Sunny, 25C".into()));

        assert_eq!(messages[1].role, "tool");
        assert_eq!(messages[1].tool_call_id, Some("call_456".into()));
        assert_eq!(messages[1].name, Some("get_time".into()));
        assert_eq!(messages[1].content, Some("12:00 PM".into()));
    }

    #[test]
    fn groq_request_includes_the_current_scratchpad() {
        let mut session = CoreAgentSession::new()
            .with_system("Investigator prompt")
            .with_user("continue investigation");
        session.scratchpad = Some(Scratchpad {
            main_goal: "answer question".into(),
            completed_milestones: vec!["inspect config".into()],
            current_focus: "check tool registration".into(),
            vital_findings: vec!["executor supports scratchpad".into()],
        });
        let model = Model::with_default_temp(ModelName::GptOss120B);
        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &[],
        };

        let groq_request = OpenAiRequest::from(&request);
        let scratchpad_message = &groq_request.messages[1];
        assert_eq!(scratchpad_message.role, "system");
        assert!(
            scratchpad_message
                .content
                .as_deref()
                .unwrap()
                .contains("check tool registration")
        );
    }
}
