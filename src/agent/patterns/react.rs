use crate::agent::prompts::STRUCTURING_SYSTEM_PROMPT;
use crate::agent::{Agent, AgentError, AgentState, ToolExecutionEngine, ToolExecutionPolicy};
use crate::core::{AgentRequest, AgentResponse, CoreAgentSession, LLMProvider, ProviderError};
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
        Self::new_with_policy(provider, ToolExecutionPolicy::default())
    }

    pub fn new_with_policy(provider: P, policy: ToolExecutionPolicy) -> Self {
        ReactLoop {
            provider,
            tool_engine: ToolExecutionEngine::new_with_policy(policy),
        }
    }

    #[tracing::instrument(skip(self, agent), fields(loop_kind = "React"))]
    pub async fn run<T>(&mut self, agent: &mut Agent) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        agent.hooks.on_loop_start();
        let result = self.run_loop::<T>(agent).await;
        agent.update_state(if result.is_ok() {
            AgentState::Completed
        } else {
            AgentState::Failed
        });
        result
    }

    async fn run_loop<T>(&mut self, agent: &mut Agent) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        let stop_args: Value;
        loop {
            if let Some(value) = agent.session.take_final_answer() {
                agent.hooks.on_final_answer();
                return serde_json::from_value::<T>(value)
                    .map_err(|_| AgentError::ScheemaViolation);
            }

            if agent.session.steps_exhausted() {
                agent.hooks.on_loop_exhausted();
                return Err(AgentError::StepsExhausted);
            }

            agent.update_state(AgentState::Thinking);
            let response = match self.call_llm(agent).await? {
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
            self.tool_engine.dispatch_tool_batch(agent, response).await;
        }
        self.structure_output::<T>(&stop_args, agent).await
    }

    async fn call_llm(&mut self, agent: &mut Agent) -> Result<Option<AgentResponse>, AgentError> {
        let schemas = agent.action_space.model_schemas();
        let core_session = CoreAgentSession::from(&agent.session);
        let request = AgentRequest {
            model: &agent.model,
            session: &core_session,
            tools_metadata: &schemas,
        };

        match self.provider.complete(request).await {
            Ok(response) => Ok(Some(response)),
            Err(ProviderError::InvalidToolCall { source }) => {
                agent.hooks.on_invalid_tool_call(&source.to_string());
                agent.session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(ProviderError::MalformedResponse { source }) => {
                agent.hooks.on_provider_error(&source.to_string());
                agent.session.add_error(format!("{}", source));
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
        stop_args: &Value,
        agent: &mut Agent,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        agent.update_state(AgentState::StructuringOutput);
        let output_session = CoreAgentSession::new()
            .with_system(STRUCTURING_SYSTEM_PROMPT)
            .with_user(stop_args.to_string());

        agent.hooks.on_structuring_start();
        let raw = self
            .provider
            .complete_structured(&output_session, T::schema())
            .await?;
        let response_text = raw.to_string();
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        agent.session.add_assistant(response_text);
        agent.hooks.on_structuring_complete();
        Ok(typed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentBuilder, FinalAnswerSignal, NextCommand, UpdateScratchpadSignal};
    use crate::core::{
        AgentResponse, AgentToolCall, ConversationEvent, Model, ModelName, ToolCall, ToolResult,
    };
    use serde_json::{Value, json};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
            session: &CoreAgentSession,
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

        let prior = CoreAgentSession {
            events: vec![
                ConversationEvent::System("test".into()),
                ConversationEvent::User("original question".into()),
                ConversationEvent::ToolCalls(vec![
                    ToolCall::new("read_file", json!({"path":"file"}), "call-1")
                        .with_thinking_state(Some("provider-signature".into())),
                ]),
                ConversationEvent::ToolResults(vec![ToolResult::new(
                    "read_file",
                    "contents",
                    "call-1",
                )]),
            ],
            scratchpad: None,
        };

        let mut agent = AgentBuilder::base("test", Model::with_default_temp(ModelName::GptOss120B))
            .with_session(Some(prior))
            .with_user_prompt("resumed question")
            .build();
        let saved_history = agent.session.events.clone();

        runner.run::<NextCommand>(&mut agent).await.unwrap();

        assert_eq!(
            &agent.session.events[..saved_history.len()],
            saved_history.as_slice()
        );
        assert!(matches!(
            agent.session.events.last(),
            Some(ConversationEvent::Assistant(_))
        ));
        assert!(!structured_session_had_tools.load(Ordering::SeqCst));
    }

    #[derive(Clone, Default)]
    struct StopBeforeScratchpad(Arc<AtomicUsize>);

    impl LLMProvider for StopBeforeScratchpad {
        async fn complete(
            &self,
            _request: AgentRequest<'_>,
        ) -> Result<AgentResponse, ProviderError> {
            let call_index = self.0.fetch_add(1, Ordering::SeqCst);
            let call = match call_index {
                0 => AgentToolCall::new(
                    "stop".into(),
                    format!("stop_{call_index}"),
                    json!({"cmd":"echo done","man":"Print done.","scale":"Full"}),
                    None,
                ),
                _ => panic!("unexpected model call"),
            };
            Ok(AgentResponse::single(call))
        }

        async fn complete_structured(
            &self,
            _session: &CoreAgentSession,
            _schema: Value,
        ) -> Result<Value, ProviderError> {
            Ok(json!({"cmd":"echo done","man":"Print done.","scale":"Full"}))
        }
    }

    #[tokio::test]
    async fn structured_stop_is_not_gated_by_scratchpad_updates() {
        let provider = StopBeforeScratchpad::default();
        let calls = provider.0.clone();
        let mut runner = ReactLoop::new(provider);
        let mut agent = AgentBuilder::base("test", Model::with_default_temp(ModelName::GptOss120B))
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .with_user_prompt("request")
            .build();

        runner.run::<NextCommand>(&mut agent).await.unwrap();

        assert!(agent.session.scratchpad.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[derive(Clone)]
    struct FinalAnswerSignalProvider;

    impl LLMProvider for FinalAnswerSignalProvider {
        async fn complete(
            &self,
            _request: AgentRequest<'_>,
        ) -> Result<AgentResponse, ProviderError> {
            Ok(AgentResponse::single(AgentToolCall::new(
                "final_answer".into(),
                "call_fa".into(),
                json!({"cmd":"git status","man":"Check status.","scale":"Full"}),
                None,
            )))
        }

        async fn complete_structured(
            &self,
            _session: &CoreAgentSession,
            _schema: Value,
        ) -> Result<Value, ProviderError> {
            panic!(
                "complete_structured should not be called when final_answer signal finishes loop"
            );
        }
    }

    #[tokio::test]
    async fn final_answer_signal_ends_react_loop_with_value() {
        let mut runner = ReactLoop::new(FinalAnswerSignalProvider);
        let mut agent = AgentBuilder::base("test", Model::with_default_temp(ModelName::GptOss120B))
            .with_signals(vec![Box::new(FinalAnswerSignal {
                properties: NextCommand::schema(),
            })])
            .with_user_prompt("status")
            .build();

        let result: NextCommand = runner.run(&mut agent).await.unwrap();
        assert_eq!(result.cmd, "git status");
        assert_eq!(result.man, "Check status.");
    }
}
