use crate::agent::agents::Agent;
use crate::agent::error::AgentError;
use crate::agent::patterns::tool_engine::ToolExecutionEngine;
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::responce::AgentResponse;
use crate::core::session::AgentSession;
use crate::utils::FlatSchema;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct ReactLoop<P: LLMProvider> {
    provider: P,
    tool_engine: ToolExecutionEngine,
}

impl<P: LLMProvider> ReactLoop<P> {
    pub fn new(provider: P) -> Self {
        ReactLoop {
            provider,
            tool_engine: ToolExecutionEngine::new(),
        }
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<crate::core::responce::AgentToolCall>) -> Self {
        self.tool_engine = self.tool_engine.with_events_streaming(tx);
        self
    }

    #[tracing::instrument(skip(self, agent, session), fields(loop_kind = "React"))]
    pub async fn run<T>(
        &mut self,
        agent: &mut Agent,
        session: &mut AgentSession,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        agent.hooks.on_loop_start();
        let stop_args: Value;
        loop {
            if let Some(value) = session.take_final_answer() {
                agent.hooks.on_final_answer();
                return serde_json::from_value::<T>(value)
                    .map_err(|_| AgentError::ScheemaViolation);
            }

            if session.steps_exhausted() {
                agent.hooks.on_loop_exhausted();
                return Err(AgentError::StepsExhausted);
            }

            let response = match self.call_llm(session, agent).await? {
                Some(r) => r,
                None => continue,
            };

            if response.is_stop() {
                let call = response.calls().first().expect("is_stop guarantees one call");
                agent.hooks.on_tool_received(call);
                stop_args = call.arguments();
                break;
            }

            self.tool_engine.dispatch_tool_batch(session, agent, response);
        }
        self.structure_output::<T>(session, &stop_args, agent).await
    }

    async fn call_llm(
        &mut self,
        session: &mut AgentSession,
        agent: &mut Agent,
    ) -> Result<Option<AgentResponse>, AgentError> {
        let request = AgentRequest {
            model: &agent.model,
            session,
            tools_metadata: agent.registry.metadata(),
        };

        match self.provider.complete(request).await {
            Ok(response) => Ok(Some(response)),
            Err(ProviderError::InvalidToolCall { source }) => {
                agent.hooks.on_invalid_tool_call(&source.to_string());
                session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(ProviderError::MalformedResponse { source }) => {
                agent.hooks.on_provider_error(&source.to_string());
                session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(e) => {
                agent.hooks.on_provider_error(&e.to_string());
                Err(e.into())
            }
        }
    }

    async fn structure_output<T>(
        &mut self,
        session: &mut AgentSession,
        stop_args: &Value,
        agent: &mut Agent,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        session.clear_events();
        session.add_system("Your one and ONLY job is to return the following text into the scheema provided to you");
        session.add_user(stop_args.to_string());

        agent.hooks.on_structuring_start();
        let raw = self.provider.complete_structured(session, T::schema()).await?;
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        agent.hooks.on_structuring_complete();
        Ok(typed)
    }
}