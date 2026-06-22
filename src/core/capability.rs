use crate::tools::error::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub type ToolRegistry = HashMap<&'static str, Box<dyn Capability>>;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ToolMetaData {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub trait Capability: Send + Sync {
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolMetaData;
    fn execute(&self, args: Value) -> Result<String, ToolError>;
}
