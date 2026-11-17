use clap::Parser;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::cmds::{investigate, memory, next_cmd};
use super::{Cli, Commands};

struct Router;

impl Router {
    async fn dispatch(cli: Cli) {
        match cli.command {
            Commands::Memory(args) => memory::run(args).await,
            Commands::NextCmd(args) => next_cmd::run(args).await,
            Commands::Investigate(args) => investigate::run(args).await,
        }
    }
}

pub fn run() {
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("logs");

    let file_appender = tracing_appender::rolling::never(log_dir, "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::EnvFilter::new(
            "warn,smart_terminal=debug",
        ))
        .try_init()
        .ok();

    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    tracing::info!("CMD Task Started");
    runtime.block_on(Router::dispatch(cli));
    tracing::info!("CMD Task Ended");
    runtime.shutdown_background();
}
