use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use crate::core::session::AgentToolCall;

mod render;

pub struct Presenter {
    rx: UnboundedReceiver<AgentToolCall>,
}

impl Presenter {
    pub fn new() -> (Self, UnboundedSender<AgentToolCall>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { rx }, tx)
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(call) = self.rx.recv().await {
                println!("{}", render::format_call(&call));
            }
        })
    }
}