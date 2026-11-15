mod agents;
mod error;
mod patterns;
pub(crate) mod prompts;
mod signals;
mod workflows;

pub use agents::{Agent, AgentBuilder, AgentEvent, AgentState, DefaultAgentHook, ToolsRegulator};
pub use error::AgentError;
pub use patterns::{
    AgentLoopHook, DEFAULT_JOB_CAPACITY, HookAction, OneShot, PermissionDecision, ReactLoop,
    SignalDispatcher, TOOL_WAIT_THRESHOLD, ToolDispatcher, ToolExecutionEngine,
    ToolExecutionPolicy, ToolPermissionMode,
};
pub use signals::{
    ActionSpaceError, AgentAction, AgentActionSpace, AgentLoopControl, AgentSignal,
    AgentSignalContext, AgentSignalInvocation, AgentSignalOutcome, FinalAnswerSignal, ModelCall,
    ToolInvocation, UpdateScratchpadSignal, WAIT_JOB_NAME, WaitJobSignal,
};
pub use workflows::{
    Investigator, InvestigatorToolFactory, NextCmd, NextCmdToolFactory, NextCommand, Plan,
    PlanStep, Report, Reversibility,
};

