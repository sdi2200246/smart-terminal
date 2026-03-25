use serde_json::Value;
use serde::Deserialize;
use schemars::{JsonSchema};
use crate::utils::FlatSchema;
use crate::interfaces::capability::{Capability, ToolFunction};
use super::error::ToolError;
use std::io::{self, Write};



#[derive(JsonSchema , Deserialize , Debug)]
pub struct ModelQuestion{
    /// This is your question where you speak with the user to achieve alignment.
    pub question:String
}
impl FlatSchema for ModelQuestion {}

pub struct AskUser;

impl  Capability  for AskUser{

    fn name(&self) -> &'static str {
        "ask_user"
    }
    fn metadata(&self) -> ToolFunction {
        ToolFunction {
            name: self.name().into(),
            description:"Use this tool to achieve alignment with the user when their intent, \
              preferences, or context are unclear. Ask one focused question at a time. \
              Do not use this tool for information you can infer yourself.".into(),
            parameters: ModelQuestion::schema(),
        }
    }
    fn execute(&self, args: Value) -> Result<String, ToolError> {
        ask_user(args)
    }
}

fn ask_user(args:Value)->Result<String ,ToolError>{

    let llm_question:ModelQuestion = serde_json::from_value(args)
        .map_err(|e| ToolError::ToolExecution { source: (e.into())})?;

    print!("\n🤖 {}\n\n(type your reply and press Enter)\n> ", llm_question.question);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|e| ToolError::ToolExecution { source: (e.into())})?;
    
    return  Ok(input);
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore]
    fn test_ask_user_manual() {
        let args = json!({ "question": "What branch are you working on?" });
        let result = ask_user(args);
        println!("User responded: {:?}", result);
        assert!(result.is_ok());
    }
}