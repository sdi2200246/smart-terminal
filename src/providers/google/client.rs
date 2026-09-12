use super::api::request::GeminiRequest;
use super::api::responce::{GeminiResponse, LlmStructuredOutput, LlmResponse , LlmToolCall};
use super::error::GoogleError;
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::session::AgentSession;
use crate::core::responce::{AgentResponse , AgentToolCall};
use crate::providers::client::{ClientConfig, GenericLlmClient, ProviderCodec};
use reqwest::StatusCode;
use serde_json::Value;

#[derive(Clone)]
pub struct GoogleProtocol;

impl ProviderCodec for GoogleProtocol {
    type Request = GeminiRequest;
    type Response = GeminiResponse;
    type Error = GoogleError;

    fn endpoint_path(&self, req: &Self::Request) -> String {
        return format!("/models/{}:generateContent", req.model);
    }

    fn build_complete_request(&self, req: &AgentRequest<'_>) -> Self::Request {
        GeminiRequest::from(req)
    }

    fn build_structured_request(&self, session: &AgentSession, schema: Value) -> Self::Request {
        GeminiRequest::structured(session, schema, "gemini-3.6-flash".into())
    }
    
    fn parse_agent_responce(&self, res: Self::Response) -> Result<AgentResponse, Self::Error> {
        let response = LlmResponse::try_from(res)?;
        let calls :Vec<AgentToolCall> = response
            .calls
            .into_iter()
            .map(|call| AgentToolCall::new(call.name, "".into(), call.args, Some(call.thinking_state)))
            .collect();
        Ok(AgentResponse::from(calls))
    }

    fn parse_structured(&self, res: Self::Response) -> Result<Value, Self::Error> {
        let out = LlmStructuredOutput::try_from(res)?;
        Ok(out.value)
    }

    fn map_status_error(&self, status: StatusCode, body: String) -> Self::Error {
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return GoogleError::TokenLimit { body };
        }
        if status == StatusCode::BAD_REQUEST && (body.contains("tool") || body.contains("function"))
        {
            return GoogleError::InvalidToolCall { body };
        }
        GoogleError::Protocol { status, body }
    }

    fn map_network_error(&self, err: reqwest::Error) -> Self::Error {
        if err.is_timeout() {
            GoogleError::Timeout { source: err }
        } else {
            GoogleError::Network { source: err }
        }
    }
}

#[derive(Clone)]
pub struct GoogleClient {
    inner: GenericLlmClient<GoogleProtocol>,
}

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

impl GoogleClient {
    pub fn pooled() -> Self {
        Self::build(2)
    }
    pub fn no_pool() -> Self {
        Self::build(0)
    }

    fn build(_max_idle: usize) -> Self {
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .expect("GEMINI_API_KEY or GOOGLE_API_KEY must be set");

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-goog-api-key"),
            HeaderValue::from_str(&api_key).expect("Invalid header value"),
        );

        let config = ClientConfig {
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            headers,
            timeout: Duration::from_secs(30),
        };

        Self {
            inner: GenericLlmClient::new(config, GoogleProtocol {}),
        }
    }
}

impl Default for GoogleClient {
    fn default() -> Self {
        Self::no_pool()
    }
}
impl LLMProvider for GoogleClient {
    async fn complete(&self, request: AgentRequest<'_>) -> Result<AgentResponse, ProviderError> {
        self.inner.run_complete(&request).await
    }

    async fn complete_structured(
        &self,
        session: &AgentSession,
        schema: Value,
    ) -> Result<Value, ProviderError> {
        self.inner.run_complete_structured(session, schema).await
    }
}
#[cfg(test)]
mod unit {
    use super::*;
    use crate::core::responce::{AgentResponse, AgentToolCall};
    use crate::providers::google::api::request::GeminiRequest;
    use crate::providers::google::api::responce::{GeminiResponse, LlmResponse};
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

