use crate::agent::error::AgentError;
use crate::agent::patterns::hook::AgentLoopHook;
use crate::core::capability::{ToolMetaData, ToolRegistry};
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::model::Model;
use crate::core::session::{AgentSession, ToolCall, ToolResult};
use crate::core::responce::{AgentResponse, AgentToolCall};
use crate::utils::FlatSchema;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct ReactLoop<P: LLMProvider> {
    provider: P,
    events_stream: Option<UnboundedSender<AgentToolCall>>,
}

impl<P: LLMProvider> ReactLoop<P> {
    pub fn new(provider: P) -> Self {
        ReactLoop { provider, events_stream: None }
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<AgentToolCall>) -> Self {
        self.events_stream = Some(tx);
        self
    }

    #[tracing::instrument(skip(self, session, tools, model, hooks), fields(loop_kind = "React"))]
    pub async fn run<T>(
        &mut self,
        session: &mut AgentSession,
        tools: &ToolRegistry,
        model: &Model,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        hooks.on_loop_start();
        let stop_args: Value;
        loop {
            if let Some(value) = session.take_final_answer() {
                hooks.on_final_answer();
                return serde_json::from_value::<T>(value)
                    .map_err(|_| AgentError::ScheemaViolation);
            }

            if session.steps_exhausted() {
                hooks.on_loop_exhausted();
                return Err(AgentError::StepsExhausted);
            }

            let response = match self
                .call_llm(session, tools.metadata(), model, hooks)
                .await?
            {
                Some(r) => r,
                None => continue,
            };

            tracing::info!(
                tool_call_count = response.len(),
                "Received response from LLM"
            );

            if response.is_stop() {
                let call = response.calls().first().expect("is_stop guarantees one call");
                hooks.on_tool_received(call);
                stop_args = call.arguments();
                break;
            }

            self.dispatch_tool_batch(session, tools, response, hooks);
        }
        self.structure_output::<T>(session, &stop_args, hooks).await
    }


    fn dispatch_tool_batch(
        &self,
        session: &mut AgentSession,
        tools: &ToolRegistry,
        response: AgentResponse,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) {
        let calls = response.into_calls();

        for call in &calls {
            hooks.on_tool_received(call);
        }

        session.add_tool_calls(
            calls
                .iter()
                .map(|c| {
                    ToolCall::new(c.name(), c.arguments().clone(), c.id())
                        .with_thinking_state(c.thinking_state())
                })
                .collect(),
        );

        let mut results = Vec::with_capacity(calls.len());
        let mut final_answer: Option<Value> = None;

        for call in &calls {
            let payload = match self.execute_tool(tools, call, hooks) {
                Ok(result) => {
                    if call.name() == "final_answer" {
                        final_answer = Some(call.arguments().clone());
                    } else if let Some(stream) = &self.events_stream {
                        let _ = stream.send(call.clone());
                    }
                    result
                }
                Err(e) => format!("Tool '{}' failed: {}", call.name(), e),
            };
            results.push(ToolResult::new(call.name(), payload, call.id()));
        }

        session.add_tool_results(results);

        if let Some(value) = final_answer {
            session.set_final_answer(value);
        }
    }

    fn execute_tool(
        &self,
        tools: &ToolRegistry,
        call: &AgentToolCall,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<String, String> {
        match tools
            .get(call.name())
            .expect("correct_tool_name")
            .execute(call.arguments().clone())
        {
            Ok(result) => Ok(result),
            Err(e) => {
                let msg = e.to_string();
                hooks.on_tool_failed(call, &msg);
                Err(msg)
            }
        }
    }

    async fn call_llm(
        &mut self,
        session: &mut AgentSession,
        tools_meta: &[ToolMetaData],
        model: &Model,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<Option<AgentResponse>, AgentError> {
        let request = AgentRequest { model, session, tools_metadata: tools_meta };

        match self.provider.complete(request).await {
            Ok(response) => Ok(Some(response)),
            Err(ProviderError::InvalidToolCall { source }) => {
                hooks.on_invalid_tool_call(&source.to_string());
                session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(ProviderError::MalformedResponse { source }) => {
                hooks.on_provider_error(&source.to_string());
                session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(e) => {
                hooks.on_provider_error(&e.to_string());
                Err(e.into())
            }
        }
    }

    async fn structure_output<T>(
        &mut self,
        session: &mut AgentSession,
        stop_args: &Value,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        session.clear_events();
        session.add_system("Your one and ONLY job is to return the following text into the scheema provided to you");
        session.add_user(stop_args.to_string());

        hooks.on_structuring_start();
        let raw = self.provider.complete_structured(session, T::schema()).await?;
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        hooks.on_structuring_complete();
        Ok(typed)
    }
}