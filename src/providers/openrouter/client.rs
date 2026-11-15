use super::adapters::to_openrouter_model_string;
use crate::core::{
    AgentRequest, AgentResponse, AgentSession, AgentToolCall, LLMProvider, ProviderError,
};
use crate::providers::openai_compatible::{
    build_openai_request, build_openai_structured_request, LlmStructuredOutput, LlmToolCall,
    OpenAiError, OpenAiRequest, OpenAiResponse,
};
use crate::providers::{ClientConfig, GenericLlmClient, ProviderCodec};
use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::time::Duration;

pub type OpenRouterError = OpenAiError;

#[derive(Clone)]
pub struct OpenRouterProtocol;

impl ProviderCodec for OpenRouterProtocol {
    type Request = OpenAiRequest;
    type Response = OpenAiResponse;
    type Error = OpenAiError;

    fn build_complete_request(&self, req: &AgentRequest<'_>) -> Self::Request {
        build_openai_request(req, to_openrouter_model_string(req.model.get_name()))
    }

    fn build_structured_request(&self, session: &AgentSession, schema: Value) -> Self::Request {
        build_openai_structured_request(session, schema, "meta-llama/llama-3.3-70b-instruct:free")
    }

    fn parse_agent_responce(&self, res: Self::Response) -> Result<AgentResponse, Self::Error> {
        let call = LlmToolCall::try_from(res)?;
        Ok(AgentResponse::single(AgentToolCall::new(
            call.name, call.id, call.args, None,
        )))
    }

    fn parse_structured(&self, res: Self::Response) -> Result<Value, Self::Error> {
        let out = LlmStructuredOutput::try_from(res)?;
        Ok(out.value)
    }

    fn map_status_error(&self, status: StatusCode, body: String) -> Self::Error {
        if status == StatusCode::PAYLOAD_TOO_LARGE
            || (status == StatusCode::BAD_REQUEST
                && (body.contains("context_length_exceeded")
                    || body.contains("maximum context length")))
        {
            return OpenAiError::TokenLimit { body };
        }
        if status == StatusCode::BAD_REQUEST
            && (body.contains("tool_use_failed")
                || body.contains("output_parse_failed")
                || body.contains("tool_call")
                || body.contains("function"))
        {
            return OpenAiError::InvalidToolCall { body };
        }
        OpenAiError::Protocol { status, body }
    }

    fn map_network_error(&self, err: reqwest::Error) -> Self::Error {
        if err.is_timeout() {
            OpenAiError::Timeout { source: err }
        } else {
            OpenAiError::Network { source: err }
        }
    }
}

#[derive(Clone)]
pub struct OpenRouterClient {
    inner: GenericLlmClient<OpenRouterProtocol>,
}

impl OpenRouterClient {
    pub fn pooled() -> Self {
        Self::build(2)
    }

    pub fn no_pool() -> Self {
        Self::build(0)
    }

    fn build(_max_idle: usize) -> Self {
        let api_key = std::env::var("OPENROUTER_API_KEY").expect("Missing OPENROUTER_API_KEY");

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).expect("Invalid header value"),
        );
        headers.insert(
            HeaderName::from_static("http-referer"),
            HeaderValue::from_static("https://github.com/smart-terminal"),
        );
        headers.insert(
            HeaderName::from_static("x-title"),
            HeaderValue::from_static("Smart Terminal"),
        );

        let config = ClientConfig {
            base_url: "https://openrouter.ai/api/v1/chat/completions".into(),
            headers,
            timeout: Duration::from_secs(45),
        };

        Self {
            inner: GenericLlmClient::new(config, OpenRouterProtocol),
        }
    }
}

impl Default for OpenRouterClient {
    fn default() -> Self {
        Self::no_pool()
    }
}

impl LLMProvider for OpenRouterClient {
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
mod tests {
    use super::*;
    use serde_json::json;

    fn openrouter_response(
        tool_name: &str,
        tool_id: &str,
        arguments: serde_json::Value,
    ) -> OpenAiResponse {
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
    fn openrouter_client_initializes_with_correct_base_url_and_headers() {
        unsafe {
            std::env::set_var("OPENROUTER_API_KEY", "test_openrouter_key_123");
        }

        let client = OpenRouterClient::no_pool();

        assert_eq!(
            client.inner.config.base_url,
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(client.inner.config.timeout, Duration::from_secs(45));

        let auth_header = client
            .inner
            .config
            .headers
            .get(AUTHORIZATION)
            .expect("Missing Authorization header");

        assert_eq!(
            auth_header.to_str().unwrap(),
            "Bearer test_openrouter_key_123"
        );

        let referer = client
            .inner
            .config
            .headers
            .get("http-referer")
            .expect("Missing http-referer header");
        assert_eq!(referer.to_str().unwrap(), "https://github.com/smart-terminal");

        let title = client
            .inner
            .config
            .headers
            .get("x-title")
            .expect("Missing x-title header");
        assert_eq!(title.to_str().unwrap(), "Smart Terminal");
    }

    #[test]
    fn maps_payload_too_large_error() {
        let protocol = OpenRouterProtocol;
        let err = protocol.map_status_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Payload too large".into(),
        );

        match err {
            OpenAiError::TokenLimit { body } => {
                assert_eq!(body, "Payload too large");
            }
            _ => panic!("Expected OpenAiError::TokenLimit, got {:?}", err),
        }
    }

    #[test]
    fn maps_context_length_exceeded_error() {
        let protocol = OpenRouterProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "maximum context length exceeded".into(),
        );

        match err {
            OpenAiError::TokenLimit { body } => {
                assert_eq!(body, "maximum context length exceeded");
            }
            _ => panic!("Expected OpenAiError::TokenLimit, got {:?}", err),
        }
    }

    #[test]
    fn maps_invalid_tool_call_error() {
        let protocol = OpenRouterProtocol;
        let err = protocol.map_status_error(
            StatusCode::BAD_REQUEST,
            "invalid function tool_call parameters".into(),
        );

        match err {
            OpenAiError::InvalidToolCall { body } => {
                assert_eq!(body, "invalid function tool_call parameters");
            }
            _ => panic!("Expected OpenAiError::InvalidToolCall, got {:?}", err),
        }
    }

    #[test]
    fn maps_generic_protocol_error() {
        let protocol = OpenRouterProtocol;
        let err = protocol.map_status_error(StatusCode::BAD_REQUEST, "invalid input".into());

        match err {
            OpenAiError::Protocol { status, body } => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(body, "invalid input");
            }
            _ => panic!("Expected OpenAiError::Protocol, got {:?}", err),
        }
    }

    #[test]
    fn parse_tool_call_wraps_single_call_in_agent_response() {
        let protocol = OpenRouterProtocol;
        let resp = openrouter_response("read_file", "call_openrouter_1", json!({"path": "src/main.rs"}));
        let response = protocol.parse_agent_responce(resp).unwrap();
        assert_eq!(response.len(), 1);
        let call = &response.calls()[0];
        assert_eq!(call.name(), "read_file");
        assert_eq!(call.id(), "call_openrouter_1");
        assert_eq!(call.arguments(), json!({"path": "src/main.rs"}));
    }

    #[test]
    fn parse_stop_conclusion() {
        let protocol = OpenRouterProtocol;
        let resp: OpenAiResponse = serde_json::from_value(json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "All tasks completed."
                }
            }]
        })).unwrap();

        let response = protocol.parse_agent_responce(resp).unwrap();
        assert_eq!(response.len(), 1);
        let call = &response.calls()[0];
        assert_eq!(call.name(), "stop");
        assert_eq!(call.arguments(), json!("All tasks completed."));
    }
}
