use super::traits::AgentLoop;

use crate::agent::request::AgentRequest;
use crate::agent::error::AgentError;
use crate::core::error::ProviderError;
use crate::core::session::Model;
use crate::core::llm_client::LLMProvider;

use serde_json::Value;

pub struct ReactLoop {
    model: Model,
}

impl ReactLoop {
    pub fn new(model: Model) -> ReactLoop {
        ReactLoop { model }
    }
}

impl AgentLoop for ReactLoop {
    #[tracing::instrument(skip(self, req, provider), fields(loop_kind = "React"))]
    async fn agent_loop(&mut self, req: AgentRequest, provider: &mut impl LLMProvider) -> Result<Value, AgentError> {
        let tools = Self::build_tools_registry(&req);
        let mut session = Self::build_attempt_session(&tools, &req, self.model.clone());
        let mut last_args: Option<Value> = None;

        loop {
            if session.steps_exhausted() {
                tracing::warn!("agent exhausted all steps");
                return Err(AgentError::StepsExhausted);
            }

            if session.is_resolved() {
                return Ok(last_args.unwrap());
            }

            let call = match provider.complete(&session).await {
                Ok(call) => call,
                Err(ProviderError::InvalidToolCal { source }) => {
                    tracing::warn!(%source, "invalid tool call, recovering and continuing");
                    let available: Vec<_> = tools.keys().copied().collect();
                    session.add_error(format!(
                        "Invalid tool call!:\nOnly Available tools:{}",
                        available.join(", ")
                    ));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            tracing::info!(tool = %call.name(), args = %call.arguments(), "executing tool");

            let result = match tools[call.name()].execute(call.arguments().clone()) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(tool = %call.name(), error = %e, "tool execution failed");
                    session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                    continue;
                }
            };
            last_args = Some(call.arguments().clone());
            session.add_tool_call(call.name(), call.arguments().clone(), call.id());
            session.add_tool_result(call.name(), result, call.id());
        }
    }
}