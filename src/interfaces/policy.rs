use crate::agent::request::AgentRequest;
use crate::agent::responce::AgentResponse;
use tokio::sync::mpsc::Sender;

pub enum AgentMode {
    Auto,
    Align,
}

pub struct AgentIntent {
    pub prompt: String,
    pub mode: AgentMode,
}

pub trait AgentPolicy {
    fn create_req(&self, args:AgentIntent,response_tx: Sender<AgentResponse>) -> AgentRequest;
}