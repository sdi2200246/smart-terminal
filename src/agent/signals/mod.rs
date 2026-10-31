pub mod action_space;
pub mod final_answer;
pub mod scratchpad;

pub use action_space::{
    ActionSpaceError, AgentAction, AgentActionSpace, AgentSignalInvocation, ModelCall,
    ToolInvocation,
};
pub use final_answer::FinalAnswerSignal;
pub use scratchpad::UpdateScratchpadSignal;

use crate::agent::agents::Agent;
use crate::core::capability::ToolMetaData;
use crate::core::session::AgentSession;
use crate::tools::error::ToolError;
use serde_json::Value;

pub struct AgentSignalContext<'a> {
    pub session: &'a mut AgentSession,
    pub agent: &'a mut Agent,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AgentLoopControl {
    Continue,
    Finish(Value),
}

#[derive(Debug, PartialEq, Clone)]
pub struct AgentSignalOutcome {
    /// Text returned to the model as this call's tool result.
    pub result: String,
    pub control: AgentLoopControl,
}

pub trait AgentSignal: Send + Sync {
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolMetaData;
    fn prompt_guidance(&self) -> Option<&'static str> {
        None
    }
    fn handle(
        &self,
        args: Value,
        ctx: &mut AgentSignalContext<'_>,
    ) -> Result<AgentSignalOutcome, ToolError>;
}
