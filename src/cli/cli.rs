use clap::{Args, Parser, Subcommand};

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
    /// Manipulate the memory of you assistant
    Memory(MemoryArgs),
    /// Investigate a question about the project or environment
    Investigate(InvestigateArgs),
}

#[derive(Args)]
pub struct NextCmdArgs {
    ///Terminal buffer or a promt describing the expected command
    pub buffer: String,
}

#[derive(Args)]
pub struct MemoryArgs {
    #[command(subcommand)]
    pub action: MemoryAction,
}

#[derive(Subcommand)]
pub enum MemoryAction {
    /// Register current directory for memory
    Init,
    /// Unregister and delete memory for current directory
    Delete,
    /// Wipe interactions but keep the folder registered
    Clear,
    /// Print the current folder's stored interactions
    Show,
}

#[derive(Args)]
pub struct InvestigateArgs {
    /// The question to investigate
    pub question: String,
}
