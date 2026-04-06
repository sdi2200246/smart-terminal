use serde_json::Value;
use crate::core::capability::{Capability, ToolFunction};
use super::error::ToolError;

pub struct Json {
    pub properties: Value,
}

impl Capability for Json {
    fn name(&self) -> &'static str {
        "final_answe"
    }

    fn metadata(&self) -> ToolFunction {
        ToolFunction {
            name: self.name().into(),
            description: "Use this tool to submit your final answer. You MUST call this tool to complete your task — it is the only valid way to produce output.".into(),
            parameters: self.properties.clone(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
         if self.properties.is_null() {
            return Ok(args.to_string());
        }

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
    struct Script {
        pub script: String,
    }
    impl FlatSchema for Script {}

    #[derive(JsonSchema, Deserialize)]
    struct NextCommand {
        pub cmd: String,
        pub man: String,
    }
    impl FlatSchema for NextCommand {}

    fn json_tool(schema: Value) -> Json {
        Json { properties: schema }
    }

    #[test]
    fn accepts_valid_contract() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": "#!/bin/bash\necho hello" });
        let result = tool.execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_missing_required_field() {
        let tool = json_tool(Script::schema());
        let args = json!({});
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wrong_type() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": 42 });
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn error_message_contains_field_name() {
        let tool = json_tool(NextCommand::schema());
        let args = json!({ "cmd": "ls" });
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_multi_field_contract() {
        let tool = json_tool(NextCommand::schema());
        let args = json!({ "cmd": "ls -la", "man": "list directory contents" });
        let result = tool.execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn ok_returns_serialized_args() {
        let tool = json_tool(Script::schema());
        let args = json!({ "script": "echo test" });
        let result = tool.execute(args.clone()).unwrap();
        let round_trip: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(round_trip, args);
    }

    #[test]
    fn null_schema_accepts_anything() {
        let tool = json_tool(Value::Null);
        let args = json!({ "anything": "goes" });
        let result = tool.execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn metadata_name_is_final_answer() {
        let tool = json_tool(Script::schema());
        assert_eq!(tool.name(), "json");
        assert_eq!(tool.metadata().name, "json");
    }

    #[test]
    fn metadata_parameters_match_schema() {
        let schema = Script::schema();
        let tool = json_tool(schema.clone());
        assert_eq!(tool.metadata().parameters, schema);
    }
}