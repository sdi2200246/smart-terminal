#[cfg(test)]
mod integration {
    use smart_terminal::groq::client::GroqClient;
    use smart_terminal::interfaces::llm_client::LLMProvider;
    use smart_terminal::interfaces::session::{AgentSession, AgentOutcome, ConversationEvent , Model};
    use smart_terminal::interfaces::capability::ToolFunction;
    use serde_json::json;

    fn final_answer_tool() -> ToolFunction {
        ToolFunction {
            name: "final_answer".into(),
            description: "Return the final answer to the user".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "result": {
                        "type": "string",
                        "description": "The final answer"
                    }
                },
                "required": ["result"]
            }),
        }
    }

    fn simple_session() -> AgentSession {
        AgentSession {
            events: vec![
                ConversationEvent::System(
                    "You are a helpful assistant. You MUST always respond by calling the final_answer tool.".into()
                ),
                ConversationEvent::User("What is 1 + 1?".into()),
            ],
            available_tools: vec![final_answer_tool()],
            steps: 0,
            model:Model::GptOss120B,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore]
    async fn complete_returns_final_answer() {

        dotenv::dotenv().ok();

        let key = std::env::var("GROQ_API_KEY")
            .expect("GROQ_API_KEY must be set to run integration tests");

        let mut client = GroqClient {
            client: reqwest::Client::new(),
            api_key: key,
            completions_url: "https://api.groq.com/openai/v1/chat/completions".into(),
        };

        let session = simple_session();
        let result = client.complete(&session).await;

        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        match result.unwrap() {
            AgentOutcome::FinalAnswer { arguments } => {
                println!("Got final answer: {}", arguments);
                assert!(arguments.get("result").is_some(), "missing 'result' key");
            }
            AgentOutcome::Tool { name, .. } => {
                panic!("Expected FinalAnswer but got tool call: {}", name);
            }
        }
    }
}