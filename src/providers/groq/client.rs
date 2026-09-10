use super::error::GroqError;
use super::protocol::request::GroqRequest;
use super::protocol::responce::{GroqResponse, LlmStructuredOutput, LlmToolCall};
use crate::core::error::ProviderError;
use crate::core::llm_client::{AgentRequest, LLMProvider};
use crate::core::session::{AgentSession, AgentToolCall};
use crate::providers::client::{GenericLlmClient, ProviderCodec , ClientConfig };
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct GroqProtocol;

impl ProviderCodec for GroqProtocol {
    type Request = GroqRequest;
    type Response = GroqResponse;
    type Error = GroqError;

    fn build_complete_request(&self, req: &AgentRequest<'_>) -> Self::Request {
        GroqRequest::from(req)
    }

    fn build_structured_request(&self, session: &AgentSession, schema: Value) -> Self::Request {
        GroqRequest::structured(session, schema)
    }

    fn parse_tool_call(&self, res: Self::Response) -> Result<AgentToolCall, Self::Error> {
        let call = LlmToolCall::try_from(res)?;
        Ok(AgentToolCall::new(call.name, call.id, call.args, None))
    }

    fn parse_structured(&self, res: Self::Response) -> Result<Value, Self::Error> {
        let out = LlmStructuredOutput::try_from(res)?;
        Ok(out.value)
    }

    fn map_status_error(&self, status: StatusCode, body: String) -> Self::Error {
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return GroqError::TokenLimit { body };
        }
        if status == StatusCode::BAD_REQUEST
            && (body.contains("tool_use_failed") || body.contains("output_parse_failed"))
        {
            return GroqError::InvalidToolCall { body };
        }
        GroqError::Protocol { status, body }
    }

    fn map_network_error(&self, err: reqwest::Error) -> Self::Error {
        if err.is_timeout() {
            GroqError::Timeout { source: err }
        } else {
            GroqError::Network { source: err }
        }
    }
}

#[derive(Clone)]
pub struct GroqClient {
    inner: GenericLlmClient<GroqProtocol>,
}

    impl GroqClient {
        pub fn pooled() -> Self { Self::build(2) }
        pub fn no_pool() -> Self { Self::build(0) }

        fn build(max_idle: usize) -> Self {
            let api_key = std::env::var("GROQ_API_KEY").expect("Missing GROQ_API_KEY");

            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}")).expect("Invalid header value"),
            );

            let config = ClientConfig {
                base_url: "https://api.groq.com/openai/v1/chat/completions".into(),
                headers,
                timeout: Duration::from_secs(30),
            };

            Self {
                inner: GenericLlmClient::new(config, GroqProtocol{}),
            }
        }
    }

impl Default for GroqClient {
    fn default() -> Self { Self::no_pool() }
}

impl LLMProvider for GroqClient {
    async fn complete(&self, request: AgentRequest<'_>) -> Result<AgentToolCall, ProviderError> {
        self.inner.run_complete(&request).await
    }

    async fn complete_structured(&self, session: &AgentSession, schema: Value) -> Result<Value, ProviderError> {
        self.inner.run_complete_structured(session, schema).await
    }
}

#[cfg(test)]
mod unit {
    use super::*;
    use crate::providers::groq::protocol::request::GroqRequest;
    use crate::providers::groq::protocol::responce::{GroqResponse, LlmToolCall};
    use crate::core::session::AgentToolCall;
    use reqwest::header::AUTHORIZATION;
    use reqwest::StatusCode;
    use serde_json::json;

    fn groq_response(
        tool_name: &str,
        tool_id: &str,
        arguments: serde_json::Value,
    ) -> GroqResponse {
        serde_json::from_value(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": tool_id,
                        "type": "function",
                        "function": {
                            "name": tool_name,
                            "arguments": arguments.to_string()
                        }
                    }]
                }
            }]
        }))
        .unwrap()
    }

    #[test]
    fn final_answer_parses_as_tool_call() {
        let resp = groq_response("final_answer", "call_test", json!({"result":"42"}));
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "final_answer");
        assert_eq!(call.args, json!({"result": "42"}));
    }

    #[test]
    fn regular_tool_parses_as_tool_call() {
        let resp = groq_response("git_status", "call_test", json!({"path":"."}));
        let call = LlmToolCall::try_from(resp).unwrap();
        assert_eq!(call.name, "git_status");
        assert_eq!(call.args, json!({"path": "."}));
    }

    #[test]
    fn llm_tool_call_converts_to_agent_tool_call() {
        let resp = groq_response("final_answer", "call_test", json!({"result":"42"}));
        let llm_call = LlmToolCall::try_from(resp).unwrap();
        let agent_call = AgentToolCall::new(llm_call.name, llm_call.id, llm_call.args, None);
        assert_eq!(agent_call.name(), "final_answer");
        assert_eq!(agent_call.arguments().clone(), json!({"result": "42"}));
    }

    #[test]
    fn protocol_uses_default_empty_endpoint_path() {
        let protocol = GroqProtocol;
        let req = GroqRequest {
            ..Default::default()
        };

        let path = protocol.endpoint_path(&req);
        assert_eq!(path, "");
    }

    #[test]
    fn groq_client_initializes_with_correct_base_url_and_headers() {
        unsafe { std::env::set_var("GROQ_API_KEY", "test_groq_key_123"); }

        let client = GroqClient::no_pool();

        assert_eq!(
            client.inner.config.base_url,
            "https://api.groq.com/openai/v1/chat/completions"
        );

        assert_eq!(
            client.inner.config.timeout,
            Duration::from_secs(30)
        );

        let auth_header = client
            .inner
            .config
            .headers
            .get(AUTHORIZATION)
            .expect("Missing Authorization header");

        assert_eq!(auth_header.to_str().unwrap(), "Bearer test_groq_key_123");
    }

    #[test]
    fn maps_payload_too_large_error() {
        let protocol = GroqProtocol;
        let err = protocol.map_status_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Request context limit exceeded".into(),
        );

        match err {
            GroqError::TokenLimit { body } => {
                assert_eq!(body, "Request context limit exceeded")
            }
            _ => panic!("Expected GroqError::TokenLimit, got {:?}", err),
        }
    }

    #[test]
    fn maps_invalid_tool_call_error_tool_use_failed() {
        let protocol = GroqProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "Failed due to tool_use_failed during execution".into(),
        );

        match err {
            GroqError::InvalidToolCall { body } => {
                assert_eq!(body, "Failed due to tool_use_failed during execution")
            }
            _ => panic!("Expected GroqError::InvalidToolCall, got {:?}", err),
        }
    }

    #[test]
    fn maps_invalid_tool_call_error_output_parse_failed() {
        let protocol = GroqProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "Failed due to output_parse_failed in schema".into(),
        );

        match err {
            GroqError::InvalidToolCall { body } => {
                assert_eq!(body, "Failed due to output_parse_failed in schema")
            }
            _ => panic!("Expected GroqError::InvalidToolCall, got {:?}", err),
        }
    }

    #[test]
    fn maps_generic_protocol_error_on_standard_bad_request() {
        let protocol = GroqProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "invalid request parameters".into(),
        );

        match err {
            GroqError::Protocol { status, body } => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(body, "invalid request parameters");
            }
            _ => panic!("Expected GroqError::Protocol, got {:?}", err),
        }
    }
}