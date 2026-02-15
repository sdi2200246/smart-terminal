use std::process::Command;
use serde_json::Value;
use super::capability::Capability;
use super::error::ToolError;
use crate::protocol::tool::{Tool, ToolFunction};

pub fn git_diff_staged(_args: Value) -> Result<Value, ToolError> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .output()
        .map_err(|_| ToolError::Execution)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(Value::String(stdout))
}

pub struct GitDiffStaged;

impl Capability for GitDiffStaged {
    fn name(&self) -> &'static str {
        "git_diff_staged"
    }

    fn to_protocol(&self) -> Tool {
        Tool::factory(
            ToolFunction {
                name: self.name().into(),
                description: Some(
                    "Returns the staged changes (git diff --staged) of the current repository".into()
                ),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                arguments: None,
            }
        )
    }

    fn execute(&self, args: Value) -> Result<Value, ToolError> {
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
