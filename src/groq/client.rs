use reqwest::{Client , StatusCode};
use crate::contracts::llm_client::LLMProvider;
use crate::contracts::session::{AgentSession , AgentOutcome};
use crate::contracts::error::DomainError;
use super::error::GroqError;
use super::protocol::responce::{GroqResponse , LlmOutcome};
use super::protocol::request::GroqRequest;

pub struct GroqClient{
    pub client:Client,
    pub api_key:String,
    pub completions_url:String,
}
impl GroqClient{
    pub async fn call_llm(&mut self,req: GroqRequest) -> Result<GroqResponse, GroqError> {

        let res = self.client
            .post(&self.completions_url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .json(&req)
            .send()
            .await
            .map_err(| e| GroqError::Http{source:e.into()})?;

        let status = res.status();

        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(Self::map_status(status, body));
        }

        res.json::<GroqResponse>()
            .await
            .map_err(|e| GroqError::MalformedResponse { source: e.into() })
    }

    fn map_status(status: StatusCode, body: String) -> GroqError {

        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return GroqError::TokenLimit{ source: anyhow::anyhow!("{} {}", status, body),};
        }

        GroqError::Protocol {
            source: anyhow::anyhow!("{} {}", status, body),
        }
    }  
}
impl Default for  GroqClient{
    fn default()->GroqClient{
        GroqClient{
            client:Client::new(),
            api_key:std::env::var("GROQ_API_KEY").unwrap(),
            completions_url:"https://api.groq.com/openai/v1/chat/completions".into(),
        }
    }
}

impl LLMProvider for GroqClient {

    async fn complete(&mut self , session:&AgentSession,)->Result<AgentOutcome,DomainError>{
        let req = GroqRequest::from(session);
        let res = self.call_llm(req).await.map_err(DomainError::from)?;
        let outcome = LlmOutcome::try_from(res).map_err(DomainError::from)?;

        match outcome {
            LlmOutcome::FinalAnswer { answer } => Ok(AgentOutcome::FinalAnswer { arguments: answer }),
            LlmOutcome::ToolCall { name, id, args } => Ok(AgentOutcome::Tool { name, id, arguments: args }),
        }
    }   
}


#[cfg(test)]
mod unit {
    use crate::groq::protocol::responce::{GroqResponse, LlmOutcome};
    use crate::contracts::session::AgentOutcome;
    use serde_json::json;

    
    fn groq_response(tool_name: &str, arguments: &str) -> GroqResponse {
        serde_json::from_value(json!({
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test",
                        "type": "function",
                        "function": {
                            "name": tool_name,
                            "arguments": arguments
                        }
                    }]
                }
            }]
        })).unwrap()
    }

    // ── LlmOutcome::try_from ───────────────────────────────────────────────

    #[test]
    fn final_answer_maps_to_outcome() {
        let resp = groq_response("final_answer", r#"{"result":"42"}"#);
        let outcome = LlmOutcome::try_from(resp).unwrap();
        assert!(matches!(outcome, LlmOutcome::FinalAnswer { .. }));
    }

    #[test]
    fn tool_call_maps_to_outcome() {
        let resp = groq_response("git_status", r#"{"path":"."}"#);
        let outcome = LlmOutcome::try_from(resp).unwrap();
        match outcome {
            LlmOutcome::ToolCall { name, id, args } => {
                assert_eq!(name, "git_status");
                assert_eq!(id, "call_test");
                assert_eq!(args, json!({"path": "."}));
            }
            _ => panic!("Expected ToolCall"),
        }
    }

    #[test]
    fn final_answer_converts_to_agent_outcome() {
        let resp = groq_response("final_answer", r#"{"result":"42"}"#);
        let outcome = LlmOutcome::try_from(resp).unwrap();
        let agent_outcome = match outcome {
            LlmOutcome::FinalAnswer { answer } => AgentOutcome::FinalAnswer { arguments: answer },
            LlmOutcome::ToolCall { name, id, args } => AgentOutcome::Tool { name, id, arguments: args },
        };
        assert!(matches!(agent_outcome, AgentOutcome::FinalAnswer { .. }));
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::contracts::llm_client::LLMProvider;
    use crate::contracts::session::{AgentSession, AgentOutcome, ConversationEvent};
    use crate::contracts::capability::ToolFunction;
    use serde_json::json;

    fn final_answer_tool() -> ToolFunction {
        ToolFunction {
            name: "final_answer".into(),
            description: "Return the final answer to the user".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "result": {
                        "type": "string",
                        "description": "The final answer"
                    }
                },
                "required": ["result"]
            }),
        }
    }

    fn simple_session() -> AgentSession {
        AgentSession {
            events: vec![
                ConversationEvent::System(
                    "You are a helpful assistant. You MUST always respond by calling the final_answer tool.".into()
                ),
                ConversationEvent::User("What is 2 + 2?".into()),
            ],
            available_tools: vec![final_answer_tool()],
            steps: 0,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore]
    async fn complete_returns_final_answer() {

        dotenv::dotenv().ok();

        let key = std::env::var("GROQ_API_KEY")
            .expect("GROQ_API_KEY must be set to run integration tests");

        let mut client = GroqClient {
            client: reqwest::Client::new(),
            api_key: key,
            completions_url: "https://api.groq.com/openai/v1/chat/completions".into(),
        };

        let session = simple_session();
        let result = client.complete(&session).await;

        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        match result.unwrap() {
            AgentOutcome::FinalAnswer { arguments } => {
                println!("Got final answer: {}", arguments);
                assert!(arguments.get("result").is_some(), "missing 'result' key");
            }
            AgentOutcome::Tool { name, .. } => {
                panic!("Expected FinalAnswer but got tool call: {}", name);
            }
        }
    }
}