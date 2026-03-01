use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{sync::mpsc::Sender};
use crate::contracts::capability::ToolNames;
use crate::agent::responce::AgentResponse;


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
        let sys_message = Message::system(promt);
        self.messages.push(sys_message);
        self
    }

}
#[derive(Serialize, Deserialize, Debug, Clone , PartialEq)]
pub struct Message {
    pub role: String,
    pub content:String,
}   

impl  Message {
    pub fn is_user(&self)->bool{
        self.role == "user"
    }
    pub fn user(content:String)->Message{
        Message {
            role:"user".into(),
            content,
        }
    }

    pub fn is_system(&self)->bool{
        self.role == "system"
    }

    pub fn system(content:String)->Message{
        Message {
            role:"system".into(),
            content,
        }
    }
    pub fn context<T:Serialize>(ctx:&T)->Message{
        let json = serde_json::to_string_pretty(ctx).unwrap();
        let content = format!("Context:\n{}", json);

        Message {
            role:"system".into(),
            content:content,
        }
    }
}