    fn google_response_multi(calls: &[(&str, serde_json::Value)]) -> GeminiResponse {
        let parts: Vec<_> = calls
            .iter()
            .map(|(name, args)| {
                json!({
                    "functionCall": { "name": name, "args": args }
                })
            })
            .collect();

        serde_json::from_value(json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": parts
                }
            }]
        }))
        .unwrap()
    }

    #[test]
    fn final_answer_parses_as_tool_call() {
        let resp = google_response("final_answer", "call_test", json!({"result":"42"}));
        let response = LlmResponse::try_from(resp).unwrap();
        assert_eq!(response.calls.len(), 1);
        assert_eq!(response.calls[0].name, "final_answer");
        assert_eq!(response.calls[0].args, json!({"result": "42"}));
    }

    #[test]
    fn regular_tool_parses_as_tool_call() {
        let resp = google_response("git_status", "call_test", json!({"path":"."}));
        let response = LlmResponse::try_from(resp).unwrap();
        assert_eq!(response.calls.len(), 1);
        assert_eq!(response.calls[0].name, "git_status");
        assert_eq!(response.calls[0].args, json!({"path": "."}));
    }

    #[test]
    fn llm_response_converts_to_agent_tool_call() {
        let resp = google_response("final_answer", "call_test", json!({"result":"42"}));
        let llm_response = LlmResponse::try_from(resp).unwrap();
        let call = &llm_response.calls[0];
        let agent_call = AgentToolCall::new(
            call.name.clone(),
            "".into(),
            call.args.clone(),
            None,
        );
        assert_eq!(agent_call.name(), "final_answer");
        assert_eq!(agent_call.arguments().clone(), json!({"result": "42"}));
    }

    #[test]
    fn parse_agent_responce_wraps_single_call() {
        let protocol = GoogleProtocol;
        let resp = google_response("git_status", "call_test", json!({"path":"."}));

        let response: AgentResponse = protocol.parse_agent_responce(resp).unwrap();

        assert_eq!(response.len(), 1);
        assert_eq!(response.calls()[0].name(), "git_status");
        assert_eq!(response.calls()[0].arguments(), json!({"path": "."}));
    }

    #[test]
    fn parse_agent_responce_wraps_multiple_parallel_calls() {
        let protocol = GoogleProtocol;
        let resp = google_response_multi(&[
            ("read_file", json!({"path": "a.rs"})),
            ("read_file", json!({"path": "b.rs"})),
        ]);

        let response: AgentResponse = protocol.parse_agent_responce(resp).unwrap();

        assert_eq!(response.len(), 2);
        assert_eq!(response.calls()[0].arguments(), json!({"path": "a.rs"}));
        assert_eq!(response.calls()[1].arguments(), json!({"path": "b.rs"}));
    }

    #[test]
    fn protocol_generates_correct_endpoint_path() {
        let protocol = GoogleProtocol;

        let req = GeminiRequest {
            model: "gemini-1.5-flash".into(),
            ..Default::default()
        };

        let path = protocol.endpoint_path(&req);

        assert_eq!(path, "/models/gemini-1.5-flash:generateContent");
    }

    #[test]
    fn google_client_initializes_with_correct_base_url_and_headers() {
        unsafe {
            std::env::set_var("GEMINI_API_KEY", "test_gemini_key_123");
        }
        let client = GoogleClient::no_pool();
        assert_eq!(
            client.inner.config.base_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );

        assert_eq!(client.inner.config.timeout, Duration::from_secs(30));

        let auth_header = client
            .inner
            .config
            .headers
            .get("x-goog-api-key")
            .expect("Missing API key header");
        assert_eq!(auth_header.to_str().unwrap(), "test_gemini_key_123");
    }

    #[test]
    fn maps_payload_too_large_error() {
        let protocol = GoogleProtocol;
        let err = protocol.map_status_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Request payload size exceeds the limit".into(),
        );

        match err {
            GoogleError::TokenLimit { body } => {
                assert_eq!(body, "Request payload size exceeds the limit")
            }
            _ => panic!("Expected GoogleError::TokenLimit, got {:?}", err),
        }
    }

    #[test]
    fn maps_invalid_tool_call_error() {
        let protocol = GoogleProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "invalid function call formatting".into(),
        );

        match err {
            GoogleError::InvalidToolCall { body } => {
                assert_eq!(body, "invalid function call formatting")
            }
            _ => panic!("Expected GoogleError::InvalidToolCall, got {:?}", err),
        }
    }

    #[test]
    fn maps_generic_protocol_error_on_standard_bad_request() {
        let protocol = GoogleProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "missing required field 'contents'".into(),
        );

        match err {
            GoogleError::Protocol { status, body } => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(body, "missing required field 'contents'");
            }
            _ => panic!("Expected GoogleError::Protocol, got {:?}", err),
        }
    }
}