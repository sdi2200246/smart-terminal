use smart_terminal::cmds::{exec , next_cmd};
use smart_terminal::cmds::cli::{Cli , Commands};
use clap::Parser;

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
    let cli = Cli::parse();
    Router::dispatch(cli).await;
}
