use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;
use tokio::{sync::mpsc::Sender};

use crate::agent::responce::AgentResponse;

use super::protocol::message::Message;

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
#[derive(Clone)]
pub struct AgentRequest {
    pub tools:Vec<ToolNames>,
    pub messages:Vec<Message>,
    pub contract:Value,
    pub pipe:Sender<AgentResponse>
}

impl AgentRequest {
    pub fn new(tools: Vec<ToolNames>,messages: Vec<Message>,contract: Value,pipe:Sender<AgentResponse>) ->AgentRequest{
        AgentRequest {tools, messages, contract,pipe}
    }
    pub fn builder(pipe:Sender<AgentResponse>)-> AgentRequest{
        AgentRequest {tools:vec![], messages:vec![], contract:Value::Null , pipe}
    }
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
    pub fn messages(mut self , messages:Vec<Message>) -> Self{
        self.messages = messages;
        self

    }
    pub fn tool(mut self, tool: ToolNames) -> Self {
        self.tools.push(tool);
        self
    }
    pub fn tools(mut self, tools: Vec<ToolNames>) -> Self {
        self.tools = tools;
        self
    }

    pub fn contract(mut self, contract: Value) ->  Self {
        self.contract = contract;
        self
    }

    pub fn with_context<T:Serialize>(mut self , ctx:&T)->Self{
        let ctx_message = Message::context(ctx);
        self.messages.push(ctx_message);
        self
    }

    pub fn with_system_promt(mut self , promt:String)->  Self{
        let sys_message = Message::system(Some(promt));
        self.messages.push(sys_message);
        self
    }

}
