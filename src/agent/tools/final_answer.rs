use serde_json::Value;
use super::capability::{Capability , ToolFunction};
use super::error::ToolError;
pub struct FinalAnswer {
    pub properties: Value,
}

impl Capability for FinalAnswer {
    fn name(&self) -> &'static str {
        "final_answer"
    }

    fn to_protocol(&self) -> ToolFunction{
        ToolFunction {
                    name: self.name().into(),
                    description:"You MUST use this tool for your final answer.".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": self.properties,
                        "required": self.properties
                            .as_object()
                            .map(|o| o.keys().cloned().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                }
    }

    fn execute(&self, _args: Value) -> Result<String,  ToolError> {
        Err(ToolError::Execution)
    }
    fn arg_schema(&self) -> Option<&Value> {
        Some(&self.properties)
    }

}