use super::{AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome};
use crate::agent::prompts::{SCRATCHPAD_DESCRIPTION, SCRATCHPAD_GUIDANCE};
use crate::core::{Scratchpad, ToolMetaData, ToolSafety};
use crate::tools::ToolError;
use crate::utils::FlatSchema;
use serde_json::Value;

impl FlatSchema for Scratchpad {}

pub struct UpdateScratchpadSignal;

impl AgentSignal for UpdateScratchpadSignal {
    fn name(&self) -> &'static str {
        "update_scratchpad"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: SCRATCHPAD_DESCRIPTION.into(),
            parameters: Scratchpad::schema(),
            safety: ToolSafety::Safe,
        }
    }

    fn prompt_guidance(&self) -> Option<&'static str> {
        Some(SCRATCHPAD_GUIDANCE)
    }

    fn handle(
        &self,
        args: Value,
        ctx: &mut AgentSignalContext<'_>,
    ) -> Result<AgentSignalOutcome, ToolError> {
        let sp = serde_json::from_value::<Scratchpad>(args).map_err(|e| {
            ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("invalid scratchpad — {e}"),
            }
        })?;
        ctx.session.update_scratchpad(sp);
        Ok(AgentSignalOutcome {
            result: "scratchpad updated successfully".into(),
            control: AgentLoopControl::Continue,
        })
    }
}

