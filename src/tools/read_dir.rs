use serde_json::Value;
use serde::Deserialize;
use schemars::JsonSchema;
use crate::core::capability::{Capability, ToolFunction};
use crate::utils::FlatSchema;
use super::error::ToolError;

#[derive(JsonSchema, Deserialize, Debug)]
struct ReadDirArgs {
    /// The directory path to read. Use '.' for current directory.
    pub path: String,
    /// Whether to read recursively. Defaults to false.
    pub recursive: bool,
}
impl FlatSchema for ReadDirArgs {}

pub struct ReadDir;

impl Capability for ReadDir {
    fn name(&self) -> &'static str {
        "read_dir"
    }

    fn metadata(&self) -> ToolFunction {
        ToolFunction {
            name: self.name().into(),
            description: "Read the contents of a directory. \
                Use this to explore the filesystem before writing your script. \
                Set recursive to true to include all subdirectories. \
                Automatically excludes target/ and .git/ directories. \
                Only for reading directory structure — not for reading file contents.".into(),
            parameters: ReadDirArgs::schema(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: ReadDirArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

        let output = if args.recursive {
            std::process::Command::new("find")
                .arg(&args.path)
                .arg("-not").arg("-path").arg("*/target/*")
                .arg("-not").arg("-path").arg("*/.git/*")
                .output()
        } else {
            std::process::Command::new("ls")
                .arg("-la")
                .arg(&args.path)
                .output()
        }
        .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(ToolError::ToolExecution {
                source: anyhow::anyhow!("{stderr}"),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_read_current_dir() {
        let tool = ReadDir;
        let result = tool.execute(json!({ "path": ".", "recursive": false }));
        assert!(result.is_ok());
        println!("{}", result.unwrap());
    }

    #[test]
    fn test_read_src_recursive() {
        let tool = ReadDir;
        let result = tool.execute(json!({ "path": "./src", "recursive": true }));
        assert!(result.is_ok());
        println!("{}", result.unwrap());
    }

    #[test]
    fn test_invalid_path_returns_error() {
        let tool = ReadDir;
        let result = tool.execute(json!({ "path": "./nonexistent", "recursive": false }));
        assert!(result.is_err());
    }
}