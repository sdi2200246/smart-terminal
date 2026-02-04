use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::context::state::DirsState;

#[derive(Debug)]
pub enum LlmError {
    Http(reqwest::Error),
    BadStatus(reqwest::StatusCode, String),
    Json(reqwest::Error),
}

#[derive(Debug, Deserialize , Serialize)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

impl  Message {
    pub fn new(role:String , content:Option<String>)->Message{
        Message { role, content, reasoning:None, tool_calls:vec![]}
    }

}
#[derive(Deserialize , Debug)]
struct ChatResponse {
     pub choices: Vec<Choice>,
}
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<String>,
}

#[derive(Deserialize , Debug)]
struct Choice {
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub logprobs: Option<serde_json::Value>,
    pub message: Message,

}   
#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema)]
pub enum Tools{
    GitStatus,
    ProcessList,
    FinalAnswer(serde_json::Value),
}
#[derive(Serialize , Deserialize , PartialEq, Eq , JsonSchema)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunction,
}

impl Tool{
    pub fn factory(variant:Tools)->Tool{
        let r_type = "function".to_string();
        let var = &variant;
        match var{
            Tools::GitStatus =>{
                Tool { 
                    r#type:r_type,
                    function:ToolFunction::factory(&variant)
                }
            }
            Tools::ProcessList =>{
                Tool { 
                    r#type:r_type,
                    function:ToolFunction::factory(&variant)
                }
            }
            Tools::FinalAnswer(_)=>{
                Tool { 
                    r#type:r_type,
                    function:ToolFunction::factory(&variant)
                }
            }
        }
    }

}
#[derive(Serialize , Deserialize , PartialEq , Eq , JsonSchema , Debug )]
pub struct ToolFunction {
    pub name: String,
    #[serde(default)]
    description:Option<String>,
    parameters: serde_json::Value,
}


impl ToolFunction{

    pub fn factory(variant:&Tools) ->ToolFunction{
         let empty_params = serde_json::json!({
            "type": "object",
            "properties": {}
        });

        match variant{
            Tools::ProcessList => {
                ToolFunction{ 
                    name:"running_processes".into(),
                    description:Some("Returns running processes with names and pids".into()),
                    parameters:empty_params
                }
            }
            Tools::GitStatus => {
                ToolFunction{ 
                    name:"git_status".into(),
                    description:Some("Returns the current status of the current github repo".into()),
                    parameters:empty_params
                }
            }
            Tools::FinalAnswer(properties) => {
                ToolFunction {
                    name: "final_answer".into(),
                    description: Some(
                        "You MUST use this tool for your final answer. \
                        The arguments MUST match the required JSON schema exactly."
                            .into()
                    ),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": properties
                            .as_object()
                            .map(|o| o.keys().cloned().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                }
            }
        }
    }
}


#[derive(Debug, Deserialize , Serialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolFunction,
}

struct GroqClient{
    client:Client,
    api_key:String,
    model:String,
    completions_url:String,
}


impl GroqClient{

    pub async fn llm_request(&self , req:ChatRequest) -> Result<ChatResponse ,LlmError>{
        let body = req;
        let res = self.client
            .post(&self.completions_url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(LlmError::Http)?;


        let status = res.status();

        if !status.is_success(){
            let body = res.text().await.unwrap_or_default();
            return Err(LlmError::BadStatus(status, body));
        }

        let json: ChatResponse = res
            .json::<ChatResponse>()
            .await
            .map_err(|e| LlmError::Json(e))?;

        Ok(json)
      
    }  



}
#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use reqwest::Client;


    fn test_client(server: &MockServer) -> GroqClient {
        GroqClient {
            client: Client::new(),
            api_key: "test-key".into(),
            model: "openai/gpt-oss-120b".into(),
            completions_url: server.url("/openai/v1/chat/completions"),
        }
    }

    fn valid_chat_request() -> ChatRequest {
        ChatRequest {
            model: "openai/gpt-oss-120b".into(),
            messages: vec![Message {
                role: "user".into(),
                content: Some("Predict next command".into()),
                reasoning: None,
                tool_calls: vec![],
            }],
            tools: None,
            tool_choice: None,
        }
    }

    #[tokio::test]
    async fn parses_tool_call_response() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(POST)
                .path("/openai/v1/chat/completions");

            then.status(200)
                .json_body(serde_json::json!({
                    "choices": [{
                        "index": 0,
                        "finish_reason": "tool_calls",
                        "message": {
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "History",
                                    "parameters": "{\"message\":\"\"}",
                                }
                            }]
                        }
                    }]
                }));
        });

        let client = test_client(&server);
        let req = valid_chat_request();

        let res = client.llm_request(req).await.unwrap();

        let msg = &res.choices[0].message;

        assert!(msg.content.is_none());
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].function.name, "History");
    }

    #[tokio::test]
    async fn parses_final_answer_response() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(POST)
                .path("/openai/v1/chat/completions");

            then.status(200)
                .json_body(serde_json::json!({
                    "choices": [{
                        "index": 0,
                        "finish_reason": "stop",
                        "message": {
                            "role": "assistant",
                            "content": "ls && cd project && git status"
                        }
                    }]
                }));
        });

        let client = test_client(&server);
        let req = valid_chat_request();

        let res = client.llm_request(req).await.unwrap();

        let msg = &res.choices[0].message;

        assert!(msg.tool_calls.is_empty());
        assert_eq!(
            msg.content.as_deref(),
            Some("ls && cd project && git status")
        );
    }
}


#[cfg(test)]
mod tool_function_factory_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn creates_git_status_tool_function() {
        let func = ToolFunction::factory(&Tools::GitStatus);

        assert_eq!(func.name, "GitStatus");
        assert_eq!(
            func.description.as_deref(),
            Some("Returns the current status of the current github repo")
        );

        assert_eq!(
            func.parameters,
            json!({
                "type": "object",
                "properties": {}
            })
        );
    }

    #[test]
    fn creates_process_list_tool_function() {
        let func = ToolFunction::factory(&Tools::ProcessList);

        assert_eq!(func.name, "RunningProcesses");
        assert_eq!(
            func.description.as_deref(),
            Some("Returns running processes with names and pids")
        );

        assert_eq!(
            func.parameters,
            json!({
                "type": "object",
                "properties": {}
            })
        );
    }
}

#[cfg(test)]
mod tool_factory_tests {
    use super::*;

    #[test]
    fn creates_git_status_tool() {
        let tool = Tool::factory(Tools::GitStatus);

        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "GitStatus");
    }

    #[test]
    fn creates_process_list_tool() {
        let tool = Tool::factory(Tools::ProcessList);

        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "RunningProcesses");
    }


    #[test]
    fn tool_serializes_to_openai_shape() {
        let tool = Tool::factory(Tools::GitStatus);

        let json = serde_json::to_value(&tool).unwrap();

        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "GitStatus");
        assert_eq!(
            json["function"]["parameters"],
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        );
    }


}
