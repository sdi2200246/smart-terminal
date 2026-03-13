use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "An AI-powered terminal assistant")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(Subcommand)]
pub enum Commands {
    /// Suggest the next command based on context
    NextCmd(NextCmdArgs),

    /// Execute a command with AI assistance
    Exec(ExecArgs),
}

#[derive(Args)]
pub struct NextCmdArgs;

#[derive(Args)]
pub struct ExecArgs {
    /// The prompt describing what you want to execute
    pub prompt: String,

    /// Run autonomously without confirmation or align first
    #[arg(long, default_value_t = true)]
    pub align: bool,
}