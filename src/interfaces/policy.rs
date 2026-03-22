use crate::agent::request::AgentRequest;
pub enum AgentMode {
    Auto,
    Align,
}
pub struct AgentIntent {
    pub prompt: String,
    pub mode: AgentMode,
}
pub trait AgentPolicy {
    fn create_req(&self, args:AgentIntent) -> AgentRequest;
}