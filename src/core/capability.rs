use crate::tools::error::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolSafety {
    #[default]
    Safe,
    Mutating,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ToolMetaData {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    #[serde(default)]
    pub safety: ToolSafety,
}

pub trait Capability: Send + Sync {
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolMetaData;
    fn prompt_guidance(&self) -> Option<&'static str> {
        None
    }
    fn execute(&self, args: Value) -> Result<String, ToolError>;
}
