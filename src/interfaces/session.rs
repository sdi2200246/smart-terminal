use crate::interfaces::capability::ToolFunction;
use serde_json::Value;

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
pub enum AgentOutcome{

    FinalAnswer{
        arguments:Value
    },
    Tool{
        name: String,
        id: String,
        arguments: Value,
    },
}

#[derive(Debug)]
pub struct AgentSession{
    pub events:Vec<ConversationEvent>,
    pub available_tools:Vec<ToolFunction>,
    pub steps:usize,
    pub model:Model,
}
