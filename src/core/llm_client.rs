use super::capability::ToolMetaData;
use super::error::ProviderError;
use super::model::Model;
use super::session::AgentSession;
use super::responce::AgentResponse;
use serde_json::Value;
use std::future::Future;
pub struct AgentRequest<'a> {
    pub model: &'a Model,
    pub session: &'a AgentSession,
    pub tools_metadata: &'a [ToolMetaData],
}

pub trait LLMProvider: Send {
    fn complete(
        &self,
        request: AgentRequest<'_>,
    ) -> impl Future<Output = Result<AgentResponse, ProviderError>> + Send;

    fn complete_structured(
        &self,
        session: &AgentSession,
        schema: Value,
    ) -> impl Future<Output = Result<Value, ProviderError>> + Send;
}
