use smart_terminal::cli::{exec , next_cmd};
use smart_terminal::cli::cli::{Cli , Commands};
use clap::Parser;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct Router;

impl Router {
    pub async fn dispatch(cli: Cli) {
        match cli.command {
            Commands::NextCmd(args) => next_cmd::run(args).await,
            Commands::Exec(args) => exec::run(args).await,
        }
    }
}
#[tokio::main]
async fn main() {

    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("logs");

    let file_appender = tracing_appender::rolling::daily(log_dir, "app.log");
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


    let cli = Cli::parse();
    Router::dispatch(cli).await;
}
