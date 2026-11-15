mod agent_setup;
mod cli;
pub mod cmds;
mod presenters;
mod suggestion_handoff;

pub use agent_setup::CliToolProvider;
pub use cli::{
    Cli, CliPermissionMode, Commands, InvestigateAction, InvestigateArgs,
    InvestigationSessionAction, InvestigationSessionArgs, MemoryAction, MemoryArgs, NextCmdArgs,
};
pub use presenters::Presenter;
pub use suggestion_handoff::write_recommendation;

