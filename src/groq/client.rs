use reqwest::{Client, StatusCode };
use serde_json::Value;
use crate::core::llm_client::{LLMProvider , AgentRequest};
use crate::core::session::{AgentSession , AgentToolCall};
use crate::core::error::ProviderError;
use super::error::GroqError;
use super::protocol::responce::{GroqResponse , LlmToolCall , LlmStructuredOutput};
use super::protocol::request::GroqRequest;

#[derive(Clone)]
pub struct GroqClient{
    pub client:Client,
    pub api_key:String,
    pub completions_url:String,
}
impl GroqClient{

    pub fn pooled() -> Self {
        Self::build(2)
    }

    pub fn no_pool() -> Self {
        Self::build(0)
    }

    fn build(max_idle: usize) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(max_idle)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        GroqClient {
            client,
            api_key: std::env::var("GROQ_API_KEY").unwrap(),
            completions_url: "https://api.groq.com/openai/v1/chat/completions".into(),
        }
    }

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

        if status == StatusCode::BAD_REQUEST && (body.contains("tool_use_failed")|| body.contains("output_parse_failed")){
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
            .pool_max_idle_per_host(0) 
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

    async fn complete(&mut self , request:AgentRequest<'_>)->Result<AgentToolCall,ProviderError>{
        let req = GroqRequest::from(&request);
        let res = self.call_llm(req).await.map_err(ProviderError::from)?;
        let tool_call = LlmToolCall::try_from(res).map_err(ProviderError::from)?;

        Ok(AgentToolCall::new(tool_call.name, tool_call.id, tool_call.args))
    }

     async fn complete_structured(&mut self, session: &AgentSession, schema: Value) -> Result<Value, ProviderError> {
        let req = GroqRequest::structured(&session, schema);
        let res = self.call_llm(req).await.map_err(ProviderError::from)?;
        let out = LlmStructuredOutput::try_from(res).map_err(ProviderError::from)?;
        Ok(out.value)
    }
}

#[cfg(test)]
mod unit {
    use crate::groq::protocol::responce::{GroqResponse, LlmToolCall};
    use crate::core::session::AgentToolCall;
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

    #[test]
    fn final_answer_parses_as_tool_call() {
        let resp = groq_response("final_answer", r#"{"result":"42"}"#);
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "final_answer");
        assert_eq!(call.args, json!({"result": "42"}));
    }

    #[test]
    fn regular_tool_parses_as_tool_call() {
        let resp = groq_response("git_status", r#"{"path":"."}"#);
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "git_status");
        assert_eq!(call.id, "call_test");
        assert_eq!(call.args, json!({"path": "."}));
    }

    #[test]
    fn llm_tool_call_converts_to_agent_tool_call() {
        let resp = groq_response("final_answer", r#"{"result":"42"}"#);
        let llm_call = LlmToolCall::try_from(resp).unwrap();
        let agent_call = AgentToolCall::new(llm_call.name, llm_call.id, llm_call.args);
        assert_eq!(agent_call.name(), "final_answer");
        assert_eq!(agent_call.id(), "call_test");
        assert_eq!(agent_call.arguments().clone(), json!({"result": "42"}));
    }
}