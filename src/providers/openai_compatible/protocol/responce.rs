use super::super::error::OpenAiError;
use super::message::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct LlmToolCall {
    pub name: String,
    pub id: String,
    pub args: Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAiResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Choice {
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub logprobs: Option<serde_json::Value>,
    pub message: Message,
}

impl TryFrom<OpenAiResponse> for LlmToolCall {
    type Error = OpenAiError;

    fn try_from(value: OpenAiResponse) -> Result<Self, Self::Error> {
        let choice = value
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| OpenAiError::UnexpectedOutput {
                body: "No choices in response".to_string(),
            })?;

        if choice.finish_reason == Some("stop".to_string()) {
            let conclusion = choice
                .message
                .content
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| OpenAiError::UnexpectedOutput {
                    body: "Model stopped without producing a conclusion. Expected non-empty content alongside finish_reason=stop.".to_string(),
                })?;

            return Ok(LlmToolCall {
                name: "stop".into(),
                id: "".into(),
                args: Value::String(conclusion),
            });
        }

        let tool = choice
            .message
            .tool_calls
            .into_iter()
            .next()
            .ok_or_else(|| OpenAiError::UnexpectedOutput {
                body: "Expected a tool call but none were found.".to_string(),
            })?;

        let args_str = tool
            .function
            .arguments
            .ok_or_else(|| OpenAiError::UnexpectedOutput {
                body: "No arguments were found in the tool call.".to_string(),
            })?;

        let parsed_args: Value =
            serde_json::from_str(&args_str).map_err(|e| OpenAiError::InvalidToolCall {
                body: e.to_string(),
            })?;

        Ok(LlmToolCall {
            name: tool.function.name,
            id: tool.id,
            args: parsed_args,
        })
    }
}

pub struct LlmStructuredOutput {
    pub value: Value,
}

impl TryFrom<OpenAiResponse> for LlmStructuredOutput {
    type Error = OpenAiError;

    fn try_from(res: OpenAiResponse) -> Result<Self, Self::Error> {
        let choice = res
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| OpenAiError::UnexpectedOutput {
                body: "No choices in response".to_string(),
            })?;

        let content = choice
            .message
            .content
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| OpenAiError::UnexpectedOutput {
                body: "Model response content is empty in structured mode".to_string(),
            })?;

        let value: Value =
            serde_json::from_str(&content).map_err(|e| OpenAiError::UnexpectedOutput {
                body: format!("Failed to parse structured output as JSON: {e}"),
            })?;

        Ok(LlmStructuredOutput { value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(json_value: serde_json::Value) -> OpenAiResponse {
        serde_json::from_value(json_value).unwrap()
    }

    #[test]
    fn parses_tool_call() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
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
        assert_eq!(result.id, "call_1");
        assert_eq!(result.args, json!({"path": "."}));
    }

    #[test]
    fn stop_with_content_maps_to_stop_sentinel() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": "I have everything I need."
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw)).unwrap();
        assert_eq!(result.name, "stop");
        assert_eq!(result.args, json!("I have everything I need."));
    }

    #[test]
    fn stop_without_content_is_unexpected_output() {
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
        assert!(matches!(result, Err(OpenAiError::UnexpectedOutput { .. })));
    }

    #[test]
    fn stop_with_empty_content_is_unexpected_output() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": "   "
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(OpenAiError::UnexpectedOutput { .. })));
    }

    #[test]
    fn fails_when_no_choices() {
        let raw = json!({ "choices": [] });
        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(OpenAiError::UnexpectedOutput { .. })));
    }

    #[test]
    fn fails_when_tool_calls_missing() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(OpenAiError::UnexpectedOutput { .. })));
    }

    #[test]
    fn fails_when_tool_calls_empty() {
        let raw = json!({
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "logprobs": null,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": []
                }
            }]
        });

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(OpenAiError::UnexpectedOutput { .. })));
    }
}
