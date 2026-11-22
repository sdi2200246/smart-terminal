use super::error::ToolError;
use serde::{Deserialize, Serialize};
use std::fs;

#[cfg(test)]
use crate::core::Capability;

const MAX_CHARS: usize = 200;

#[derive(Debug, Deserialize, Serialize)]
struct ReadLastErrorOutput {
    /// The actual error content.
    pub content: String,
    /// Marks true if the output of the error was trancuated to max chars the system supports.
    pub truncated: bool,
}

define_tool! {
    pub struct ReadLastError;
    guidance = "Read the captured stderr before answering when the user asks why a command failed, retries a likely failed command, or says a previously suggested command did not work. Base the correction on the captured error rather than guessing.";
    /// Read the stderr output of the user's last shell command.
    /// Returns JSON with the last 200 characters at most. `truncated` indicates
    /// whether earlier output was cut off. Empty content means the last command
    /// produced no errors.
    fn read_last_error() -> Result<String, ToolError> {
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
        super::json_output(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::io::Write;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_temp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn missing_env_var_errors() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("ERR_LAST") };
        let tool = ReadLastError;
        let result = tool.execute(json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn returns_full_content_when_under_limit() {
        let _lock = ENV_LOCK.lock().unwrap();
        let f = write_temp("short error message");
        unsafe { std::env::set_var("ERR_LAST", f.path()) };

        let tool = ReadLastError;
        let result = tool.execute(json!({})).unwrap();
        let output: ReadLastErrorOutput = serde_json::from_str(&result).unwrap();

        assert_eq!(output.content, "short error message");
        assert!(!output.truncated);

        unsafe { std::env::remove_var("ERR_LAST") };
    }

    #[test]
    fn truncates_to_last_200_chars() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prefix = "a".repeat(100);
        let tail = "b".repeat(200);
        let full = format!("{}{}", prefix, tail);

        let f = write_temp(&full);
        unsafe { std::env::set_var("ERR_LAST", f.path()) };

        let tool = ReadLastError;
        let result = tool.execute(json!({})).unwrap();
        let output: ReadLastErrorOutput = serde_json::from_str(&result).unwrap();

        assert_eq!(output.content, tail);
        assert_eq!(output.content.chars().count(), 200);
        assert!(output.truncated);

        unsafe { std::env::remove_var("ERR_LAST") };
    }

    #[test]
    fn serializes_empty_error_output() {
        let output = ReadLastErrorOutput {
            content: String::new(),
            truncated: false,
        };

        let result = crate::tools::json_output(&output).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&result).unwrap(),
            serde_json::json!({"content": "", "truncated": false})
        );
    }
}
