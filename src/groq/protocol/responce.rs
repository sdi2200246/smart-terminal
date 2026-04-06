use serde::Deserialize;
use serde_json::Value;
use super::message::Message;
use crate::groq::error::GroqError;

#[derive(Debug)]
pub struct LlmToolCall {
    pub name: String,
    pub id: String,
    pub args: Value,
}

#[derive(Deserialize, Debug)]
pub struct GroqResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
pub struct Choice {
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub logprobs: Option<serde_json::Value>,
    pub message: Message,
}

impl TryFrom<GroqResponse> for LlmToolCall {
    type Error = GroqError;

    fn try_from(value: GroqResponse) -> Result<Self, Self::Error> {
        let choice = value
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| GroqError::MalformedResponse {
                source: anyhow::anyhow!("No choices in response"),
            })?;

        let tool = choice
            .message
            .tool_calls
            .into_iter()
            .next()
            .ok_or(GroqError::InvalidToolCall {
                source: anyhow::anyhow!("No tools where found"),
            })?;

        let args_str = tool.function.arguments.ok_or(GroqError::InvalidToolCall {
            source: anyhow::anyhow!("No arguments where found"),
        })?;

        let parsed_args: Value = serde_json::from_str(&args_str)
            .map_err(|e| GroqError::MalformedResponse { source: e.into() })?;

        Ok(LlmToolCall {
            name: tool.function.name,
            id: tool.id,
            args: parsed_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(json_value: serde_json::Value) -> GroqResponse {
        serde_json::from_value(json_value).unwrap()
    }

    #[test]
    fn parses_final_answer() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "final_answer",
                            "arguments": "{\"result\":\"done\"}"
                        }
                    }]
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw)).unwrap();
        assert_eq!(result.name, "final_answer");
        assert_eq!(result.args, json!({"result": "done"}));
    }

    #[test]
    fn parses_regular_tool_call() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_2",
                        "type": "function",
                        "function": {
                            "name": "git_status",
                            "arguments": "{\"path\":\".\"}"
                        }
                    }]
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw)).unwrap();
        assert_eq!(result.name, "git_status");
        assert_eq!(result.id, "call_2");
        assert_eq!(result.args, json!({"path": "."}));
    }

    #[test]
    fn fails_when_no_choices() {
        let raw = json!({ "choices": [] });
        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(GroqError::MalformedResponse { .. })));
    }

    #[test]
    fn fails_when_tool_calls_missing() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(GroqError::InvalidToolCall { .. })));
    }

    #[test]
    fn fails_when_tool_calls_empty() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": []
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(GroqError::InvalidToolCall { .. })));
    }
}