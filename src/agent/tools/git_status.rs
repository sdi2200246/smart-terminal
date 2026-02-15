use std::process::Command;
use serde_json::Value;
use super::capability::Capability;
use super::error::ToolError;
use crate::protocol::tool::{Tool, ToolFunction};

pub fn git_status(_args: Value) -> Result<Value, ToolError> {
    let output = Command::new("git")
        .arg("status")
        .output()
        .map_err(|_| ToolError::Execution)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(Value::String(stdout))
}

pub struct GitStatus;

impl Capability for GitStatus {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn to_protocol(&self) -> Tool{
        Tool::factory(
        ToolFunction {
                name: self.name().into(),
                description: Some(
                    "Returns the current status of the current github repo".into()
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
        git_status(args)
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_git_status_runs() {
        let result = git_status(json!({}))
            .expect("git status should run");

        println!("{result}");
    }
}
