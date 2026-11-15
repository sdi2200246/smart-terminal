mod action_space;
mod final_answer;
mod scratchpad;
mod wait_job;

pub use action_space::{
    ActionSpaceError, AgentAction, AgentActionSpace, AgentSignalInvocation, ModelCall,
    ToolInvocation,
};
pub use final_answer::FinalAnswerSignal;
pub use scratchpad::UpdateScratchpadSignal;
pub use wait_job::{WAIT_JOB_NAME, WaitJobSignal};

use crate::agent::Agent;
use crate::core::{AgentSession, ToolMetaData};
use crate::multithreading::JobStore;
use crate::tools::ToolError;
use serde_json::Value;

pub struct AgentSignalContext<'a> {
    pub session: &'a mut AgentSession,
    pub agent: &'a mut Agent,
    pub job_store: &'a JobStore,
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
    fn must_run_alone(&self) -> bool {
        false
    }
    fn handle(
        &self,
        args: Value,
        ctx: &mut AgentSignalContext<'_>,
    ) -> Result<AgentSignalOutcome, ToolError>;
}
