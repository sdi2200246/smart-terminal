use serde_json::Value;
use crate::protocol::tool::Tool;
use super::git_status::GitStatus;
use super::git_diff::GitDiffStaged;
use super::list_process::ProcessList;
use super::error::ToolError;

pub trait Capability:Send + Sync{
    fn name(&self) -> &'static str;
    fn to_protocol(&self) -> Tool;
    fn execute(&self, args: Value) -> Result<Value, ToolError>;
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
