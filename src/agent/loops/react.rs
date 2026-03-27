use super::traits::AgentLoop;

use crate::agent::request::AgentRequest;
use crate::agent::error::AgentError;

use crate::interfaces::error::ProviderError;
use crate::interfaces::session::{AgentOutcome , Model};
use crate::interfaces::llm_client::LLMProvider;

use serde_json::Value;

pub struct ReactLoop{
    model:Model,
}
impl ReactLoop {
    pub fn new(model:Model)->ReactLoop{
        ReactLoop { model }
    }
}
impl AgentLoop for ReactLoop {

    #[tracing::instrument(skip(self , req , provider), fields(loop_kind = "React"))]
    async fn agent_loop(
        &mut self,
        req: AgentRequest,
        provider: &mut impl LLMProvider,
    ) -> Result<Value, AgentError> {
        let tools = Self::build_tools_registry(&req);
        let mut session = Self::build_attempt_session(&tools, &req , self.model.clone());

        loop {
            if session.steps_exhausted() {
                tracing::warn!("agent exhausted all steps");
                return Err(AgentError::StepsExhausted);
            }

            match provider.complete(&session).await {
                Err(ProviderError::InvalidToolCal { source }) => {
                    tracing::warn!(%source, "invalid tool call, recovering and continuing");
                    session.add_error(source.to_string());
                    continue;
                }
                Err(e) => return Err(e.into()),

                Ok(AgentOutcome::FinalAnswer { arguments }) => {
                    self.validate_contract(&arguments, &req.contract)?;
                    return Ok(arguments);
                }

                Ok(AgentOutcome::Tool { name, id, arguments }) => {
                    tracing::info!(tool = %name, args = %arguments, "executing tool");
                    let result = tools[name.as_str()]
                        .execute(arguments.clone())
                        .map_err(|e| AgentError::Internal(e.into()))?;

                    session.add_tool_call(name.clone(), arguments, id.clone());
                    session.add_tool_result(name, result, id);
                }
            }
        }
    }
}