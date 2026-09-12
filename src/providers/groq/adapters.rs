use super::protocol::message::Message;
use super::protocol::request::GroqRequest;
use super::protocol::tool::{Tool, ToolCall, ToolMetaData};
use crate::core::llm_client::AgentRequest;
use crate::core::model::ModelName;
use crate::core::session::{ConversationEvent, ToolCall as CoreToolCall, ToolResult as CoreToolResult};
use crate::core::capability::{ToolMetaData as CoreToolMetaData};
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
            
            ConversationEvent::ToolCalls(calls) => {
                return vec![Message::tool_calls(calls.iter().map(ToolCall::from).collect())]
            }
            ConversationEvent::ToolResults(results) => {
                results.iter().map(Message::from).collect()
            }
        }
    }
}

pub fn to_groq_model_string(model: ModelName) -> String {
    match model {
        ModelName::GptOss120B => "openai/gpt-oss-120b".into(),
        ModelName::Llma3p18B => "llama-3.1-8b-instant".into(),
        _ => "openai/gpt-oss-120b".into(),
    }
}

impl From<&AgentRequest<'_>> for GroqRequest {
    fn from(request: &AgentRequest<'_>) -> Self {
        let messages = request
            .session
            .events
            .iter()
            .flat_map(Vec::<Message>::from)
            .collect();

        let tools: Vec<Tool> = request
            .tools_metadata
            .iter()
            .map(Tool::from)
            .collect();

        GroqRequest {
            model: to_groq_model_string(request.model.get_name()),
            messages,
            tools, 
            temperature: request.model.get_temp(),
            tool_choice: None,
            response_format: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(msg.tool_calls[0].function.arguments.as_deref(), Some("{\"location\":\"Athens\"}"));
        
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
}