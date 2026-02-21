use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::Value;

#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema , Debug, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunction,
}

impl Tool{
    pub fn factory(function:ToolFunction)->Tool{
        Tool { 
            r#type:"function".to_string(),
            function
        }
    }
}

#[derive(Serialize , Deserialize , PartialEq , Eq , JsonSchema , Debug , Clone )]
pub struct ToolFunction {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description:Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl  ToolFunction {
    pub fn arguments(&self)->Value{
        if let Some(v) = &self.arguments{
            return v.clone();
        }
        else {
            return serde_json::Value::Null;
        }
    }
}

#[derive(Debug, Deserialize , Serialize , Clone , PartialEq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolFunction,
    #[serde(default)]
    pub name:String,
}

