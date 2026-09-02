use crate::agent::archtectures::hook::AgentLoopHook;
use crate::agent::error::AgentError;
use crate::core::capability::{ToolMetaData, ToolRegistry};
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::model::Model;
use crate::core::session::{AgentSession, AgentToolCall};
use crate::utils::FlatSchema;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub struct ReactLoop<P: LLMProvider> {
    provider: P,
    events_stream: Option<UnboundedSender<AgentToolCall>>,
}

impl<P: LLMProvider> ReactLoop<P> {
    pub fn new(provider: P) -> Self {
        ReactLoop {
            provider,
            events_stream: None,
        }
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
        let mut call: AgentToolCall;
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

            call = match self
                .call_llm(session, tools.metadata(), model, hooks)
                .await?
            {
                Some(c) => c,
                None => continue,
            };

            hooks.on_tool_received(&call);
            if call.name() == "stop" {
                break;
            }
            session.add_tool_call(call.name(), call.arguments().clone(), call.id() , call.thinking_state() );
            if self
                .dispatch_tool_step(session, tools, &call, hooks)
                .is_err()
            {
                continue;
            }
        }
        self.structure_output::<T>(session, &call.arguments(), hooks)
            .await
    }

    fn dispatch_tool_step(
        &self,
        session: &mut AgentSession,
        tools: &ToolRegistry,
        call: &AgentToolCall,
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<(), ()> {
        let result = match tools
            .get(call.name())
            .expect("correct_tool_name")
            .execute(call.arguments().clone())
        {
            Ok(result) => result,
            Err(e) => {
                hooks.on_tool_failed(call, &e.to_string());
                session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                return Err(());
            }
        };

        if call.name() == "final_answer" {
            session.set_final_answer(call.arguments().clone());
        } else {
            session.add_tool_result(call.name(), result, call.id() , call.thinking_state());
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
        hooks: &mut Box<dyn AgentLoopHook>,
    ) -> Result<Option<AgentToolCall>, AgentError> {
        let request = AgentRequest {
            model,
            session,
            tools_metadata: tools_meta,
        };

        match self.provider.complete(request).await {
            Ok(call) => Ok(Some(call)),
            Err(ProviderError::InvalidToolCall { source }) => {
                hooks.on_invalid_tool_call(&source.to_string());
                session.add_error(format!("{}", source));
                Ok(None)
            },
            Err(ProviderError::MalformedResponse { source })=>{
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
        let raw = self
            .provider
            .complete_structured(session, T::schema())
            .await?;
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        hooks.on_structuring_complete();
        Ok(typed)
    }
}
