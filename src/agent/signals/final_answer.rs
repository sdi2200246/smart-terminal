use super::{AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome};
use crate::core::capability::{ToolMetaData, ToolSafety};
use crate::tools::error::ToolError;
use serde_json::Value;

pub struct FinalAnswerSignal {
    pub properties: Value,
}

impl AgentSignal for FinalAnswerSignal {
    fn name(&self) -> &'static str {
        "final_answer"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Submits the final, structured JSON response to the user. Use this tool when you have successfully gathered all required information and are ready to end the loop.".into(),
            parameters: self.properties.clone(),
            safety: ToolSafety::Safe,
        }
    }

    fn handle(
        &self,
        args: Value,
        _ctx: &mut AgentSignalContext<'_>,
    ) -> Result<AgentSignalOutcome, ToolError> {
        let validator = jsonschema::validator_for(&self.properties).map_err(|e| {
            ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("Invalid contract schema: {}", e),
            }
        })?;

        if validator.is_valid(&args) {
            let result = args.to_string();
            Ok(AgentSignalOutcome {
                result,
                control: AgentLoopControl::Finish(args),
            })
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
    use crate::agent::agents::AgentBuilder;
    use crate::core::model::{Model, ModelName};
    use crate::core::session::AgentSession;
    use crate::utils::FlatSchema;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    struct Script {
        pub _script: String,
    }
    impl FlatSchema for Script {}

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    struct NextCommand {
        pub _cmd: String,
        pub _man: String,
    }
    impl FlatSchema for NextCommand {}

    fn final_answer_signal(schema: Value) -> FinalAnswerSignal {
        FinalAnswerSignal { properties: schema }
    }

    fn run_signal(
        signal: &FinalAnswerSignal,
        args: Value,
    ) -> Result<AgentSignalOutcome, ToolError> {
        let mut session = AgentSession::new(5);
        let mut agent =
            AgentBuilder::base("test", Model::with_default_temp(ModelName::GptOss120B)).build();
        let mut ctx = AgentSignalContext {
            session: &mut session,
            agent: &mut agent,
        };
        signal.handle(args, &mut ctx)
    }

    #[test]
    fn accepts_valid_args() {
        let signal = final_answer_signal(Script::schema());
        let args = json!({ "_script": "#!/bin/bash\necho hello" });
        let result = run_signal(&signal, args.clone());
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert_eq!(outcome.result, args.to_string());
        assert_eq!(outcome.control, AgentLoopControl::Finish(args));
    }

    #[test]
    fn rejects_missing_required_field() {
        let signal = final_answer_signal(NextCommand::schema());
        let args = json!({ "_cmd": "ls -la" });
        let result = run_signal(&signal, args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn rejects_wrong_type() {
        let signal = final_answer_signal(Script::schema());
        let args = json!({ "_script": 42 });
        let result = run_signal(&signal, args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn rejects_unknown_field() {
        let signal = final_answer_signal(Script::schema());
        let args = json!({ "_script": "echo hi", "extra": "nope" });
        let result = run_signal(&signal, args);
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
    }

    #[test]
    fn error_message_includes_validation_detail() {
        let signal = final_answer_signal(NextCommand::schema());
        let args = json!({ "cmd": "ls" });
        let err = run_signal(&signal, args).unwrap_err();
        let source = std::error::Error::source(&err)
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert!(
            source.contains("man") || source.contains("required"),
            "expected validation detail in error source, got: {source}"
        );
    }

    #[test]
    fn metadata_exposes_schema() {
        let schema = Script::schema();
        let signal = final_answer_signal(schema.clone());
        let meta = signal.metadata();
        assert_eq!(meta.name, "final_answer");
        assert_eq!(meta.parameters, schema);
    }
}

