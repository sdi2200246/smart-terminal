use std::collections::HashMap;
use crate::core::llm_client::LLMProvider;
use crate::core::capability::{Capability , ToolFunction};
use crate::core::session::{AgentSession , ConversationEvent , Model};
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
        let tool_functions: Vec<ToolFunction> = tools.values()
            .map(|t| t.metadata())
            .collect();

        let mut session = AgentSession::new(tool_functions, DEFAULT_STEPS , model);
        session.events = req.messages.iter()
            .map(|m| ConversationEvent::from(m.clone()))
            .collect();

        session
    }

    fn agent_loop(&mut self, req: AgentRequest,provider: &mut impl LLMProvider,) -> impl Future<Output = Result<Value, AgentError>> + Send;
}