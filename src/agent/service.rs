use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver , Sender};

use crate::protocol::message::{Message};
use crate::protocol::tool::Tool;
use crate::protocol::model_result::ModelOutcome;
use crate::protocol::request::ChatRequest;

use crate::groq::client::{GroqClient, LlmError};

use super::tools::capability::{Capability ,available_tools};
use super::tools::final_answer::FinalAnswer;
use super::request::AgentRequest;
use super::responce::AgentResponse;
use super::session::AgentSession;

#[derive(Serialize , Deserialize , Debug , PartialEq)]
pub enum AgentError{
    LoopError,
    FinalAnswerError,
    NetworkError,
    ToolError,
    IncorrectArgs,
    SessionCreation,
    SessionCompletion,
    ClientError,
    Protocol
}

pub struct AgentService{
    rx: Receiver<AgentRequest>,
    tools_registry: HashMap<&'static str, Box<dyn Capability>>,
}

impl AgentService{
    
      pub fn spawn() -> Sender<AgentRequest> {
        let (tx, rx) = mpsc::channel(8);

        let service = AgentService::new(rx);

        tokio::spawn(async move {
            service.run().await;
        });

        tx
    }

    async fn run(mut self) {
        let client:GroqClient = GroqClient::default();

        while let Some(req) = self.rx.recv().await {
            let pipe = req.pipe.clone();

            match self.worker(req , &client).await{
                Ok(v) =>{
                    let _ = pipe.send(AgentResponse::Success(v)).await;
                }
                Err(e) =>{
                    let _ = pipe.send(AgentResponse::Error(e)).await;
                }
            }
        }
    }

    pub fn new(rx:Receiver<AgentRequest>)->AgentService{
        let tools = available_tools();
        let mut tool_map:HashMap< &'static str, Box<dyn Capability>> = HashMap::new();
        for t in tools{
            tool_map.insert(t.name().into(), t);
        }
        AgentService {rx,tools_registry:tool_map}
    }
    
    pub fn session(&self, req: AgentRequest) -> Result<AgentSession, AgentError> {
        let mut tool_protocol: Vec<Tool> = req
            .tools
            .iter()
            .map(|t| {
                self.tools_registry
                    .get(t.as_ref())
                    .ok_or(AgentError::SessionCreation)
                    .map(|cap| cap.to_protocol())
            })
            .collect::<Result<_, _>>()?;

        tool_protocol.push(FinalAnswer{properties: req.contract.clone()}.to_protocol());

        Ok(AgentSession::new("openai/gpt-oss-120b".into(),req.messages,tool_protocol, 10,req.contract,))
    }

    pub fn validate_response(&self , response: &Value, contract: &Value) -> Result<() , AgentError> {
        let validator = match jsonschema::validator_for(contract) {
            Ok(v) => v,
            Err(_) => return Err(AgentError::FinalAnswerError),
        };
        if validator.is_valid(response){
            Ok(())
        }
        else {
            Err(AgentError::FinalAnswerError)
        }
    }

    fn execute_tool(&self,name: &str,arguments: Value,) -> Result<String, AgentError> {

        let capability = self
            .tools_registry
            .get(name)
            .ok_or(AgentError::ToolError)?;

        capability.execute(arguments).map_err(|_| AgentError::ToolError)
    }

    pub async fn worker(&self , req:AgentRequest ,  client:&GroqClient)->Result<Value , AgentError>{
        let mut session = self.session(req)?;

        while session.steps()!= 0{
            let res = match client.llm_request(ChatRequest::from(&session)).await {
                    Ok(r) => r,
                    Err(e) => {
                        match e {
                            LlmError::BadStatus(_, message) => {
                                session.error(message);
                                session.decrease_steps();
                                continue;
                            }
                            _ => return Err(AgentError::ClientError),
                        }
                    }
                };


            let outcome = ModelOutcome::try_from(&res).map_err(|_|{AgentError::Protocol})?;

            match outcome {
                ModelOutcome::Tool { name, arguments, id } => {
                    if name == "final_answer".to_string(){
                        self.validate_response(&arguments, &session.contract)?;
                        return Ok(arguments);

                    }
                    let tool_result = self.execute_tool(&name, arguments)?;
                    session.model_res(res.message());
                    session.tool_result(Message::tool_responce(Some(tool_result), id, name));
                }
            }
            session.decrease_steps();

        }
        return Err(AgentError::SessionCompletion);

    }
}
