#[cfg(test)]
mod integration {
    use smart_terminal::groq::client::GroqClient;
    use smart_terminal::core::llm_client::LLMProvider;
    use smart_terminal::core::session::{AgentSession, Model, ModelName};
    use smart_terminal::core::capability::ToolFunction;
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
        let mut session = AgentSession::new(
            vec![final_answer_tool()],
            5,
            Model::new(ModelName::GptOss120B, 0.7),
        );
        session.add_system("You are a helpful assistant. You MUST always respond by calling the final_answer tool.");
        session.add_user("What is 1 + 1?");
        session
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore]
    async fn complete_returns_tool_call() {
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

        let call = result.unwrap();
        println!("Got tool call: {} with args: {}", call.name(), call.arguments());
        assert_eq!(call.name(), "final_answer");
        assert!(call.arguments().get("result").is_some(), "missing 'result' key");
    }
}