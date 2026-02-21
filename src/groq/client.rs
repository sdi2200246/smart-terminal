use reqwest::Client;
use super::protocol::responce::ChatResponse;
use super::protocol::request::ChatRequest;

#[derive(Debug)]
pub enum LlmError {
    Http(reqwest::Error),
    BadStatus(reqwest::StatusCode, String),
    Json(reqwest::Error),
}

pub struct GroqClient{
    client:Client,
    api_key:String,
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
impl Default for  GroqClient{
    fn default()->GroqClient{
        GroqClient{
            client:Client::new(),
            api_key:std::env::var("GROQ_API_KEY").unwrap(),
            completions_url:"https://api.groq.com/openai/v1/chat/completions".into(),
        }
    }
}