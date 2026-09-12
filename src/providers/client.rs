use crate::core::error::ProviderError;
use crate::core::llm_client::AgentRequest;
use crate::core::session::AgentSession;
use crate::core::responce::{AgentResponse};
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::time::Duration;

pub trait ProviderCodec: Clone + Send + Sync + 'static {
    type Request: Serialize + Send + Sync;
    type Response: DeserializeOwned + Send + Sync;
    type Error: Into<ProviderError> + Send + Sync;

    fn endpoint_path(&self, _req: &Self::Request) -> String {
        String::new()
    }

    fn build_complete_request(&self, req: &AgentRequest<'_>) -> Self::Request;
    fn build_structured_request(&self, session: &AgentSession, schema: Value) -> Self::Request;
    fn parse_agent_responce(&self, res: Self::Response) -> Result<AgentResponse, Self::Error>;
    fn parse_structured(&self, res: Self::Response) -> Result<Value, Self::Error>;
    fn map_status_error(&self, status: StatusCode, body: String) -> Self::Error;
    fn map_network_error(&self, err: reqwest::Error) -> Self::Error;
}

#[derive(Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub headers: HeaderMap,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct GenericLlmClient<P: ProviderCodec> {
    http_client: Client,
    pub config: ClientConfig,
    protocol: P,
}

impl<P: ProviderCodec> GenericLlmClient<P> {
    pub fn new(config: ClientConfig, protocol: P) -> Self {
        let http_client = reqwest::Client::builder()
            .default_headers(config.headers.clone())
            .timeout(config.timeout)
            .build()
            .expect("failed to initialize reqwest client");

        Self {
            http_client,
            config,
            protocol,
        }
    }

    async fn call_llm(&self, req: &P::Request) -> Result<P::Response, P::Error> {
        let url = format!(
            "{}{}",
            self.config.base_url,
            self.protocol.endpoint_path(req)
        );
        let res = self
            .http_client
            .post(url)
            .json(req)
            .send()
            .await
            .map_err(|e| self.protocol.map_network_error(e))?;

        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(self.protocol.map_status_error(status, body));
        }

        res.json::<P::Response>()
            .await
            .map_err(|e| self.protocol.map_network_error(e))
    }

    pub async fn run_complete(
        &self,
        request: &AgentRequest<'_>,
    ) -> Result<AgentResponse, ProviderError> {
        let req = self.protocol.build_complete_request(request);
        let res = self.call_llm(&req).await.map_err(Into::into)?;
        self.protocol.parse_agent_responce(res).map_err(Into::into)
    }

    pub async fn run_complete_structured(
        &self,
        session: &AgentSession,
        schema: Value,
    ) -> Result<Value, ProviderError> {
        let req = self.protocol.build_structured_request(session, schema);
        let res = self.call_llm(&req).await.map_err(Into::into)?;
        self.protocol.parse_structured(res).map_err(Into::into)
    }
}
