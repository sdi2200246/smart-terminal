use crate::core::capability::ToolFunction;
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

impl AgentSession {
    pub fn new(available_tools: Vec<ToolFunction>, steps: usize , model:Model) -> Self {
        Self {
            events: Vec::new(),
            available_tools,
            steps,
            model
        }
    }
    pub fn add_system(&mut self, message: impl Into<String>) {
        self.events.push(ConversationEvent::System(message.into()));
    }

    pub fn add_user(&mut self, message: impl Into<String>) {
        self.events.push(ConversationEvent::User(message.into()));
    }

    pub fn add_outcome(&mut self, outcome: AgentOutcome) {
        match outcome {
            AgentOutcome::Tool { name, id, arguments } => {
                self.events.push(ConversationEvent::ToolCall { name, arguments, id });
            }
            AgentOutcome::FinalAnswer { .. } => {}
        }
    }
    pub fn add_reflection(&mut self, reflection: impl Into<String>) {
        self.events.push(ConversationEvent::System(
            format!("[REFLECTION] {}", reflection.into())
        ));
}

    pub fn add_tool_call(&mut self, name: impl Into<String>, arguments:Value, id: impl Into<String>) {
         self.events.push(ConversationEvent::ToolCall { name:name.into(), arguments,id:id.into() });
    }

    pub fn add_tool_result(&mut self, name: impl Into<String>, result: impl Into<String>, id: impl Into<String>) {
        self.tool_result_event(name.into(), result.into(), id.into());
    }

    pub fn is_final(outcome: &AgentOutcome) -> bool {
        matches!(outcome, AgentOutcome::FinalAnswer { .. })
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

    pub fn add_error(&mut self, er: String) {
        self.events.push(ConversationEvent::System(er));
    }

    fn _tool_call_event(&mut self, tool_call: AgentOutcome) {
        match tool_call {
            AgentOutcome::Tool { name, id, arguments } => {
                self.events.push(ConversationEvent::ToolCall { name, arguments, id });
            }
            _ => {}
        }
    }
    fn tool_result_event(&mut self, name: String, result: String, id: String) {
        self.events.push(ConversationEvent::ToolResult { name, result, id });
    }

    /// Remove all tools except final_answer
    pub fn lock_to_final_answer(&mut self) {
        self.available_tools.retain(|t| t.name == "final_answer");
    }
    pub fn get_model(&self)->&Model{
        &self.model
    }

}




#[cfg(test)]
mod tests {
    use crate::core::session::ModelName;

    use super::*;
    use serde_json::json;

    fn make_session(steps: usize) -> AgentSession {
        AgentSession::new(vec![], steps , Model::new(ModelName::GptOss120B , 0.5))
    }

    fn make_tool_outcome(name: &str, id: &str) -> AgentOutcome {
        AgentOutcome::Tool {
            name: name.to_string(),
            id: id.to_string(),
            arguments: json!({}),
        }
    }

    #[test]
    fn test_new_session_is_empty() {
        let session = make_session(5);
        assert!(session.is_empty());
        assert_eq!(session.current_steps(), 0);
        assert_eq!(session.steps, 5);
    }

    #[test]
    fn test_add_system_event() {
        let mut session = make_session(5);
        session.add_system("you are a helpful assistant");
        assert_eq!(session.events().len(), 1);
        assert!(matches!(&session.events()[0], ConversationEvent::System(msg) if msg == "you are a helpful assistant"));
    }

    #[test]
    fn test_add_user_event() {
        let mut session = make_session(5);
        session.add_user("hello");
        assert!(matches!(&session.events()[0], ConversationEvent::User(msg) if msg == "hello"));
    }

    #[test]
    fn test_add_tool_outcome_pushes_tool_call() {
        let mut session = make_session(5);
        session.add_outcome(make_tool_outcome("search", "id-1"));
        assert_eq!(session.current_steps(), 1);
        assert!(matches!(&session.events()[0], ConversationEvent::ToolCall { name, id, .. } if name == "search" && id == "id-1"));
    }

    #[test]
    fn test_add_final_answer_pushes_no_event() {
        let mut session = make_session(5);
        session.add_outcome(AgentOutcome::FinalAnswer { arguments: json!({"answer": "42"}) });
        assert!(session.is_empty());
        assert_eq!(session.current_steps(), 0);
    }

    #[test]
    fn test_is_final() {
        let tool = make_tool_outcome("search", "id-1");
        let final_answer = AgentOutcome::FinalAnswer { arguments: json!({}) };
        assert!(!AgentSession::is_final(&tool));
        assert!(AgentSession::is_final(&final_answer));
    }

    #[test]
    fn test_steps_exhausted() {
        let mut session = make_session(2);
        assert!(!session.steps_exhausted());

        session.add_outcome(make_tool_outcome("search", "id-1"));
        assert!(!session.steps_exhausted());

        session.add_outcome(make_tool_outcome("search", "id-2"));
        assert!(session.steps_exhausted());
    }

    #[test]
    fn test_steps_not_exhausted_by_non_tool_events() {
        let mut session = make_session(2);
        session.add_system("system prompt");
        session.add_user("user message");
        assert!(!session.steps_exhausted());
        assert_eq!(session.current_steps(), 0);
    }

    #[test]
    fn test_add_tool_result() {
        let mut session = make_session(5);
        session.add_tool_result("search", "some results", "id-1");
        assert!(matches!(
            &session.events()[0],
            ConversationEvent::ToolResult { name, result, id }
            if name == "search" && result == "some results" && id == "id-1"
        ));
    }

    #[test]
    fn test_current_steps_only_counts_tool_calls() {
        let mut session = make_session(10);
        session.add_system("sys");
        session.add_user("user");
        session.add_outcome(make_tool_outcome("tool_a", "id-1"));
        session.add_tool_result("tool_a", "result", "id-1");
        session.add_outcome(make_tool_outcome("tool_b", "id-2"));

        assert_eq!(session.current_steps(), 2);
        assert_eq!(session.events().len(), 5);
    }

    #[test]
    fn test_error_event_pushes_system_message() {
        let mut session = make_session(5);
        session.add_error("something went wrong".to_string());
        assert!(matches!(&session.events()[0], ConversationEvent::System(msg) if msg == "something went wrong"));
    }
}