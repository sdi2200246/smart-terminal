use super::{AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome};
use crate::agent::prompts::SCRATCHPAD_GUIDANCE;
use crate::core::{Scratchpad, ToolMetaData};
use crate::tools::ToolError;
use serde_json::Value;

define_signal! {
    pub struct UpdateScratchpadSignal;
    guidance = SCRATCHPAD_GUIDANCE;
    /// Update the agent's scratchpad with new working memory.
    fn update_scratchpad(args: Scratchpad, ctx: &mut AgentSignalContext<'_>) -> Result<AgentSignalOutcome, ToolError> {
        ctx.agent.session.update_scratchpad(args);
        Ok(AgentSignalOutcome {
            result: "scratchpad updated successfully".into(),
            control: AgentLoopControl::Continue,
        })
    }
}
