use crate::agent::error::AgentError;
use crate::core::capability::{ToolRegistry , ToolMetaData};
use crate::core::error::ProviderError;
use crate::core::session::{AgentSession, AgentToolCall , Model};
use crate::core::llm_client::{LLMProvider , AgentRequest};
use serde::de::DeserializeOwned;
use crate::utils::FlatSchema;
pub struct ReactLoop<P: LLMProvider> {
    provider: P,
}

impl<P: LLMProvider> ReactLoop<P> {
    pub fn new(provider: P) -> Self {
        ReactLoop { provider }
    }

    #[tracing::instrument(skip(self, session, tools, tools_meta, model), fields(loop_kind = "React"))]
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
        let mut call: AgentToolCall;
        loop {
            if session.steps_exhausted() {
                tracing::warn!("agent exhausted all steps");
                return Err(AgentError::StepsExhausted);
            }

            let request = AgentRequest {
                model,
                session,
                tools_metadata: tools_meta,
            };

            call = match self.provider.complete(request).await {
                Ok(call) => call,
                Err(ProviderError::InvalidToolCal { source }) => {
                    let compressed = source.to_string().lines().take(3).collect::<Vec<_>>().join("\n");
                    session.add_error(format!("Invalid tool call:\n{}\n", compressed));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            tracing::info!(tool = %call.name(), args = %call.arguments(), "executing tool");
            if call.name() == "stop" {
                break;
            }

            let result = match tools[call.name()].execute(call.arguments().clone()) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(tool = %call.name(), error = %e, "tool execution failed");
                    session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                    continue;
                }
            };
            session.add_tool_call(call.name(), call.arguments().clone(), call.id());
            session.add_tool_result(call.name(), result, call.id());
        }

        session.clear_events();
        session.add_user(call.arguments().to_string());
        let raw = self.provider.complete_structured(&session, T::schema()).await?;
        let typed = serde_json::from_value::<T>(raw).expect("Type must always be right");
        Ok(typed)
    }
}