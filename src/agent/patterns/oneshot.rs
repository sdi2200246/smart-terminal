use crate::agent::{Agent, AgentError};
use crate::core::{CoreAgentSession, LLMProvider};
use crate::utils::FlatSchema;
use serde::de::DeserializeOwned;

pub struct OneShot<P: LLMProvider> {
    provider: P,
}

impl<P: LLMProvider> OneShot<P> {
    pub fn new(provider: P) -> Self {
        OneShot { provider }
    }

    #[tracing::instrument(skip(self, agent), fields(loop_kind = "OneShot"))]
    pub async fn run<T>(&mut self, agent: &Agent) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        let core_session = CoreAgentSession::from(&agent.session);
        let raw = self
            .provider
            .complete_structured(&core_session, T::schema())
            .await?;
        serde_json::from_value::<T>(raw).map_err(|_| AgentError::ScheemaViolation)
    }
}
