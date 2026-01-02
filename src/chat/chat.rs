use serde::{Deserialize, Serialize};
use super::{NextCmd, Promt};
use crate::context::state::DirsState;
use crate::context::traits::{LLMforamt};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize , Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize , Debug)]
struct Choice {
    message: AssistantMessage,
}   

#[derive(Deserialize , Debug)]
struct AssistantMessage {
    content: String,
}

pub async fn chat(prompt: String) -> Result<String, Box<dyn std::error::Error>> {

    dotenv::dotenv().ok();
    let api_key = std::env::var("GROQ_API_KEY")
    .expect("GROQ_API_KEY");

    let client = reqwest::Client::new();

    let body = ChatRequest {
        model: "openai/gpt-oss-20b".to_string(),
        messages: vec![
            Message {
                role: "user".into(),
                content: prompt.into(),
            }
        ]   ,
    };
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;

    let answer = &res.choices[0].message.content;
    Ok(answer.to_string())
}


pub async fn predict_next_cmd(dir_state:DirsState)-> Result<NextCmd, Box<dyn std::error::Error>>{
    let p = Promt::new(
                "You are a next terminal commnad predictor".to_string(),
                "Analyze the following inofrmation and follow the format to predict the next terminal commnad of the user.".to_string(),
                dir_state);
        
    let response_str = chat(p.to_smartlog_prompt(NextCmd::to_json_format())).await?;
    let prediction: NextCmd = serde_json::from_str(&response_str)?;
    Ok(prediction)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_content() {
        let resp = ChatResponse {
            choices: vec![Choice { message: AssistantMessage { content: "hello".into() } }],
        };
        assert_eq!(resp.choices[0].message.content, "hello");
    }
     fn fake_state() -> DirsState {
        use std::path::PathBuf;
        let cwd = PathBuf::from("/home/jason/Github_Repos/smart-terminal");

        let files = vec![
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "src/main.rs".to_string(),
            "src/chat/mod.rs".to_string(),
            "src/context/state.rs".to_string(),
            "README.md".to_string(),
        ];

        let cmd_history = vec![
            "cd src".to_string(),
            "ls".to_string(),
            "git status".to_string(),
            "cargo test".to_string(),
            "cargo run".to_string(),
            "git branch".to_string(),
            "docker -help".to_string(),
            "docker top".to_string(),
        ];

        DirsState::new(cwd, files, cmd_history , "docker -f".to_string())

    }
    #[tokio::test]
    async fn test_to_smartlog_prompt2(){
        let state = fake_state();
        let next = predict_next_cmd(state).await.unwrap();
        println!("{:?}" , next);

    }

}



