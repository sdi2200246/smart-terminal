use super::protocol::message::Message;
use super::protocol::request::{OpenAiRequest, ResponseFormat};
use super::protocol::tool::{Tool, ToolCall, ToolMetaData};
use crate::core::{
    AgentRequest, ConversationEvent, CoreAgentSession, ToolCall as CoreToolCall,
    ToolMetaData as CoreToolMetaData, ToolResult as CoreToolResult,
};
use serde_json::Value;

impl From<&CoreToolMetaData> for ToolMetaData {
    fn from(core_meta: &CoreToolMetaData) -> Self {
        ToolMetaData {
            name: core_meta.name.clone(),
            description: Some(core_meta.description.clone()),
            parameters: core_meta.parameters.clone(),
            arguments: None,
        }
    }
}

impl From<&CoreToolMetaData> for Tool {
    fn from(core_meta: &CoreToolMetaData) -> Self {
        Tool::factory(ToolMetaData::from(core_meta))
    }
}

impl From<&CoreToolCall> for ToolCall {
    fn from(core_call: &CoreToolCall) -> Self {
        ToolCall {
            id: core_call.id.clone(),
            call_type: "function".to_string(),
            function: ToolMetaData {
                name: core_call.name.clone(),
                description: None,
                parameters: Value::Null,
                arguments: Some(core_call.arguments.to_string()),
            },
        }
    }
}

impl From<&CoreToolResult> for Message {
    fn from(core_res: &CoreToolResult) -> Self {
        Message::tool_responce(
            Some(core_res.result.clone()),
            core_res.id.clone(),
            core_res.name.clone(),
        )
    }
}

impl From<&ConversationEvent> for Vec<Message> {
    fn from(event: &ConversationEvent) -> Vec<Message> {
        match event {
            ConversationEvent::System(message) => vec![Message::system(Some(message.clone()))],

            ConversationEvent::User(message) => vec![Message::user(Some(message.clone()))],

            ConversationEvent::Assistant(message) => vec![Message {
                role: "assistant".into(),
                content: Some(message.clone()),
                tool_calls: vec![],
                tool_call_id: None,
                name: None,
            }],

            ConversationEvent::ToolCalls(calls) => {
                vec![Message::tool_calls(
                    calls.iter().map(ToolCall::from).collect(),
                )]
            }
            ConversationEvent::ToolResults(results) => results.iter().map(Message::from).collect(),
        }
    }
}

pub fn build_messages_with_scratchpad(session: &CoreAgentSession) -> Vec<Message> {
    let mut messages: Vec<Message> = session
        .events
        .iter()
        .flat_map(Vec::<Message>::from)
        .collect();

    if let Some(scratchpad) = &session.scratchpad {
        let scratchpad_message = Message::system(Some(format!(
            "Current scratchpad:\n{}",
            serde_json::to_string_pretty(scratchpad).expect("scratchpad serializes to JSON")
        )));
        let position = messages
            .iter()
            .position(|message| message.role != "system")
            .unwrap_or(messages.len());
        messages.insert(position, scratchpad_message);
    }

    messages
}

pub fn build_tools(tools_metadata: &[CoreToolMetaData]) -> Vec<Tool> {
    tools_metadata.iter().map(Tool::from).collect()
}

pub fn build_openai_request(request: &AgentRequest<'_>, model: impl Into<String>) -> OpenAiRequest {
    let messages = build_messages_with_scratchpad(request.session);
    let tools = build_tools(request.tools_metadata);

    OpenAiRequest {
        model: model.into(),
        messages,
        tools,
        temperature: request.model.get_temp(),
        tool_choice: None,
        response_format: None,
    }
}

pub fn build_openai_structured_request(
    session: &CoreAgentSession,
    schema: Value,
    model: impl Into<String>,
) -> OpenAiRequest {
    let messages = session
        .events
        .iter()
        .flat_map(Vec::<Message>::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AgentRequest, CoreAgentSession, Model, ModelName, Scratchpad};
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
    fn openai_request_includes_the_current_scratchpad() {
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

        let req = build_openai_request(&request, "test-model");
        let scratchpad_message = &req.messages[1];
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
