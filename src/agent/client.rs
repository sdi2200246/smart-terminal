use tokio::sync::mpsc::{self};
use super::service::AgentService;
use super::loops::traits::AgentLoop;
use super::request::AgentRequest;
use super::responce::AgentResponse;
use crate::interfaces::llm_client::LLMProvider;

pub struct AgentClient {
    tx: mpsc::Sender<AgentRequest>,
    response_tx: mpsc::Sender<AgentResponse>,
    response_rx: mpsc::Receiver<AgentResponse>,
}

impl AgentClient {
    pub fn new<P, L>(name: impl Into<String>, provider: P, agent_loop: L) -> Self
    where
        P: LLMProvider + Send + 'static,
        L: AgentLoop + Send + 'static,
    {
        let tx = AgentService::spawn(name.into(), provider, agent_loop);
        let (response_tx, response_rx) = mpsc::channel(1);
        Self { tx, response_tx, response_rx }
    }

    pub fn response_sender(&self) -> mpsc::Sender<AgentResponse> {
        self.response_tx.clone()
    }

    pub async fn execute_request(&mut self, req: AgentRequest) -> AgentResponse {
        self.tx.send(req).await.unwrap();
        self.response_rx.recv().await.unwrap()
    }
}