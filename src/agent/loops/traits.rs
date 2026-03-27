use std::collections::HashMap;
use crate::interfaces::llm_client::LLMProvider;
use crate::interfaces::capability::{Capability , ToolFunction , FinalAnswer};
use crate::interfaces::session::{AgentSession , ConversationEvent , Model};
use crate::agent::error::AgentError;
use crate::agent::request::AgentRequest;
use serde_json::Value;


const DEFAULT_STEPS: usize = 20;
pub type ToolRegistry = HashMap<&'static str, Box<dyn Capability>>;

#[allow(async_fn_in_trait)]
pub trait AgentLoop:Send{

    fn build_tools_registry(req: &AgentRequest) -> ToolRegistry {
        let mut tools = ToolRegistry::new();
        req.tools.iter().for_each(|t| {
            let capability = t.to_capability();
            tools.insert(capability.name(), capability);
        });
        tools
    }

    fn build_attempt_session(tools: &ToolRegistry, req: &AgentRequest , model:Model) -> AgentSession {
        let mut tool_functions: Vec<ToolFunction> = tools.values()
            .map(|t| t.metadata())
            .collect();

        tool_functions.push(FinalAnswer { properties: req.contract.clone() }.metadata());

        let mut session = AgentSession::new(tool_functions, DEFAULT_STEPS , model);
        session.events = req.messages.iter()
            .map(|m| ConversationEvent::from(m.clone()))
            .collect();

        session
    }

    fn validate_contract(&self, response: &Value, contract: &Value) -> Result<(), AgentError> {
        if contract.is_null() {
            return Ok(());
        }
        let validator = jsonschema::validator_for(contract)
            .map_err(|e| AgentError::InvalidContract(e.to_string()))?;

        if validator.is_valid(response) {
            Ok(())
        } else {
            Err(AgentError::ContractViolation)
        }
    }

    fn agent_loop(&mut self, req: AgentRequest,provider: &mut impl LLMProvider,) -> impl Future<Output = Result<Value, AgentError>> + Send;
}