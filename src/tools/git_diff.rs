use std::process::Command;
use serde_json::Value;
use crate::core::capability::{Capability , ToolFunction};
use super::error::ToolError;

pub fn git_diff_staged(_args: Value) -> Result<String, ToolError> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--staged").arg("--stat").arg("-p")
        .output()
        .map_err(|e| ToolError::ToolExecution { source: (e.into())})?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stdout.is_empty() {
        return Ok("no staged changes".to_string());
    }

    Ok(stdout)
}

pub struct GitDiffStaged;

impl Capability for GitDiffStaged {
    fn name(&self) -> &'static str {
        "git_diff_staged"
    }

    fn metadata(&self) -> ToolFunction {
        ToolFunction {
            name: self.name().into(),
            description:"Returns the staged changes (git diff --staged) of the current repository".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        git_diff_staged(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_git_diff_staged_runs() {
        let result = git_diff_staged(json!({}))
            .expect("git diff --staged should run");

        println!("{result}");
    }
}
