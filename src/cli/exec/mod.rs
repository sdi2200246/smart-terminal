mod policy;
use super::cli::ExecArgs;
use policy::{Policy , Script};
use crate::agent::service::AgentService;
use crate::agent::responce::AgentResponse;
use crate::groq::client::GroqClient;
use tokio::sync::mpsc;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn render_success(stdout: &str) {
    println!("\x1b[32m✓ Success\x1b[0m");
    if !stdout.is_empty() {
        println!("{stdout}");
    }
}

fn render_error(stderr: &str) {
    eprintln!("\x1b[31m✗ Failed\x1b[0m");
    if !stderr.is_empty() {
        eprintln!("{stderr}");
    }
}

pub async fn run(args:ExecArgs){

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
    let tx = AgentService::spawn("Shell_Agent".into() , client);
    let (response_tx, mut response_rx) = mpsc::channel(1);

    let policy = Policy::select_policy(&args);
    let req = policy.create_req(args, response_tx);

    tx.send(req).await.unwrap();

    let response = response_rx.recv().await.unwrap();

    match response {
        AgentResponse::Success(value) => {
            let script: Script = serde_json::from_value(value).unwrap();

           let status = std::process::Command::new("bash")
                .arg("-c")
                .arg(script.script)
                .status()
                .expect("failed to execute script");

            if status.success() {
                render_success("");
            } else {
                render_error("script failed");
            }
        }
        AgentResponse::Error(e) => {
            render_error(&e.to_string());
            std::process::exit(1);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn run_with_align_true() {
        let args = ExecArgs {
            prompt: "the most freequent pair of commnads?".to_string(),
            align: true,
        };
        run(args).await;
    }

    #[tokio::test]
    async fn run_with_align_false() {
        let args = ExecArgs {
            prompt: "show me all structs definitions and their fields in the current direcotiry and  edirect them in a file structs.txt".to_string(),
            align: false,
        };
        run(args).await;
    }
}       