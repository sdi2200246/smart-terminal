use super::error::ToolError;
use crate::core::{Capability, ToolMetaData};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::io::{self, Write};

#[derive(JsonSchema, Deserialize, Debug)]
pub struct ModelQuestion {
    /// This is your question where you speak with the user to achieve alignment.
    pub question: String,
}
define_tool! {
    pub struct AskUser;
    /// Use this tool to achieve alignment with the user when their intent,
    /// preferences, or context are unclear. Ask one focused question at a time.
    /// Do not use this tool for information you can infer yourself.
    fn ask_user(args: ModelQuestion) -> Result<String, ToolError> {
        print!(
            "\n🤖 {}\n\n(type your reply and press Enter)\n> ",
            args.question
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore]
    fn ask_user_manual() {
        let tool = AskUser;
        let res = tool.execute(json!({"question": "Are you ready?"}));
        assert!(res.is_ok());
    }
}
