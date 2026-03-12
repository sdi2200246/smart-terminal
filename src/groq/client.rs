use reqwest::{Client, StatusCode };
use crate::interfaces::llm_client::LLMProvider;
use crate::interfaces::session::{AgentSession , AgentOutcome};
use crate::interfaces::error::ProviderError;
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

        if status == StatusCode::BAD_REQUEST && body.contains("tool_use_failed") {
            return GroqError::InvalidToolCall { source: anyhow::anyhow!("{} {}", status, body) };
        }

        GroqError::Protocol {
            source: anyhow::anyhow!("{} {}", status, body),
        }
    }  
}
impl Default for  GroqClient{
    
    fn default()->GroqClient{

        let client = Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(0) // avoid stale reused connections
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        GroqClient{
            client,
            api_key:std::env::var("GROQ_API_KEY").unwrap(),
            completions_url:"https://api.groq.com/openai/v1/chat/completions".into(),
        }
    }
}

impl LLMProvider for GroqClient {

    async fn complete(&mut self , session:&AgentSession,)->Result<AgentOutcome,ProviderError>{
        let req = GroqRequest::from(session);
        let res = self.call_llm(req).await.map_err(ProviderError::from)?;
        let outcome = LlmOutcome::try_from(res).map_err(ProviderError::from)?;

        match outcome {
            LlmOutcome::FinalAnswer { answer } => Ok(AgentOutcome::FinalAnswer { arguments: answer }),
            LlmOutcome::ToolCall { name, id, args } => Ok(AgentOutcome::Tool { name, id, arguments: args }),
        }
    }   
}


#[cfg(test)]
mod unit {
    use crate::groq::protocol::responce::{GroqResponse, LlmOutcome};
    use crate::interfaces::session::AgentOutcome;
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