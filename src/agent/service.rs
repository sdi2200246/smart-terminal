use tokio::sync::mpsc::{self, Receiver, Sender};
use crate::interfaces::llm_client::LLMProvider;
use super::request::AgentRequest;
use super::responce::AgentResponse;
use super::loops::traits::AgentLoop;

pub struct AgentService<P: LLMProvider , L:AgentLoop> {
    id: String,
    rx: Receiver<AgentRequest>,
    provider:P,
    agent_loop:L
}

impl<P: LLMProvider , L:AgentLoop> AgentService<P , L> {

    pub fn new(rx: Receiver<AgentRequest>, provider: P , id:String , agent_loop:L) -> Self {
        AgentService {id, rx, provider ,agent_loop }
    }

    pub fn spawn(id : String , provider: P , agent_loop:L) -> Sender<AgentRequest>
    where
        P: Send + 'static,
        L: Send + 'static,
    {
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            AgentService::new( rx, provider , id , agent_loop).run().await;
        });
        tx
    }

    #[tracing::instrument(skip(self), fields(agent_id = %self.id))]
    async fn run(mut self) {
        while let Some(req) = self.rx.recv().await {
            let pipe = req.pipe.clone();
            let response = match self.agent_loop.agent_loop(req, &mut self.provider).await {
                Ok(value) => {
                    tracing::info!("agent request completed successfully");
                    AgentResponse::Success(value)
                },
                Err(e) => {
                    tracing::error!(error = ?e,"agent request failed");
                    AgentResponse::Error(e)
                }
            };
            let _ = pipe.send(response).await;
        }
    }
}

