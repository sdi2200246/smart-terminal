use clap::ValueEnum;
use super::cli::ExecArgs;
use super::cli::NextCmdArgs;
use crate::agent::request::{AgentIntent , AgentMode};

#[derive(ValueEnum, Clone, Debug)]
pub enum Mode {
    /// Run autonomously without confirmation
    Auto,
    /// Align with user before executing
    Align,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Auto
    }
}

impl From<ExecArgs> for AgentIntent {
    fn from(args: ExecArgs) -> Self {
        AgentIntent {
            prompt: args.prompt,
            mode: match args.mode {
                Mode::Auto => AgentMode::Auto,
                Mode::Align => AgentMode::Align,
            },
        }
    }
}

impl From<NextCmdArgs> for AgentIntent {
    fn from(args: NextCmdArgs) -> Self {
        AgentIntent {
            prompt: args.buffer,
            mode: AgentMode::Auto,
        }
    }
}