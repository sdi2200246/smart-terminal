use crate::interfaces::llm_client::LLMProvider;
use super::loops::traits::AgentLoop;
use super::request::AgentRequest;
use super::responce::AgentResponse;

pub struct AgentClient<P: LLMProvider, L: AgentLoop> {
    id: String,
    provider: P,
    agent_loop: L,
}

impl<P: LLMProvider, L: AgentLoop> AgentClient<P, L> {
    pub fn new(id: impl Into<String>, provider: P, agent_loop: L) -> Self {
        Self { id: id.into(), provider, agent_loop }
    }

    pub async fn execute_request(&mut self, req: AgentRequest) -> AgentResponse {
        tracing::info!(agent_id = %self.id, "executing request");
        match self.agent_loop.agent_loop(req, &mut self.provider).await {
            Ok(value) => {
                tracing::info!(agent_id = %self.id, "agent request completed successfully");
                AgentResponse::Success(value)
            }
            Err(e) => {
                tracing::error!(agent_id = %self.id, error = ?e, "agent request failed");
                AgentResponse::Error(e)
            }
        }
    }
}