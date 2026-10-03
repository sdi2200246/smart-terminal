use crate::agent::agents::{Agent, AgentState};
use crate::agent::error::AgentError;
use crate::agent::patterns::tool_engine::ToolExecutionEngine;
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::responce::AgentResponse;
use crate::core::session::AgentSession;
use crate::utils::FlatSchema;

use serde::de::DeserializeOwned;
use serde_json::Value;

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
        let result = self.run_loop::<T>(agent, session).await;
        agent.update_state(if result.is_ok() {
            AgentState::Completed
        } else {
            AgentState::Failed
        });
        result
    }

    async fn run_loop<T>(
        &mut self,
        agent: &mut Agent,
        session: &mut AgentSession,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
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

            agent.update_state(AgentState::Thinking);
            let response = match self.call_llm(session, agent).await? {
                Some(r) => r,
                None => continue,
            };

            if response.is_stop() {
                let call = response
                    .calls()
                    .first()
                    .expect("is_stop guarantees one call");
                agent.hooks.on_tool_received(call);
                stop_args = call.arguments();
                break;
            }

            agent.update_state(AgentState::ExecutingTool);
            self.tool_engine
                .dispatch_tool_batch(session, agent, response);
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
        agent.update_state(AgentState::StructuringOutput);
        let output_session = AgentSession::builder()
            .system(
                "Your one and ONLY job is to return the following text into the scheema provided to you",
            )
            .user(stop_args.to_string())
            .build();

        agent.hooks.on_structuring_start();
        let raw = self
            .provider
            .complete_structured(&output_session, T::schema())
            .await?;
        let response_text = raw.to_string();
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        session.add_assistant(response_text);
        agent.hooks.on_structuring_complete();
        Ok(typed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::workflows::next_cmd::NextCommand;
    use crate::core::model::{Model, ModelName};
    use crate::core::responce::{AgentResponse, AgentToolCall};
    use crate::core::session::{ConversationEvent, ToolCall, ToolResult};
    use serde_json::{Value, json};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[derive(Clone)]
    struct StructuredCallTracker(Arc<AtomicBool>);

    impl LLMProvider for StructuredCallTracker {
        async fn complete(
            &self,
            _request: AgentRequest<'_>,
        ) -> Result<AgentResponse, ProviderError> {
            Ok(AgentResponse::single(AgentToolCall::new(
                "stop".into(),
                String::new(),
                json!({"cmd":"echo done","man":"Print done.","scale":"Full"}),
                None,
            )))
        }

        async fn complete_structured(
            &self,
            session: &AgentSession,
            _schema: Value,
        ) -> Result<Value, ProviderError> {
            self.0.store(
                session
                    .events
                    .iter()
                    .any(|event| matches!(event, ConversationEvent::ToolCalls(_))),
                Ordering::SeqCst,
            );
            Ok(json!({"cmd":"echo done","man":"Print done.","scale":"Full"}))
        }
    }

    #[tokio::test]
    async fn structured_output_does_not_clear_live_session_history() {
        let structured_session_had_tools = Arc::new(AtomicBool::new(false));
        let provider = StructuredCallTracker(structured_session_had_tools.clone());
        let mut runner = ReactLoop::new(provider);
        let mut agent = Agent::base("test", Model::with_default_temp(ModelName::GptOss120B));
        let mut session = AgentSession::builder()
            .system("test")
            .user("original question")
            .build();
        session.add_tool_calls(vec![
            ToolCall::new("read_file", json!({"path":"file"}), "call-1")
                .with_thinking_state(Some("provider-signature".into())),
        ]);
        session.add_tool_results(vec![ToolResult::new("read_file", "contents", "call-1")]);
        session.add_user("resumed question");
        let saved_history = session.events.clone();

        runner
            .run::<NextCommand>(&mut agent, &mut session)
            .await
            .unwrap();

        assert_eq!(
            &session.events[..saved_history.len()],
            saved_history.as_slice()
        );
        assert!(matches!(
            session.events.last(),
            Some(ConversationEvent::Assistant(_))
        ));
        assert!(!structured_session_had_tools.load(Ordering::SeqCst));
    }
}
