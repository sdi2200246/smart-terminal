use serde_json::{Value};
use serde::{Serialize , Deserialize};
use schemars::JsonSchema;
use crate::tools::git_status::GitStatus;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::list_process::ProcessList;
use crate::tools::error::ToolError;


#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema , Debug , Clone)]
pub enum ToolNames{
    GitStatus,
    ProcessList,
    FinalAnswer,
    GitDiffStaged
}
impl AsRef<str> for ToolNames {
    fn as_ref(&self) -> &str {
        match self {
            ToolNames::GitStatus => "git_status",
            ToolNames::GitDiffStaged =>"git_diff_staged",
            ToolNames::ProcessList => "running_processes",
            ToolNames::FinalAnswer => "final_answer",
        }
    }
}

#[derive(Serialize , Deserialize , PartialEq , Eq , Debug , Clone )]
pub struct ToolFunction {
    pub name: String,
    pub description:String,
    pub parameters: serde_json::Value,
}

pub trait Capability:Send + Sync{
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolFunction;
    fn execute(&self, args: Value) -> Result<String, ToolError>;
    fn arg_schema(&self) -> Option<&Value> {
        None
    }
}

pub fn available_tools()->Vec<Box<dyn Capability>>{
    let static_capabilities: Vec<Box<dyn Capability>> = vec![
        Box::new(GitStatus),
        Box::new(ProcessList),
        Box::new(GitDiffStaged)
    ];
    static_capabilities
}


pub struct FinalAnswer {
    pub properties: Value,
}

impl Capability for FinalAnswer {
    fn name(&self) -> &'static str {
        "final_answer"
    }

    fn metadata(&self) -> ToolFunction{
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
        Err(ToolError::ToolExecution{source: anyhow::anyhow!("FinalAnswer tool should not be called.")})
    }
    fn arg_schema(&self) -> Option<&Value> {
        Some(&self.properties)
    }

}