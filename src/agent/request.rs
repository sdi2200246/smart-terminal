use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;
use tokio::{sync::mpsc};

use crate::agent::responce::AgentResponse;

use super::protocol::message::Message;

#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema , Debug)]
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
pub struct AgentRequest {
    pub tools:Vec<ToolNames>,
    pub messages:Vec<Message>,
    pub contract:Value,
    pub pipe:mpsc::Sender<AgentResponse>

}

impl AgentRequest {
    pub fn new(tools: Vec<ToolNames>,messages: Vec<Message>,contract: Value,pipe:mpsc::Sender<AgentResponse>) -> AgentRequest{
        AgentRequest {tools, messages, contract,pipe}
    }
    pub fn builder(pipe:mpsc::Sender<AgentResponse>)-> AgentRequest{
        AgentRequest {tools:vec![], messages:vec![], contract:Value::Null , pipe}
    }
    pub fn message(&mut self, message: Message) -> &mut Self {
        self.messages.push(message);
        self
    }
    pub fn messages(&mut self , messages:Vec<Message>) -> &mut Self{
        self.messages = messages;
        self

    }
    pub fn tool(&mut self, tool: ToolNames) -> &mut Self {
        self.tools.push(tool);
        self
    }
    pub fn tools(&mut self, tools: Vec<ToolNames>) -> &mut Self {
        self.tools = tools;
        self
    }

    pub fn contract(&mut self, contract: Value) -> &mut Self {
        self.contract = contract;
        self
    }
}
