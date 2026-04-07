use std::future::Future;
use super::session::{AgentSession, AgentToolCall};
use super::error::ProviderError;

pub trait LLMProvider: Send {
    fn complete(&mut self, request: &AgentSession) -> impl Future<Output = Result<AgentToolCall, ProviderError>> + Send;
}