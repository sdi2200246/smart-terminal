use std::process::Command;
use serde_json::Value;
use super::capability::{Capability,ToolFunction};
use super::error::ToolError;

pub fn git_status(_args: Value) -> Result<String, ToolError> {
    let output = Command::new("git")
        .arg("status").arg("--porcelain")
        .output()
        .map_err(|_| ToolError::Execution)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(stdout)
}

pub struct GitStatus;

impl Capability for GitStatus {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn to_protocol(&self) -> ToolFunction{
        ToolFunction {
                name: self.name().into(),
                description: "Returns the current status of the current github repo".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
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

        println!("{}",result);
    }
}
