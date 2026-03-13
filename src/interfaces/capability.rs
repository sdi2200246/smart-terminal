use serde_json::{Value};
use schemars::{JsonSchema,schema_for};
use serde::{Serialize , Deserialize};
use crate::tools::git_status::GitStatus;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::git_log::GitLog;
use crate::tools::list_process::ProcessList;
use crate::tools::ask_user::AskUser;
use crate::tools::error::ToolError;


#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema , Debug , Clone)]
pub enum ToolNames{
    GitStatus,
    ProcessList,
    FinalAnswer,
    GitDiffStaged,
    GitLog,
    AskUser,
}


impl ToolNames {
    pub fn to_capability(&self) -> Box<dyn Capability>{
        match self {
            ToolNames::GitStatus =>   Box::new(GitStatus),
            ToolNames::GitDiffStaged => Box::new(GitDiffStaged),
            ToolNames::ProcessList =>  Box::new(ProcessList),
            ToolNames::FinalAnswer => Box::new(FinalAnswer{properties:Value::Null}),
            ToolNames::GitLog =>  Box::new(GitLog), 
            ToolNames::AskUser => Box::new(AskUser)
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

pub trait ToolArgs : JsonSchema{
    fn schema() -> Value {
        serde_json::to_value(schema_for!(Self)).unwrap()
    }
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
            parameters:self.properties.clone(),
        }
    }

    fn execute(&self, _args: Value) -> Result<String,  ToolError> {
        Err(ToolError::ToolExecution{source: anyhow::anyhow!("FinalAnswer tool should not be called.")})
    }
    fn arg_schema(&self) -> Option<&Value> {
        Some(&self.properties)
    }

}