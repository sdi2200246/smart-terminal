use serde_json::{Value};
use serde::{Serialize , Deserialize};
use super::git_status::GitStatus;
use super::git_diff::GitDiffStaged;
use super::list_process::ProcessList;
use super::error::ToolError;


#[derive(Serialize , Deserialize , PartialEq , Eq , Debug , Clone )]
pub struct ToolFunction {
    pub name: String,
    pub description:String,
    pub parameters: serde_json::Value,
}

pub trait Capability:Send + Sync{
    fn name(&self) -> &'static str;
    fn to_protocol(&self) -> ToolFunction;
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
