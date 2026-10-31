use super::{AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome};
use crate::core::capability::{ToolMetaData, ToolSafety};
use crate::core::session::Scratchpad;
use crate::tools::error::ToolError;
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
            description: "Overwrite the working scratchpad with its complete updated state.".into(),
            parameters: Scratchpad::schema(),
            safety: ToolSafety::Safe,
        }
    }

    fn prompt_guidance(&self) -> Option<&'static str> {
        Some(
            "Before calling `final_answer`, you MUST call `update_scratchpad` with the complete updated state. Update it after every meaningful investigation milestone. Maintain a concise, durable summary of the main goal, completed milestones, current focus, and vital findings so the investigation can resume from it. Always submit the full updated scratchpad, not a partial patch. The scratchpad is working state, not the final response.",
        )
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

