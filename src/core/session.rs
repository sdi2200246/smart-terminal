use serde::Serialize;
use serde_json::Value;

const DEFAULT_STEPS: usize = 50;

#[derive(Debug , PartialEq , Clone)]
pub enum ModelName{
    GptOss120B,
    GptOss20B,
    Llma3p18B,
    Llma3p370B,
}

#[derive(Debug , PartialEq , Clone)]
pub struct Model{
    name: ModelName,
    temperature:f32,
}

impl Model {
    pub fn new(name: ModelName, temperature: f32) -> Self {
        Self { name, temperature: temperature.clamp(0.0, 2.0) }
    }
    pub fn with_default_temp(name: ModelName) -> Self {
        Self::new(name, 0.7)
    }

    pub fn deterministic(name: ModelName) -> Self {
        Self::new(name, 0.1)
    }

    pub fn creative(name: ModelName) -> Self {
        Self::new(name, 1.2)
    }

    pub fn cooler(&self) -> Self {
        Self {
            name: self.name.clone(),
            temperature: (self.temperature / 2.0).max(0.05),
        }
    }
    pub fn warmer(&self) -> Self {
        Self {
            name: self.name.clone(),
            temperature: (self.temperature * 2.0).min(2.0),
        }
    }
    pub fn with_temperature(&self, temperature: f32) -> Self {
        Self {
            name: self.name.clone(),
            temperature: temperature.clamp(0.0, 2.0),
        }
    }
    pub fn get_name(&self)->ModelName{
        self.name.clone()
    }
    pub fn get_temp(&self)->f32{
        self.temperature
    }
}

#[derive(Debug , PartialEq)]
pub enum ConversationEvent {
    System(String),
    User(String),
    ToolCall {
        name: String,
        arguments: Value,
        id: String,
    },
    ToolResult {
        name: String,
        result: String,
        id: String,
    },
}

#[derive(Debug)]
 pub struct AgentToolCall{
    id: String,
    arguments: Value,
    name: String,
}
impl AgentToolCall {
    pub fn new(name: String, id: String, arguments: Value) -> Self {
        Self { name, id, arguments }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn arguments(&self) -> &Value {
        &self.arguments
    }
    pub fn into_arguments(self) -> Value {
        self.arguments
    }
}

#[derive(Debug)]
pub struct AgentSession {
    pub events: Vec<ConversationEvent>,
    pub steps: usize,
}

impl AgentSession {
    pub fn new(steps: usize) -> Self {
        Self { events: Vec::new(), steps }
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
        self.events.push(ConversationEvent::System(
            format!("[REFLECTION] {}", reflection.into()),
        ));
    }

    pub fn add_tool_call(&mut self, name: impl Into<String>, arguments: Value, id: impl Into<String>) {
        self.events.push(ConversationEvent::ToolCall {
            name: name.into(),
            arguments,
            id: id.into(),
        });
    }

    pub fn add_tool_result(&mut self, name: impl Into<String>, result: impl Into<String>, id: impl Into<String>) {
        self.events.push(ConversationEvent::ToolResult {
            name: name.into(),
            result: result.into(),
            id: id.into(),
        });
    }

    pub fn add_error(&mut self, er: String) {
        self.events.push(ConversationEvent::System(er));
    }

    pub fn current_steps(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, ConversationEvent::ToolCall { .. })).count()
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
        self.events.push(ConversationEvent::System(format!("Context:\n{}", json)));
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
        }
    }
}