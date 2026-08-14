use super::message::Message;
use crate::google::error::GoogleError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug)]
pub struct LlmToolCall {
    pub name: String,
    pub args: Value,
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

impl TryFrom<GeminiResponse> for LlmToolCall {
    type Error = GoogleError;

    fn try_from(value: GeminiResponse) -> Result<Self, Self::Error> {
        let candidate =
            value
                .candidates
                .into_iter()
                .next()
                .ok_or_else(|| GoogleError::MalformedResponse {
                    source: anyhow::anyhow!("No candidates in response"),
                })?;

        for part in &candidate.content.parts {
            if let Some(fc) = &part.function_call {
                return Ok(LlmToolCall {
                    name: fc.name.clone(),
                    args: fc.args.clone(),
                });
            }
        }

        if candidate.finish_reason.as_deref() == Some("STOP") {
            let text = candidate
                .content
                .parts
                .into_iter()
                .find_map(|p| p.text)
                .filter(|s| !s.trim().is_empty())
                .ok_or(GoogleError::MalformedResponse {
                    source: anyhow::anyhow!(
                        "Model stopped without producing a conclusion. \
                        Expected non-empty text content alongside finish_reason=STOP."
                    ),
                })?;

            return Ok(LlmToolCall {
                name: "stop".into(),
                args: Value::String(text),
            });
        }

        Err(GoogleError::MalformedResponse {
            source: anyhow::anyhow!("Neither function call nor valid stop text was found"),
        })
    }
}

pub struct LlmStructuredOutput {
    pub value: Value,
}

impl TryFrom<GeminiResponse> for LlmStructuredOutput {
    type Error = GoogleError;

    fn try_from(res: GeminiResponse) -> Result<Self, Self::Error> {
        let candidate =
            res.candidates
                .into_iter()
                .next()
                .ok_or_else(|| GoogleError::MalformedResponse {
                    source: anyhow::anyhow!("No candidates in response"),
                })?;

        let text = candidate
            .content
            .parts
            .into_iter()
            .find_map(|p| p.text)
            .ok_or_else(|| GoogleError::MalformedResponse {
                source: anyhow::anyhow!(
                    "Expected text content field for structured output, got none"
                ),
            })?;

        let value: Value = serde_json::from_str(&text)
            .map_err(|e| GoogleError::MalformedResponse { source: e.into() })?;

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
    fn parses_tool_call() {
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

        let result = LlmToolCall::try_from(parse(raw)).unwrap();
        assert_eq!(result.name, "git_status");
        assert_eq!(result.args, json!({"path": "."}));
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

        let result = LlmToolCall::try_from(parse(raw)).unwrap();
        assert_eq!(result.name, "stop");
        assert_eq!(result.args, json!("I have everything I need."));
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

        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(GoogleError::MalformedResponse { .. })));
    }

    #[test]
    fn fails_when_no_candidates() {
        let raw = json!({ "candidates": [] });
        let result = LlmToolCall::try_from(parse(raw));
        assert!(matches!(result, Err(GoogleError::MalformedResponse { .. })));
    }
}
