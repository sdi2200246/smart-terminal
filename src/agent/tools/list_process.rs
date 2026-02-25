use std::process::Command;
use serde_json::Value;
use super::error::ToolError;
use super::capability::{Capability , ToolFunction};


pub fn list_processes(_args:Value) -> Result<String, ToolError> {
    let output = Command::new("top")
        .args([
            "-l", "1",
            "-stats", "pid,command",
        ])
        .output()
        .map_err(|_| ToolError::Execution)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

pub struct ProcessList;

impl Capability for ProcessList {
    fn name(&self) -> &'static str {
        "running_processes"
    }

    fn to_protocol(&self) -> ToolFunction{
        ToolFunction {
            name: self.name().into(),
            description:"Returns running processes with names and pids".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }
    fn execute(&self, args: Value) -> Result<String , ToolError> {
        list_processes(args)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{thread, time::Duration};

    #[test]
    fn test_list_processes_runs_and_prints() {
        let result = list_processes(json!({}))
            .expect("list_processes should run");

        println!("=== list_processes output ===");
        println!("{}", result);

        thread::sleep(Duration::from_secs(1));
    }
}