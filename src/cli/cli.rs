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
    /// Investigate once without loading or changing the project's main session
    #[arg(long)]
    pub one_off: bool,
    /// The question to investigate
    pub question: Option<String>,
    #[command(subcommand)]
    pub action: Option<InvestigateAction>,
}

#[derive(Subcommand)]
pub enum InvestigateAction {
    /// Manage the project's main investigation session
    Session(InvestigationSessionArgs),
}

#[derive(Args)]
pub struct InvestigationSessionArgs {
    #[command(subcommand)]
    pub action: InvestigationSessionAction,
}

#[derive(Subcommand)]
pub enum InvestigationSessionAction {
    /// Delete the project's saved investigation session
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_off_investigation() {
        let cli = Cli::try_parse_from(["agent", "investigate", "--one-off", "question"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Investigate(InvestigateArgs {
                one_off: true,
                question: Some(question),
                action: None,
            }) if question == "question"
        ));
    }

    #[test]
    fn parses_investigation_session_actions() {
        let cli = Cli::try_parse_from(["agent", "investigate", "session", "clear"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Investigate(InvestigateArgs {
                action: Some(InvestigateAction::Session(InvestigationSessionArgs {
                    action: InvestigationSessionAction::Clear
                })),
                ..
            })
        ));
    }
}
