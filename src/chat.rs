use serde::{Deserialize, Serialize};

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
    .expect("Missing key");

    let client = reqwest::Client::new();

    let body = ChatRequest {
        model: "llama-3.1-8b-instant".to_string(),
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

// pub fn filetered_answer(prompt: String){
//     println!("🧩 Filtered Answer mode activated! {}", prompt);
//     //code to extract later only the usefull info from the model answer.
// }





