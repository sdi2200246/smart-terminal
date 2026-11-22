use super::error::ToolError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;

#[cfg(test)]
use crate::core::Capability;

#[derive(JsonSchema, Deserialize, Debug)]
pub struct ReadFileArgs {
    /// Path to the file to read, relative to cwd.
    pub path: String,
    /// First line to include (1-indexed). Defaults to 1.
    pub start: Option<usize>,
    /// Last line to include (1-indexed, inclusive). Defaults to end of file.
    pub end: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReadFileOutput {
    pub content: String,
    pub total_lines: usize,
    pub range: [usize; 2],
}

define_tool! {
    pub struct ReadFile;
    /// Read lines from a file. Returns the content within the requested
    /// line range, total line count, and selected range as JSON. Use start/end
    /// to window into large files.
    /// Lines are 1-indexed and inclusive.
    fn read_file(args: ReadFileArgs) -> Result<String, ToolError> {
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
            return super::json_output(&output);
        }

        let content = lines[(start - 1)..end].join("\n");

        let output = ReadFileOutput {
            content,
            total_lines,
            range: [start, end],
        };

        super::json_output(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn parse(result: &str) -> ReadFileOutput {
        serde_json::from_str(result).unwrap()
    }

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
        let output = parse(&result);
        assert_eq!(output.total_lines, 5);
        assert_eq!(output.range, [1, 5]);
        assert!(output.content.contains("line1"));
        assert!(output.content.contains("line5"));
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
        let output = parse(&result);
        assert_eq!(output.total_lines, 6);
        assert_eq!(output.range, [2, 4]);
        assert_eq!(output.content, "b\nc\nd");
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
        let output = parse(&result);
        assert_eq!(output.total_lines, 3);
        assert_eq!(output.range, [1, 3]);
        assert_eq!(output.content, "x\ny\nz");
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
        let output = parse(&result);
        assert_eq!(output.total_lines, 2);
        assert_eq!(output.range, [50, 2]);
        assert!(output.content.is_empty());
    }

    #[test]
    fn nonexistent_file_errors() {
        let tool = ReadFile;
        let result = tool.execute(json!({ "path": "/nonexistent/file/path" }));
        assert!(result.is_err());
    }
}
