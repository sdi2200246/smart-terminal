use serde_json::Value;
use crate::core::capability::{Capability, ToolMetaData};
use super::error::ToolError;

pub struct Json {
    pub properties: Value,
}

impl Capability for Json {
    fn name(&self) -> &'static str {
        "final_answer"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Submits the final, structured JSON response to the user. Use this tool when you have successfully gathered all required information and are ready to end the loop.".into(),
            parameters: self.properties.clone(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        let validator = jsonschema::validator_for(&self.properties)
            .map_err(|e| ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("Invalid contract schema: {}", e),
            })?;

        if validator.is_valid(&args) {
            Ok(args.to_string())
        } else {
            let errors: Vec<String> = validator
                .iter_errors(&args)
                .map(|e| e.to_string())
                .collect();
            Err(ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("Contract violation: {}", errors.join(", ")),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use crate::utils::FlatSchema;

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    struct Script {
        pub script: String,
    }
    impl FlatSchema for Script {}

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    struct NextCommand {
        pub cmd: String,
        pub man: String,
    }
    impl FlatSchema for NextCommand {}

    fn json_tool(schema: Value) -> Json {
        Json { properties: schema }
    }

    #[test]
    fn accepts_valid_args() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": "#!/bin/bash\necho hello" });
        let result = tool.execute(args.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), args.to_string());
    }

    #[test]
    fn rejects_missing_required_field() {
        let tool = json_tool(NextCommand::schema());
        let args = json!({ "cmd": "ls -la" });
        let result = tool.execute(args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn rejects_wrong_type() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": 42 });
        let result = tool.execute(args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn rejects_unknown_field() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": "echo hi", "extra": "nope" });
        let result = tool.execute(args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn error_message_includes_validation_detail() {
        let tool = json_tool(NextCommand::schema());
        let args = json!({ "cmd": "ls" });
        let err = tool.execute(args).unwrap_err();
        let msg = err.to_string();
        // The wrapped anyhow error should mention what went wrong.
        let source = std::error::Error::source(&err).map(|s| s.to_string()).unwrap_or_default();
        assert!(
            source.contains("man") || source.contains("required"),
            "expected validation detail in error source, got: {source}"
        );
    }

    #[test]
    fn metadata_exposes_schema() {
        let schema = Script::schema();
        let tool = json_tool(schema.clone());
        let meta = tool.metadata();
        assert_eq!(meta.name, "final_answer");
        assert_eq!(meta.parameters, schema);
    }
}