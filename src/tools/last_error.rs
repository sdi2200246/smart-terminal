use super::error::ToolError;
use crate::core::capability::{Capability, ToolMetaData};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

const MAX_CHARS: usize = 200;

#[derive(Serialize, Deserialize)]
struct ReadLastErrorOutput {
    /// The actual error content.
    pub content: String,
    /// Marks true if the output of the error was trancuated to max chars the system supports.
    pub truncated: bool,
}

pub struct ReadLastError;

impl Capability for ReadLastError {
    fn name(&self) -> &'static str {
        "read_last_error"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Read the stderr output of the user's last shell command,\
                Returns the last 200 characters at most. `truncated` indicates \
                whether earlier output was cut off. Empty content means the last \
                command produced no errors."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, _args: Value) -> Result<String, ToolError> {
        let path = std::env::var("ERR_LAST").map_err(|_| ToolError::ToolExecution {
            source: anyhow::anyhow!(
                "[ERROR] ERR_LAST env var not set — is the shell hook installed?"
            ),
        })?;

        let raw = fs::read_to_string(&path).map_err(|e| ToolError::ToolExecution {
            source: anyhow::anyhow!("[ERROR] reading {}: {}", path, e),
        })?;

        let total_chars = raw.chars().count();
        let truncated = total_chars > MAX_CHARS;

        let content = if truncated {
            let skip = total_chars - MAX_CHARS;
            let start_byte = raw.char_indices().nth(skip).map(|(i, _)| i).unwrap_or(0);
            raw[start_byte..].to_string()
        } else {
            raw
        };

        let output = ReadLastErrorOutput { content, truncated };

        Ok(serde_json::to_string(&output).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    // NOTE: these tests mutate a process-global env var, so they can race
    // under cargo test's parallel runner. Run with --test-threads=1
    // or serialize them with the serial_test crate.

    #[test]
    fn returns_full_content_when_under_limit() {
        let f = temp_file("zsh: command not found: l\n");
        unsafe {
            std::env::set_var("ERR_LAST", f.path());
        }
        let result = ReadLastError.execute(json!({})).unwrap();
        let out: ReadLastErrorOutput = serde_json::from_str(&result).unwrap();
        assert!(!out.truncated);
        assert!(out.content.contains("command not found"));
    }

    #[test]
    fn truncates_to_last_200_chars() {
        let long = "a".repeat(300) + &"b".repeat(200);
        let f = temp_file(&long);
        unsafe {
            std::env::set_var("ERR_LAST", f.path());
        }
        let result = ReadLastError.execute(json!({})).unwrap();
        let out: ReadLastErrorOutput = serde_json::from_str(&result).unwrap();
        assert!(out.truncated);
        assert_eq!(out.content, "b".repeat(200));
    }

    #[test]
    fn missing_env_var_errors() {
        unsafe {
            std::env::remove_var("ERR_LAST");
        }
        let result = ReadLastError.execute(json!({}));
        assert!(result.is_err());
    }
}
