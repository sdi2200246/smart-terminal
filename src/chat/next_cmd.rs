use serde::{Deserialize, Serialize};
use crate::context::traits::{LLMforamt};
use schemars::JsonSchema;

#[derive(Deserialize , Debug , JsonSchema)]
pub struct NextCmd{
    cmd:String,
    coment:String,
}

impl NextCmd{

    pub fn new(cmd:String , coment:String)->NextCmd{
        NextCmd{
            cmd,
            coment
        }
    }
}
impl LLMforamt for NextCmd {
    fn to_json_format() -> String {
        
        let schema = schemars::schema_for!(NextCmd);
        let schema_json = serde_json::to_string_pretty(&schema.schema).unwrap();
        format!(
            indoc::indoc! {r#"
                Return a JSON object with EXACTLY the following structure:
                
                {}

                Rules:
                - Output ONLY valid JSON.
                - Do NOT include comments, markdown, or explanations.
                - Do NOT add extra fields.
                - Do NOT wrap the JSON in code blocks.
            "#},
            schema_json
        )
    }
}





