use serde::{Deserialize, Serialize};
use chrono::{DateTime,Utc};
use crate::context::traits::{LLMforamt};
use schemars::JsonSchema;


#[derive(Serialize, Deserialize, Debug , JsonSchema)]
pub enum LogType{
  FatalErr,
  LogicalErr,
  RuntimeErr,
  OptProposition,
  None,
}

#[derive(Serialize, Deserialize, Debug , JsonSchema)]
pub struct SmartLog{
  message:String,
  kind:LogType,
  line:i64,
  #[schemars(skip)]
  #[serde(default = "default_timestamp")]
  timestamp: chrono::DateTime<Utc>,
}

pub fn default_timestamp() -> DateTime<Utc> {
  Utc::now()
}

impl SmartLog{
    pub fn new(message:String , kind:LogType , line:i64)->SmartLog{ 
          return SmartLog{
            message,
            kind,
            line,
            timestamp:Utc::now(),
          }
    }
    pub fn print(&self){
        println!("[{:?}][Kind:{:?}][Message:{}][Line:{}]" , self.timestamp , self.kind , self.message , self.line);
    }
}
impl LLMforamt for SmartLog {
    fn to_json_format() -> String {
        let schema = schemars::schema_for!(SmartLog);
        let schema_json = serde_json::to_string_pretty(&schema.schema).unwrap();

        format!(
            indoc::indoc! {r#"
                Return a JSON object with EXACTLY the following structure:

                {}

                IMPORTANT RULES:
                - Output ONLY valid JSON.
                - Do NOT include comments inside the JSON.
                - Do NOT add extra fields.
                - Do NOT wrap the output in code fences.
                - The field "timestamp" MUST NOT be included in the output.
            "#},
            schema_json
        )
    }
}
