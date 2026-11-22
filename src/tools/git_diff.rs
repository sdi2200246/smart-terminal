use super::error::ToolError;
use crate::core::{Capability, ToolMetaData};
use serde_json::Value;
use std::process::Command;

define_tool! {
    pub struct GitDiffStaged;
    /// Returns the staged changes (git diff --staged) of the current repository.
    fn git_diff_staged() -> Result<String, ToolError> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--staged")
            .arg("--stat")
            .arg("-p")
            .output()
            .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.is_empty() {
            return Ok("no staged changes".to_string());
        }

        Ok(stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_git_diff_staged_runs() {
        let tool = GitDiffStaged;
        let result = tool
            .execute(json!({}))
            .expect("git diff --staged should run");
        assert!(!result.is_empty());
    }
}
