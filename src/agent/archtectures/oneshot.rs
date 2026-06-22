use crate::agent::error::AgentError;
use crate::core::llm_client::LLMProvider;
use crate::core::session::AgentSession;
use crate::utils::FlatSchema;
use serde::de::DeserializeOwned;

pub struct OneShot<P: LLMProvider> {
    provider: P,
}

impl<P: LLMProvider> OneShot<P> {
    pub fn new(provider: P) -> Self {
        OneShot { provider }
    }

    #[tracing::instrument(skip(self, session,), fields(loop_kind = "OneShot"))]
    pub async fn run<T>(&mut self, session: &AgentSession) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        let raw = self
            .provider
            .complete_structured(session, T::schema())
            .await?;
        serde_json::from_value::<T>(raw).map_err(|_| AgentError::ScheemaViolation)
    }
}
