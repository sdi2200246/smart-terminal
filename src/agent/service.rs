use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};
use serde_json::Value;

use crate::contracts::capability::{Capability, available_tools , FinalAnswer , ToolFunction};
use crate::contracts::llm_client::LLMProvider;
use crate::contracts::session::{AgentOutcome, AgentSession, ConversationEvent};
use super::error::AgentError;
use super::request::AgentRequest;
use super::responce::AgentResponse;

const DEFAULT_STEPS: usize = 10;

pub struct AgentService<P: LLMProvider> {
    rx: Receiver<AgentRequest>,
    tools: HashMap<&'static str, Box<dyn Capability>>,
    provider: P,
}

impl<P: LLMProvider> AgentService<P> {

    pub fn new(rx: Receiver<AgentRequest>, provider: P) -> Self {
        let mut tools: HashMap<&'static str, Box<dyn Capability>> = HashMap::new();
        for tool in available_tools() {
            tools.insert(tool.name(), tool);
        }
        AgentService { rx, tools, provider }
    }

    pub fn spawn(provider: P) -> Sender<AgentRequest>
    where
        P: Send + 'static,
    {
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            AgentService::new(rx, provider).run().await;
        });
        tx
    }

    async fn run(mut self) {
        while let Some(req) = self.rx.recv().await {
            let pipe = req.pipe.clone();
            let response = match self.process(req).await {
                Ok(value) => AgentResponse::Success(value),
                Err(e) => AgentResponse::Error(e),
            };
            let _ = pipe.send(response).await;
        }
    }

    async fn process(&mut self, req: AgentRequest) -> Result<Value, AgentError> {
        let mut session = self.build_session(&req);

        loop {
            if session.steps_exhausted() {
                return Err(AgentError::StepsExhausted);
            }

            match self.provider.complete(&session).await? {
                AgentOutcome::FinalAnswer { arguments } => {
                    self.validate_contract(&arguments, &req.contract)?;
                    return Ok(arguments);
                }
               AgentOutcome::Tool { name, id, arguments } => {
                    let result = self.tools[name.as_str()]
                        .execute(arguments.clone())
                        .map_err(|e|AgentError::Internal(e.into()))?;

                    session.add_tool_call(name.clone(), arguments, id.clone());
                    session.add_tool_result(name, result, id);
                }
            }
        }
    }

    fn build_session(&self, req: &AgentRequest) -> AgentSession {
        let mut tools:Vec<ToolFunction>  = req.tools
            .iter()
            .filter_map(|name| self.tools.get(name.as_ref()))
            .map(|t| t.metadata())
            .collect();

        
        tools.push(FinalAnswer{properties:req.contract.clone()}.metadata());


        let mut session = AgentSession::new(tools, DEFAULT_STEPS);

        for message in &req.messages {
            session.events.push(ConversationEvent::from(message.clone()));
        }

        session
    }

    fn validate_contract(&self, response: &Value, contract: &Value) -> Result<(), AgentError> {
        if contract.is_null() {
            return Ok(());
        }
        let validator = jsonschema::validator_for(contract)
            .map_err(|e| AgentError::InvalidContract(e.to_string()))?;

        if validator.is_valid(response) {
            Ok(())
        } else {
            Err(AgentError::ContractViolation)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::future::Future;
    use tokio::sync::mpsc;
    use crate::agent::request::{AgentRequest, Message};
    use crate::contracts::capability::ToolNames;
    use crate::contracts::error::ProviderError;
    use crate::contracts::session::AgentOutcome;

    struct MockProvider;

    impl LLMProvider for MockProvider {
        fn complete(&mut self, _: &AgentSession) -> impl Future<Output = Result<AgentOutcome, ProviderError>> + Send {
            async { unimplemented!("mock") }
        }
    }

    fn make_service() -> AgentService<MockProvider> {
        let (_, rx) = mpsc::channel(1);
        AgentService::new(rx, MockProvider)
    }

    fn make_request(messages: Vec<Message>, tools: Vec<ToolNames>, contract: Value) -> AgentRequest {
        let (tx, _) = mpsc::channel(1);
        AgentRequest { tools, messages, contract, pipe: tx }
    }

    // --- build_session ---

    #[test]
    fn test_session_system_and_user_messages_mapped() {
        let service = make_service();
        let req = make_request(
            vec![
                Message::system("you are helpful".into()),
                Message::user("hello".into()),
            ],
            vec![],
            Value::Null,
        );

        let session = service.build_session(&req);

        assert_eq!(session.events().len(), 2);
        assert!(matches!(&session.events()[0], ConversationEvent::System(m) if m == "you are helpful"));
        assert!(matches!(&session.events()[1], ConversationEvent::User(m) if m == "hello"));
    }

    #[test]
    fn test_session_always_has_final_answer_tool() {
        let service = make_service();
        let req = make_request(vec![], vec![], Value::Null);

        let session = service.build_session(&req);

        assert!(session.available_tools.iter().any(|t| t.name == "final_answer"));
    }

    #[test]
    fn test_session_final_answer_has_correct_type() {
        let service = make_service();
        let contract = json!({
            "answer": { "type": "string" }
        });
        let req = make_request(vec![], vec![], contract.clone());

        let session = service.build_session(&req);

        let final_answer = session.available_tools.iter().find(|t| t.name == "final_answer").unwrap();
        assert_eq!(final_answer.parameters["type"], "object");
    }

    #[test]
    fn test_session_final_answer_properties_match_contract() {
        let service = make_service();
        let contract = json!({
            "answer": { "type": "string" }
        });
        let req = make_request(vec![], vec![], contract.clone());

        let session = service.build_session(&req);

        let final_answer = session.available_tools.iter().find(|t| t.name == "final_answer").unwrap();
        assert_eq!(final_answer.parameters["properties"], contract);
    }

    #[test]
    fn test_session_final_answer_required_matches_contract_keys() {
        let service = make_service();
        let contract = json!({
            "answer": { "type": "string" },
            "confidence": { "type": "number" }
        });
        let req = make_request(vec![], vec![], contract.clone());

        let session = service.build_session(&req);

        let final_answer = session.available_tools.iter().find(|t| t.name == "final_answer").unwrap();
        let required = final_answer.parameters["required"].as_array().unwrap();
        let mut required_keys: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        required_keys.sort();
        assert_eq!(required_keys, vec!["answer", "confidence"]);
    }

    #[test]
    fn test_session_only_includes_requested_tools_plus_final_answer() {
        let service = make_service();
        let req = make_request(vec![], vec![ToolNames::GitStatus], Value::Null);

        let session = service.build_session(&req);

        assert_eq!(session.available_tools.len(), 2); // GitStatus + FinalAnswer
        assert!(session.available_tools.iter().any(|t| t.name == "git_status"));
        assert!(session.available_tools.iter().any(|t| t.name == "final_answer"));
    }

    #[test]
    fn test_session_no_tools_still_has_final_answer() {
        let service = make_service();
        let req = make_request(vec![], vec![], Value::Null);

        let session = service.build_session(&req);

        assert_eq!(session.available_tools.len(), 1);
        assert_eq!(session.available_tools[0].name, "final_answer");
    }

    #[test]
    fn test_session_has_correct_step_limit() {
        let service = make_service();
        let req = make_request(vec![], vec![], Value::Null);

        let session = service.build_session(&req);

        assert_eq!(session.steps, DEFAULT_STEPS);
    }

    #[test]
    fn test_session_empty_when_no_messages() {
        let service = make_service();
        let req = make_request(vec![], vec![], Value::Null);

        let session = service.build_session(&req);

        assert!(session.is_empty());
    }

   
    // --- validate_contract ---

    #[test]
    fn test_contract_null_always_passes() {
        let service = make_service();
        let result = service.validate_contract(&json!({"any": "value"}), &Value::Null);
        assert!(result.is_ok());
    }

    #[test]
    fn test_contract_valid_response_passes() {
        let service = make_service();
        let contract = json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string" }
            },
            "required": ["answer"]
        });

        assert!(service.validate_contract(&json!({ "answer": "42" }), &contract).is_ok());
    }

    #[test]
    fn test_contract_missing_required_field_fails() {
        let service = make_service();
        let contract = json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string" }
            },
            "required": ["answer"]
        });

        assert!(matches!(
            service.validate_contract(&json!({ "wrong": 123 }), &contract),
            Err(AgentError::ContractViolation)
        ));
    }

    #[test]
    fn test_contract_wrong_type_fails() {
        let service = make_service();
        let contract = json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string" }
            },
            "required": ["answer"]
        });

        assert!(matches!(
            service.validate_contract(&json!({ "answer": 123 }), &contract),
            Err(AgentError::ContractViolation)
        ));
    }

    #[test]
    fn test_contract_invalid_schema_fails() {
        let service = make_service();
        let bad_contract = json!({ "type": "not_a_real_type" });

        assert!(matches!(
            service.validate_contract(&json!({}), &bad_contract),
            Err(AgentError::InvalidContract(_))
        ));
    }
}