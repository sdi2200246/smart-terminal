use crate::agent::patterns::tool_engine::ToolPermissionMode;
use clap::{Args, Parser, Subcommand};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CliPermissionMode {
    Manual,
    Auto,
}

impl clap::ValueEnum for CliPermissionMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Manual, Self::Auto]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::Manual => Some(clap::builder::PossibleValue::new("manual")),
            Self::Auto => Some(clap::builder::PossibleValue::new("auto")),
        }
    }
}

impl From<CliPermissionMode> for ToolPermissionMode {
    fn from(value: CliPermissionMode) -> Self {
        match value {
            CliPermissionMode::Manual => ToolPermissionMode::Prompt,
            CliPermissionMode::Auto => ToolPermissionMode::Allow,
        }
    }
}

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
    /// Terminal buffer or a prompt describing the expected command.
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
    /// Default permission mode for mutating tool calls during this run.
    #[arg(long = "perms", value_enum, default_value_t = CliPermissionMode::Manual)]
    pub tool_permission: CliPermissionMode,
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
        let cli = Cli::try_parse_from(["agent", "investigate", "--one-off", "--perms", "auto", "question"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Investigate(InvestigateArgs {
                one_off: true,
                tool_permission: CliPermissionMode::Auto,
                question: Some(question),
                action: None,
            }) if question == "question"
        ));
    }

    #[test]
    fn parses_investigation_session_actions() {
        let cli = Cli::try_parse_from(["agent", "investigate", "--perms", "manual", "session", "clear"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Investigate(InvestigateArgs {
                tool_permission: CliPermissionMode::Manual,
                action: Some(InvestigateAction::Session(InvestigationSessionArgs {
                    action: InvestigationSessionAction::Clear
                })),
                ..
            })
        ));
    }
}
