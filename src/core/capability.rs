use serde_json::{Value};
use schemars::{JsonSchema};
use serde::{Serialize , Deserialize};
use crate::tools::git_status::GitStatus;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::git_log::GitLog;
use crate::tools::ask_user::AskUser;
use crate::tools::json::Json;
use crate::tools::read_dir::ReadDir;
use crate::tools::error::ToolError;


#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema , Debug , Clone)]
pub enum ToolNames{
    GitStatus,
    GitDiffStaged,
    GitLog,
    AskUser,
    ReadDir,
    Json(Value)
}

impl ToolNames {
    pub fn to_capability(&self) -> Box<dyn Capability>{
        match self {
            ToolNames::GitStatus =>   Box::new(GitStatus),
            ToolNames::GitDiffStaged => Box::new(GitDiffStaged),
            ToolNames::Json(value) => Box::new(Json{ properties:value.clone()}),
            ToolNames::GitLog =>  Box::new(GitLog), 
            ToolNames::AskUser => Box::new(AskUser),
            ToolNames::ReadDir => Box::new(ReadDir),
        }
    }
}

#[derive(Serialize , Deserialize , PartialEq , Eq , Debug , Clone )]
pub struct ToolFunction {
    pub name: String,
    pub description:String,
    pub parameters:Value,
}

pub trait Capability:Send + Sync{
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolFunction;
    fn execute(&self, args: Value) -> Result<String, ToolError>;
    fn arg_schema(&self) -> Option<&Value> {
        None
    }
}
