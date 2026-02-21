use serde_json::Value;

use super::protocol::message::Message;
use super::protocol::tool::{Tool};

#[derive(Debug)]
pub struct AgentSession{
    pub model:String,
    pub messages:Vec<Message>,
    pub tools:Vec<Tool>,
    pub steps:usize,
    pub contract:Value,
}

impl AgentSession {
    pub fn new(model:&str , messages:Vec<Message> , tools:Vec<Tool> , steps:usize , contract:Value)->AgentSession{
        AgentSession {model:model.into(), messages, tools, steps, contract}
    }
    pub fn tool_result(&mut self, message: Message) -> &mut Self {
        self.messages.push(message);
        self
    }
    pub fn model_res(&mut self , message:Message) ->&mut Self{
        self.messages.push(message);
        self
    }
    pub fn error(&mut self, er:String) -> &mut Self{
        let message = Message::system(Some(er));
        self.messages.push(message);
        self
    } 

    pub fn decrease_steps(&mut self){
            self.steps -= 1;
    }
    pub fn steps(&self)->usize{
        self.steps
    }

}
