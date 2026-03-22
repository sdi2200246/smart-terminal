use crate::interfaces::session::{AgentSession, ConversationEvent};
use super::protocol:: message::Message;
use super::protocol::tool::{self,Tool};
use super::protocol::request::GroqRequest;

impl From<&ConversationEvent> for Message{
    fn from(event:&ConversationEvent)->Message{
        match event{
            ConversationEvent::System(message)=> Message::system(Some(message.clone())),
            ConversationEvent::User(message                                     )=> Message::user(Some(message.clone())),
            ConversationEvent::ToolResult { name, result, id } => Message::tool_responce(Some(result.clone()), id.clone(), name.clone()),
            ConversationEvent::ToolCall { name, arguments, id } => Message::tool_call(name.clone(), id.clone(), arguments.clone()),
        }
    }
}

impl From<&AgentSession> for GroqRequest {
    fn from(session: &AgentSession) -> Self{
        let messages = session.events.iter()
            .map(|e| Message::from(e))
            .collect();

        let tools = session.available_tools.iter()
            .map(|t|{
                Tool{
                    r#type:"function".into(),
                    function:tool::ToolFunction{
                            name:t.name.clone(),
                            description:Some(t.description.clone()),
                            parameters:t.parameters.clone(),
                            arguments:None,
                        }
                }
            })
            .collect();
        
        GroqRequest{
            model:"openai/gpt-oss-120b".into(),
            messages,
            tools,
            tool_choice:"auto".into(),
            temperature:0.5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json};
    use crate::interfaces::capability::ToolFunction;

    // ---------- SYSTEM ----------
    #[test]
    fn system_event_to_message_json() {
        let event = ConversationEvent::System("config".into());

        let msg: Message = (&event).into();
        let serialized = serde_json::to_value(&msg).unwrap();

        let expected = json!({
            "role": "system",
            "content": "config"
        });

        assert_eq!(serialized, expected);
    }

    // ---------- USER ----------
    #[test]
    fn user_event_to_message_json() {
        let event = ConversationEvent::User("hello".into());

        let msg: Message = (&event).into();
        let serialized = serde_json::to_value(&msg).unwrap();

        let expected = json!({
            "role": "user",
            "content": "hello"
        });

        assert_eq!(serialized, expected);
    }

    // ---------- TOOL CALL ----------
    #[test]
    fn tool_call_event_to_message_json() {
        let args = json!({
            "location": "San Francisco, CA",
            "unit": "fahrenheit"
        });

        let event = ConversationEvent::ToolCall {
            name: "get_weather".into(),
            arguments: args.clone(),
            id: "call_abc123".into(),
        };

        let msg: Message = (&event).into();
        let serialized = serde_json::to_value(&msg).unwrap();

        let expected = json!({
            "role": "assistant",
            "tool_calls": [{
                "id": "call_abc123",
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "arguments": args.to_string()
                }
            }]
        });

        assert_eq!(serialized, expected);
    }

    #[test]
    fn tool_result_event_to_message_json() {
        let event = ConversationEvent::ToolResult {
            name: "get_weather".into(),
            result: "72°F".into(),
            id: "call_abc123".into(),
        };

        let msg: Message = (&event).into();
        let serialized = serde_json::to_value(&msg).unwrap();

        let expected = json!({
            "role": "tool",
            "content": "72°F",
            "tool_call_id": "call_abc123",
            "name": "get_weather"
        });

        assert_eq!(serialized, expected);
    }

      #[test]
    fn session_maps_to_groq_request_correctly() {
        // ---- Arrange ----
        let session = AgentSession {
            events: vec![
                ConversationEvent::System("sys".into()),
                ConversationEvent::User("hello".into()),
                ConversationEvent::ToolCall {
                    name: "get_weather".into(),
                    arguments: json!({"a": 1}),
                    id: "123".into(),
                },
            ],
            available_tools: vec![
                ToolFunction {
                    name: "get_weather".into(),
                    description: "gets weather".into(),
                    parameters: json!({"type": "object"}),
                }
            ],
            // add other fields if needed
            steps:5
        };

        // ---- Act ----
        let req = GroqRequest::from(&session);
        // ---- Assert: Messages mapped ----
        assert_eq!(req.messages.len(), 3);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[1].role, "user");

        // Tool call message
        let tool_msg = &req.messages[2];
        assert_eq!(tool_msg.role, "assistant");
        assert_eq!(tool_msg.tool_calls.len(), 1);

        // ---- Assert: Tool definitions mapped ----
        assert_eq!(req.tools.len(), 1);

        let tool = &req.tools[0];
        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "get_weather");
        assert_eq!(tool.function.description, Some("gets weather".into()));
        assert_eq!(tool.function.parameters, json!({"type": "object"}));
        assert!(tool.function.arguments.is_none());
        println!("{:?}", req);
    }

   
}