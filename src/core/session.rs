use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub id: String,
    pub thinking_state: Option<String>,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, arguments: Value, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments,
            id: id.into(),
            thinking_state: None,
        }
    }

    pub fn with_thinking_state(mut self, state: Option<String>) -> Self {
        self.thinking_state = state;
        self
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub result: String,
    pub id: String,
}

impl ToolResult {
    pub fn new(name: impl Into<String>, result: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            result: result.into(),
            id: id.into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ConversationEvent {
    System(String),
    User(String),
    Assistant(String),
    ToolCalls(Vec<ToolCall>),
    ToolResults(Vec<ToolResult>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct Scratchpad {
    pub main_goal: String,
    pub completed_milestones: Vec<String>,
    pub current_focus: String,
    pub vital_findings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CoreAgentSession {
    #[serde(default)]
    pub events: Vec<ConversationEvent>,
    #[serde(default)]
    pub scratchpad: Option<Scratchpad>,
}

impl CoreAgentSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::System(message.into()));
        self
    }

    pub fn with_user(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::User(message.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_agent_session_round_trips_tool_calls_and_provider_metadata() {
        let session = CoreAgentSession {
            events: vec![
                ConversationEvent::User("question".into()),
                ConversationEvent::ToolCalls(vec![
                    ToolCall::new("inspect", serde_json::json!({"path": "."}), "call-1")
                        .with_thinking_state(Some("gemini-signature".into())),
                ]),
                ConversationEvent::ToolResults(vec![ToolResult::new(
                    "inspect", "result", "call-1",
                )]),
            ],
            scratchpad: None,
        };

        let serialized = serde_json::to_string(&session).unwrap();
        let restored: CoreAgentSession = serde_json::from_str(&serialized).unwrap();

        assert_eq!(restored.events, session.events);
        match &restored.events[1] {
            ConversationEvent::ToolCalls(calls) => {
                assert_eq!(calls[0].thinking_state.as_deref(), Some("gemini-signature"));
            }
            _ => panic!("expected tool calls"),
        }
    }
}
