#[macro_use]
pub mod macros;
use serde::Serialize;

mod ask_user;
mod bash;
mod docker;
mod error;
mod git_diff;
mod last_error;
mod read_dir;
mod read_file;

pub use ask_user::{AskUser, ModelQuestion};
pub use bash::Bash;
pub use docker::Docker;
pub use error::ToolError;
pub use git_diff::GitDiffStaged;
pub use last_error::ReadLastError;
pub use macros::NoArgs;
pub use read_dir::ReadDir;
pub use read_file::ReadFile;

pub(crate) fn json_output<T: Serialize>(output: &T) -> Result<String, ToolError> {
    serde_json::to_string(output).map_err(|source| ToolError::ToolExecution {
        source: source.into(),
    })
}
