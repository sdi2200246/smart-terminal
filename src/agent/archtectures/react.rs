use crate::agent::error::AgentError;
use crate::core::capability::{ToolMetaData, ToolRegistry};
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::session::{AgentSession, AgentToolCall, Model};
use crate::utils::FlatSchema;

use super::hook::{HookAction, LoopHook};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub struct ReactLoop<P: LLMProvider> {
    provider: P,
    hook: Option<Box<dyn LoopHook>>,
    events_stream: Option<UnboundedSender<AgentToolCall>>,
}

impl<P: LLMProvider> ReactLoop<P> {
    pub fn new(provider: P) -> Self {
        ReactLoop {
            provider,
            hook: None,
            events_stream: None,
        }
    }

    pub fn with_hook(mut self, hook: Box<dyn LoopHook>) -> Self {
        self.hook = Some(hook);
        self
    }
    pub fn clear_hook_state(&mut self) {
        if let Some(hook) = &mut self.hook {
            hook.clear_state();
        }
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<AgentToolCall>) -> Self {
        self.events_stream = Some(tx);
        self
    }

    #[tracing::instrument(
        skip(self, session, tools, tools_meta, model),
        fields(loop_kind = "React")
    )]
    pub async fn run<T>(
        &mut self,
        session: &mut AgentSession,
        tools: &ToolRegistry,
        tools_meta: &[ToolMetaData],
        model: &Model,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        tracing::info!(status = "started", "ReAct loop execution");
        let mut call: AgentToolCall;
        loop {
            if let Some(value) = session.take_final_answer() {
                tracing::info!(status = "success", "ReAct loop terminated: final answer provided");
                return serde_json::from_value::<T>(value)
                    .map_err(|_| AgentError::ScheemaViolation);
            }

            if session.steps_exhausted() {
                tracing::error!(status = "exhausted", "ReAct loop terminated: step limit reached");
                return Err(AgentError::StepsExhausted);
            }

            call = match self.call_llm(session, tools_meta, model).await? {
                Some(c) => c,
                None => continue,
            };

            tracing::debug!(tool = %call.name(), status = "received", "LLM provided tool call");
            if call.name() == "stop" {
                break;
            }
            session.add_tool_call(call.name(), call.arguments().clone(), call.id());

            if let Some(hook) = &mut self.hook {
                if matches!(hook.pre_call(session, &call)?, HookAction::Skip) {
                    tracing::warn!(tool = %call.name(), status = "skipped", "Hook requested tool skip");
                    continue;
                }
            }
            if self.dispatch_tool_step(session, tools, &call).is_err() {
                continue;
            }
        }
        self.structure_output::<T>(session, call.arguments()).await
    }

    fn dispatch_tool_step(
        &self,
        session: &mut AgentSession,
        tools: &ToolRegistry,
        call: &AgentToolCall,
    ) -> Result<(), ()> {
        let result = match tools[call.name()].execute(call.arguments().clone()) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(tool = %call.name(), status = "failed", error = %e, "Tool execution failed");
                session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                return Err(());
            }
        };

        if call.name() == "final_answer" {
            session.set_final_answer(call.arguments().clone());
        } else {
            session.add_tool_result(call.name(), result, call.id());
            if let Some(stream) = &self.events_stream {
                let _ = stream.send(call.clone());
            }
        }
        Ok(())
    }

    async fn call_llm(
        &mut self,
        session: &mut AgentSession,
        tools_meta: &[ToolMetaData],
        model: &Model,
    ) -> Result<Option<AgentToolCall>, AgentError> {
        let request = AgentRequest {
            model,
            session,
            tools_metadata: tools_meta,
        };

        match self.provider.complete(request).await {
            Ok(call) => Ok(Some(call)),
            Err(ProviderError::InvalidToolCal { source }) => {
                tracing::warn!(status = "invalid_call", error = %source, "LLM returned malformed tool call");
                session.add_error(format!("{}", source));
                Ok(None)
            }
            Err(e) =>{
                tracing::error!(status = "provider_error", error = %e, "LLM provider failure");
                Err(e.into())
            }
        }
    }
    async fn structure_output<T>(
        &mut self,
        session: &mut AgentSession,
        stop_args: &Value,
    ) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        session.clear_events();
        session.add_system("Your one and ONLY job is to return the following text into the scheema provided to you");
        session.add_user(stop_args.to_string());

        tracing::info!(status = "structuring", "Initiating final output structure");
        let raw = self
            .provider
            .complete_structured(session, T::schema())
            .await?;
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        tracing::info!(status = "complete", "Final output structure generated");
        Ok(typed)
    }
}
