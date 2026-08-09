use super::tool::{FunctionCall, FunctionResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<Part>,
}

impl Message {
    pub fn user(content: Option<String>) -> Message {
        Message {
            role: Some("user".into()),
            parts: vec![Part {
                text: content,
                function_call: None,
                function_response: None,
            }],
        }
    }

    pub fn system(content: Option<String>) -> Message {
        Message {
            role: None,
            parts: vec![Part {
                text: content,
                function_call: None,
                function_response: None,
            }],
        }
    }

    pub fn context<T: Serialize>(ctx: &T) -> Message {
        let json = serde_json::to_string_pretty(ctx).unwrap();
        let content = format!("Context:\n{}", json);

        Message::system(Some(content))
    }

   pub fn tool_responce(content: Option<String>, tool_name: String) -> Message {
        Message {
            role: Some("user".into()),
            parts: vec![Part {
                text: None,
                function_call: None,
                function_response: Some(FunctionResponse {
                    name: tool_name,
                    response: content.map(|c| serde_json::json!({ "content": c })),
                }),
            }],
        }
    }

    pub fn tool_call(name: String, args: Value) -> Message {
        Message {
            role: Some("model".into()),
            parts: vec![Part {
                text: None,
                function_call: Some(FunctionCall { name, args }),
                function_response: None,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestContext {
        cwd: String,
        history_len: usize,
    }

    #[test]
    fn context_creates_expected_message() {
        let ctx = TestContext {
            cwd: "/tmp".into(),
            history_len: 42,
        };

        let msg = Message::context(&ctx);

        println!("CONTENT:\n{:?}", msg.parts[0].text);
    }
}