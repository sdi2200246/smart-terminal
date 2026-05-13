use std::future::Future;
use serde_json::Value;
use super::session::{AgentSession, AgentToolCall , Model};
use super::error::ProviderError;
use super::capability::{ToolMetaData};
pub struct AgentRequest<'a> {
    pub model: &'a Model,
    pub session: &'a AgentSession,
    pub tools_metadata: &'a [ToolMetaData],
}

pub trait LLMProvider: Send {
    fn complete(
        &mut self,
        request: AgentRequest<'_>,
    ) -> impl Future<Output = Result<AgentToolCall, ProviderError>> + Send;

    fn complete_structured(
        &mut self,
        session: &AgentSession,
        schema: Value,
    ) -> impl Future<Output = Result<Value, ProviderError>> + Send;
}