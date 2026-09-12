use super::message::Message;
use crate::providers::google::error::GoogleError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug)]
pub struct LlmToolCall {
    pub name: String,
    pub args: Value,
    pub thinking_state: String,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub calls: Vec<LlmToolCall>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    #[serde(default)]
    pub finish_reason: Option<String>,
    pub content: Message,
}

impl TryFrom<GeminiResponse> for LlmResponse {
    type Error = GoogleError;

    fn try_from(value: GeminiResponse) -> Result<Self, Self::Error> {
        let candidate = value
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| GoogleError::UnexpectedOutput {
                body: "No candidates in response".to_string(),
            })?;

        let calls: Vec<LlmToolCall> = candidate
            .content
            .parts
            .iter()
            .filter_map(|part| {
                part.function_call.as_ref().map(|fc| LlmToolCall {
                    name: fc.name.clone(),
                    args: fc.args.clone(),
                    thinking_state: part.thought_signature.clone().unwrap_or("skip_thought_signature_validator".into()),
                })
            })
            .collect();

        if !calls.is_empty() {
            return Ok(LlmResponse { calls });
        }

        if candidate.finish_reason.as_deref() == Some("STOP") {
            let text = candidate
                .content
                .parts
                .into_iter()
                .find_map(|p| p.text)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| GoogleError::UnexpectedOutput {
                    body: "Model stopped without producing a conclusion. Expected non-empty text content alongside finish_reason=STOP.".to_string(),
                })?;

            return Ok(LlmResponse {
                calls: vec![LlmToolCall {
                    name: "stop".into(),
                    args: Value::String(text),
                    thinking_state: "".to_string(),
                }],
            });
        }

        Err(GoogleError::UnexpectedOutput {
            body: "Neither function call nor valid stop text was found".to_string(),
        })
    }
}

pub struct LlmStructuredOutput {
    pub value: Value,
}

impl TryFrom<GeminiResponse> for LlmStructuredOutput {
    type Error = GoogleError;

    fn try_from(res: GeminiResponse) -> Result<Self, Self::Error> {
        let candidate = res
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| GoogleError::UnexpectedOutput {
                body: "No candidates in response".to_string(),
            })?;

        let text = candidate
            .content
            .parts
            .into_iter()
            .find_map(|p| p.text)
            .ok_or_else(|| GoogleError::UnexpectedOutput {
                body: "Expected text content field for structured output, got none".to_string(),
            })?;

        let value: Value =
            serde_json::from_str(&text).map_err(|e| GoogleError::UnexpectedOutput {
                body: e.to_string(),
            })?;

        Ok(LlmStructuredOutput { value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(json_value: serde_json::Value) -> GeminiResponse {
        serde_json::from_value(json_value).unwrap()
    }

    #[test]
    fn parses_single_tool_call() {
        let raw = json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": {
                            "name": "git_status",
                            "args": { "path": "." }
                        }
                    }]
                }
            }]
        });

        let result = LlmResponse::try_from(parse(raw)).unwrap();
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "git_status");
        assert_eq!(result.calls[0].args, json!({"path": "."}));
    }

    #[test]
    fn parses_multiple_parallel_tool_calls() {
        let raw = json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "functionCall": { "name": "read_file", "args": { "path": "a.rs" } },
                            "thoughtSignature": "sig-a"
                        },
                        {
                            "functionCall": { "name": "read_file", "args": { "path": "b.rs" } },
                            "thoughtSignature": "sig-b"
                        }
                    ]
                }
            }]
        });

        let result = LlmResponse::try_from(parse(raw)).unwrap();
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "read_file");
        assert_eq!(result.calls[0].thinking_state, "sig-a");
        assert_eq!(result.calls[1].args, json!({"path": "b.rs"}));
        assert_eq!(result.calls[1].thinking_state, "sig-b");
    }

    #[test]
    fn stop_with_content_maps_to_stop_sentinel() {
        let raw = json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "I have everything I need."
                    }]
                }
            }]
        });

        let result = LlmResponse::try_from(parse(raw)).unwrap();
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "stop");
        assert_eq!(result.calls[0].args, json!("I have everything I need."));
    }

    #[test]
    fn stop_without_content_is_malformed() {
        let raw = json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": []
                }
            }]
        });

        let result = LlmResponse::try_from(parse(raw));
        assert!(matches!(result, Err(GoogleError::UnexpectedOutput { .. })));
    }

    #[test]
    fn fails_when_no_candidates() {
        let raw = json!({ "candidates": [] });
        let result = LlmResponse::try_from(parse(raw));
        assert!(matches!(result, Err(GoogleError::UnexpectedOutput { .. })));
    }
}