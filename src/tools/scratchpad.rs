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
            description: "Overwrite your working scratchpad with the FULL updated state \
                (not a diff). Call this after every important milestone.".into(),
            parameters: Scratchpad::schema(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        serde_json::from_value::<Scratchpad>(args)
            .map_err(|e| ToolError::ArgumentsParsing { source: e.into() })?;
        Ok("scratchpad updated successfully".into())
    }
}