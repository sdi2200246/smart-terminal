use clap::{Parser, Subcommand, Args};
use super::adapters::Mode;

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
pub struct NextCmdArgs{
    ///Terminal buffer or a promt describing the expected command
    pub buffer: String,
}

#[derive(Args)]
pub struct ExecArgs {
    /// The prompt describing what you want to execute
    pub prompt: String,

   #[arg(long, value_enum, default_value = "auto")]
    pub mode: Mode,
}