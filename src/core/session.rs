use serde::Serialize;
use serde_json::Value;

const DEFAULT_STEPS: usize = 50;

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq)]
pub enum ConversationEvent {
    System(String),
    User(String),
    /// One model turn's calls, in emission order. Length 1 for sequential.
    ToolCalls(Vec<ToolCall>),
    /// Results for the immediately preceding ToolCalls, same order and length.
    ToolResults(Vec<ToolResult>),
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

    pub fn add_tool_calls(&mut self, calls: Vec<ToolCall>) {
        debug_assert!(!calls.is_empty(), "empty tool call batch");
        self.events.push(ConversationEvent::ToolCalls(calls));
    }

    pub fn add_tool_results(&mut self, results: Vec<ToolResult>) {
        debug_assert_eq!(
            self.pending_call_count(),
            Some(results.len()),
            "tool results must pair 1:1 with the preceding call batch"
        );
        self.events.push(ConversationEvent::ToolResults(results));
    }


    pub fn add_error(&mut self, er: String) {
        self.events
            .push(ConversationEvent::System(format!("[ERROR]:{}", er)));
    }

    fn pending_call_count(&self) -> Option<usize> {
        match self.events.last() {
            Some(ConversationEvent::ToolCalls(calls)) => Some(calls.len()),
            _ => None,
        }
    }
    
    pub fn current_steps(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, ConversationEvent::ToolCalls(_)))
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