use super::api::request::GeminiRequest;
use super::api::responce::{GeminiResponse, LlmToolCall};
use super::error::GoogleError;
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::session::{AgentSession, AgentToolCall};
use reqwest::{Client, StatusCode};
use serde_json::Value;

#[derive(Clone)]
pub struct GoogleClient {
    pub client: Client,
    pub api_key: String,
    pub completions_url: String,
}

impl GoogleClient {
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

        GoogleClient {
            client,
            api_key: std::env::var("GEMINI_API_KEY")
                .or_else(|_| std::env::var("GOOGLE_API_KEY"))
                .unwrap(),
            completions_url: "https://generativelanguage.googleapis.com/v1beta/models/".into(),
        }
    }

    pub async fn call_llm(&mut self, req: GeminiRequest) -> Result<GeminiResponse, GoogleError> {
        let url = format!(
            "{}{}:generateContent?key={}",
            self.completions_url, req.model, self.api_key
        );

        let res = self
            .client
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| GoogleError::Http { source: e.into() })?;

        let status = res.status();

        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(Self::map_status(status, body));
        }

        res.json::<GeminiResponse>()
            .await
            .map_err(|e| GoogleError::MalformedResponse { source: e.into() })
    }

    fn map_status(status: StatusCode, body: String) -> GoogleError {
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return GoogleError::TokenLimit {
                source: anyhow::anyhow!("{} {}", status, body),
            };
        }

        if status == StatusCode::BAD_REQUEST && (body.contains("tool") || body.contains("function"))
        {
            return GoogleError::InvalidToolCall {
                source: anyhow::anyhow!("{} {}", status, body),
            };
        }

        GoogleError::Protocol {
            source: anyhow::anyhow!("{} {}", status, body),
        }
    }
}

impl Default for GoogleClient {
    fn default() -> GoogleClient {
        let client = Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(0)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        GoogleClient {
            client,
            api_key: std::env::var("GEMINI_API_KEY")
                .or_else(|_| std::env::var("GOOGLE_API_KEY"))
                .unwrap(),
            completions_url: "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent".into(),
        }
    }
}

impl LLMProvider for GoogleClient {
    async fn complete(
        &mut self,
        request: AgentRequest<'_>,
    ) -> Result<AgentToolCall, ProviderError> {
        let req = GeminiRequest::from(&request);
        let res = self.call_llm(req).await.map_err(ProviderError::from)?;
        let tool_call = LlmToolCall::try_from(res).map_err(ProviderError::from)?;

        Ok(AgentToolCall::new(
            tool_call.name,
            "".into(),
            tool_call.args,
        ))
    }

    async fn complete_structured(
        &mut self,
        session: &AgentSession,
        schema: Value,
    ) -> Result<Value, ProviderError> {
        Ok(Value::Null)
    }
}

#[cfg(test)]
mod unit {
    use crate::core::session::AgentToolCall;
    use crate::google::api::responce::{GeminiResponse, LlmToolCall};
    use serde_json::json;

    fn google_response(
        tool_name: &str,
        tool_id: &str,
        arguments: serde_json::Value,
    ) -> GeminiResponse {
        serde_json::from_value(json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": {
                            "name": tool_name,
                            "id": tool_id,
                            "args": arguments
                        }
                    }]
                }
            }]
        }))
        .unwrap()
    }

    #[test]
    fn final_answer_parses_as_tool_call() {
        let resp = google_response("final_answer", "call_test", json!({"result":"42"}));
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "final_answer");
        assert_eq!(call.args, json!({"result": "42"}));
    }

    #[test]
    fn regular_tool_parses_as_tool_call() {
        let resp = google_response("git_status", "call_test", json!({"path":"."}));
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "git_status");
        assert_eq!(call.args, json!({"path": "."}));
    }

    #[test]
    fn llm_tool_call_converts_to_agent_tool_call() {
        let resp = google_response("final_answer", "call_test", json!({"result":"42"}));
        let llm_call = LlmToolCall::try_from(resp).unwrap();
        let agent_call = AgentToolCall::new(llm_call.name, "".into(), llm_call.args);
        assert_eq!(agent_call.name(), "final_answer");
        assert_eq!(agent_call.arguments().clone(), json!({"result": "42"}));
    }
}
