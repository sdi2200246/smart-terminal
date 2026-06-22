use super::error::ToolError;
use crate::core::capability::{Capability, ToolMetaData};
use crate::utils::FlatSchema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

#[derive(JsonSchema, Deserialize, Debug)]
struct ReadFileArgs {
    /// Path to the file to read, relative to cwd.
    pub path: String,
    /// First line to include (1-indexed). Defaults to 1.
    pub start: Option<usize>,
    /// Last line to include (1-indexed, inclusive). Defaults to end of file.
    pub end: Option<usize>,
}
impl FlatSchema for ReadFileArgs {}

#[derive(Serialize, Deserialize)]
struct ReadFileOutput {
    pub content: String,
    pub total_lines: usize,
    pub range: [usize; 2],
}

pub struct ReadFile;

impl Capability for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Read lines from a file. Returns the content within the requested \
                line range and the total line count. Use start/end to window into large files. \
                Lines are 1-indexed and inclusive."
                .into(),
            parameters: ReadFileArgs::schema(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: ReadFileArgs =
            serde_json::from_value(args).map_err(|e| ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("[ERROR] {}", e),
            })?;

        let raw = fs::read_to_string(&args.path).map_err(|e| ToolError::ToolExecution {
            source: anyhow::anyhow!("[ERROR] {}", e),
        })?;

        let lines: Vec<&str> = raw.lines().collect();
        let total_lines = lines.len();

        let start = args.start.unwrap_or(1).max(1);
        let end = args.end.unwrap_or(total_lines).min(total_lines);

        if start > total_lines {
            let output = ReadFileOutput {
                content: String::new(),
                total_lines,
                range: [start, end],
            };
            return Ok(serde_json::to_string(&output).unwrap());
        }

        let content = lines[(start - 1)..end].join("\n");

        let output = ReadFileOutput {
            content,
            total_lines,
            range: [start, end],
        };

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

    #[test]
    fn reads_full_file() {
        let f = temp_file("line1\nline2\nline3\nline4\nline5");
        let tool = ReadFile;
        let result = tool
            .execute(json!({ "path": f.path().to_str().unwrap() }))
            .unwrap();
        let out: ReadFileOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(out.total_lines, 5);
        assert_eq!(out.range, [1, 5]);
        assert!(out.content.contains("line1"));
        assert!(out.content.contains("line5"));
    }

    #[test]
    fn reads_bounded_range() {
        let f = temp_file("a\nb\nc\nd\ne\nf");
        let tool = ReadFile;
        let result = tool
            .execute(json!({
                "path": f.path().to_str().unwrap(),
                "start": 2,
                "end": 4
            }))
            .unwrap();
        let out: ReadFileOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(out.total_lines, 6);
        assert_eq!(out.range, [2, 4]);
        assert_eq!(out.content, "b\nc\nd");
    }

    #[test]
    fn clamps_end_to_file_length() {
        let f = temp_file("x\ny\nz");
        let tool = ReadFile;
        let result = tool
            .execute(json!({
                "path": f.path().to_str().unwrap(),
                "start": 1,
                "end": 100
            }))
            .unwrap();
        let out: ReadFileOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(out.total_lines, 3);
        assert_eq!(out.range, [1, 3]);
    }

    #[test]
    fn start_beyond_file_returns_empty() {
        let f = temp_file("one\ntwo");
        let tool = ReadFile;
        let result = tool
            .execute(json!({
                "path": f.path().to_str().unwrap(),
                "start": 50
            }))
            .unwrap();
        let out: ReadFileOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(out.total_lines, 2);
        assert!(out.content.is_empty());
    }

    #[test]
    fn nonexistent_file_errors() {
        let tool = ReadFile;
        let result = tool.execute(json!({ "path": "/no/such/file.txt" }));
        assert!(result.is_err());
    }
}
