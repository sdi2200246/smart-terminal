use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Value::is_null")]
    pub parameters: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    pub name: String,
    pub response: Option<Value>,
}
