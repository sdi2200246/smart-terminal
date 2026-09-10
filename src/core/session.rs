use serde::Serialize;
use serde_json::Value;

const DEFAULT_STEPS: usize = 50;

#[derive(Debug, PartialEq)]
pub enum ConversationEvent {
    System(String),
    User(String),
    ToolCall {
        name: String,
        arguments: Value,
        id: String,
        thinking_state: Option<String>,
    },
    ToolResult {
        name: String,
        result: String,
        id: String,
        thinking_state: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct AgentToolCall {
    id: String,
    arguments: Value,
    name: String,
    thinking_state: Option<String>,
}
impl AgentToolCall {
    pub fn new(name: String, id: String, arguments: Value, thinking_state: Option<String>) -> Self {
        Self {
            name,
            id,
            arguments,
            thinking_state,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn arguments(&self) -> Value {
        self.arguments.clone()
    }
    pub fn into_arguments(self) -> Value {
        self.arguments
    }
    pub fn thinking_state(&self) -> Option<String> {
        self.thinking_state.clone()
    }
}

#[derive(Debug)]
pub struct AgentSession {
    pub events: Vec<ConversationEvent>,
    pub steps: usize,
    pub final_answer: Option<Value>,
}

impl AgentSession {
    pub fn new(steps: usize) -> Self {
        Self {
            events: Vec::new(),
            steps,
            final_answer: None,
        }
    }

    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }

    pub fn add_system(&mut self, message: impl Into<String>) {
        self.events.push(ConversationEvent::System(message.into()));
    }

    pub fn add_user(&mut self, message: impl Into<String>) {
        self.events.push(ConversationEvent::User(message.into()));
    }

    pub fn add_reflection(&mut self, reflection: impl Into<String>) {
        self.events.push(ConversationEvent::System(format!(
            "[REFLECTION] {}",
            reflection.into()
        )));
    }

    pub fn add_tool_call(
        &mut self,
        name: impl Into<String>,
        arguments: Value,
        id: impl Into<String>,
        thinking_state: Option<String>,
    ) {
        self.events.push(ConversationEvent::ToolCall {
            name: name.into(),
            arguments,
            id: id.into(),
            thinking_state: thinking_state,
        });
    }

    pub fn add_tool_result(
        &mut self,
        name: impl Into<String>,
        result: impl Into<String>,
        id: impl Into<String>,
        thinking_state: Option<String>,
    ) {
        self.events.push(ConversationEvent::ToolResult {
            name: name.into(),
            result: result.into(),
            id: id.into(),
            thinking_state,
        });
    }

    pub fn add_error(&mut self, er: String) {
        let error = format!("[ERROR]:{}", er);
        self.events.push(ConversationEvent::System(error));
    }

    pub fn current_steps(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, ConversationEvent::ToolCall { .. }))
            .count()
    }

    pub fn steps_exhausted(&self) -> bool {
        self.current_steps() >= self.steps
    }

    pub fn events(&self) -> &[ConversationEvent] {
        &self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn set_final_answer(&mut self, value: Value) {
        self.final_answer = Some(value);
    }

    pub fn take_final_answer(&mut self) -> Option<Value> {
        self.final_answer.take()
    }
}

pub struct SessionBuilder {
    events: Vec<ConversationEvent>,
    steps: usize,
}

impl SessionBuilder {
    fn new() -> Self {
        Self {
            events: vec![],
            steps: DEFAULT_STEPS,
        }
    }
    pub fn system(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::System(message.into()));
        self
    }

    pub fn user(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::User(message.into()));
        self
    }

    pub fn context<T: Serialize>(mut self, ctx: &T) -> Self {
        let json = serde_json::to_string_pretty(ctx).unwrap();
        self.events
            .push(ConversationEvent::System(format!("Context:\n{}", json)));
        self
    }

    pub fn steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    pub fn build(self) -> AgentSession {
        AgentSession {
            events: self.events,
            steps: self.steps,
            final_answer: None,
        }
    }
}
