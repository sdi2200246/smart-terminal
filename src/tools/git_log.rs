use std::process::Command;
use serde_json::Value;
use crate::core::capability::{Capability, ToolFunction};
use super::error::ToolError;

pub fn git_log(_args: Value) -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-10", "--pretty=format:%s"])
        .output()
        .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub struct GitLog;

impl Capability for GitLog {
    fn name(&self) -> &'static str {
        "git_log"
    }

    fn metadata(&self) -> ToolFunction {
        ToolFunction {
            name: self.name().into(),
            description: "Returns the last 10 commit messages of the current repository".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        git_log(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_git_log_runs() {
        let result = git_log(json!({}))
            .expect("git log should run");

        println!("{}", result);
    }
}