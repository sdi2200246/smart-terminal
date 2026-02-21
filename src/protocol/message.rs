use serde::{Deserialize, Serialize};
use super::tool::{ToolCall};

#[derive(Serialize, Deserialize, Debug, Clone , PartialEq)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_calls:Vec<ToolCall>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

     #[serde(default)]
     #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

}   

impl  Message {

    pub fn user(content:Option<String>)->Message{
        Message {
            role:"user".into(),
            content,
            tool_calls:vec![],
            tool_call_id:None,
            name:None
        }

    }
    pub fn system(content:Option<String>)->Message{
        Message {
            role:"system".into(),
            content,
            tool_calls:vec![],
            tool_call_id:None,
            name:None
        }

    }
    pub fn context<T:Serialize>(ctx:&T)->Message{
        let json = serde_json::to_string_pretty(ctx).unwrap();
        let content = format!("Context:\n{}", json);

        Message {
            role:"system".into(),
            content:Some(content),
            tool_calls:vec![],
            tool_call_id:None,
            name:None
        }
    }
    pub fn tool_responce(content: Option<String> , tool_call_id:String , tool_name:String)->Message{
        Message { 
            role:"tool".into(),
            content:content,
            tool_calls:vec![],
            tool_call_id:Some(tool_call_id),
            name: Some(tool_name)
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

        println!("ROLE: {}", msg.role);
        println!("CONTENT:\n{}", msg.content.clone().unwrap());
    }
}
