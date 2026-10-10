use super::error::ToolError;
use crate::core::capability::{Capability, ToolMetaData};
use crate::core::session::Scratchpad;
use crate::utils::FlatSchema;
use serde_json::Value;

impl FlatSchema for Scratchpad {}

pub struct UpdateScratchpad;

impl Capability for UpdateScratchpad {
    fn name(&self) -> &'static str {
        "update_scratchpad"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Overwrite the working scratchpad with its complete updated state.".into(),
            parameters: Scratchpad::schema(),
        }
    }

    fn prompt_guidance(&self) -> Option<&'static str> {
        Some(
            "Before calling `final_answer`, you MUST call `update_scratchpad` with the complete updated state. Update it after every meaningful investigation milestone. Maintain a concise, durable summary of the main goal, completed milestones, current focus, and vital findings so the investigation can resume from it. Always submit the full updated scratchpad, not a partial patch. The scratchpad is working state, not the final response.",
        )
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        serde_json::from_value::<Scratchpad>(args)
            .map_err(|e| ToolError::ArgumentsParsing { source: e.into() })?;
        Ok("scratchpad updated successfully".into())
    }
}
