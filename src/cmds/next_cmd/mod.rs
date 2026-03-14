mod policy;
use policy::{Policy , Command};
use crate::agent::service::AgentService;
use crate::agent::responce::AgentResponse;
use crate::cmds::cli::NextCmdArgs;
use crate::groq::client::GroqClient;

use tokio::sync::mpsc;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn run(args:NextCmdArgs){

    let file_appender = tracing_appender::rolling::daily("./logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .with(tracing_subscriber::EnvFilter::new("warn,smart_terminal=debug"))
        .try_init()
        .ok();

    let client = GroqClient::default();
    let tx = AgentService::spawn("NextCMD_Agent".into() , client);
    let (response_tx, mut response_rx) = mpsc::channel(1);

    let policy = Policy::select_policy();
    let req = policy.create_req(args, response_tx);

    tx.send(req).await.unwrap();

    let response = response_rx.recv().await.unwrap();

    match response {
        AgentResponse::Success(value) => {
            let suggestion: Command = serde_json::from_value(value).unwrap();

            println!("{}" , &suggestion.cmd);
            println!("{}" , &suggestion.man);

        }
        _=>{}
    }
}
 