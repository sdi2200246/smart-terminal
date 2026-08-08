    #[cfg(test)]
    mod integration {
        use std::print;

    use smart_terminal::core::capability::ToolMetaData;
        use smart_terminal::core::llm_client::{AgentRequest, LLMProvider};
        use smart_terminal::core::model::{Model, ModelName};
        use smart_terminal::core::session::AgentSession;
        use smart_terminal::google::client::GoogleClient;

        fn simple_session() -> AgentSession {
            let mut session = AgentSession::new(5);
            session.add_system("You are a helpful assistant.");
            session.add_user("What is 1 + 1?");
            session
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
        #[ignore]
        async fn complete_returns_tool_call() {
            dotenv::dotenv().ok();

            let key = std::env::var("GOOGLE_API_KEY")
                .expect("GOOGLE_API_KEY must be set to run integration tests");

            let mut client = GoogleClient {
                client: reqwest::Client::new(),
                api_key: key,
                completions_url: "https://generativelanguage.googleapis.com/v1beta/models/".into(),
            };

            let session = simple_session();
            let model = Model::new(ModelName::Gemini2_5Flash, 0.7);
            let tools_metadata: Vec<ToolMetaData> = vec![];

            let request = AgentRequest {
                model: &model,
                session: &session,
                tools_metadata: &tools_metadata,
            };

            let result = client.complete(request).await;
            print!("{:?}" , result);

            assert!(result.is_ok(), "complete() failed: {:?}", result.err());

            let call = result.unwrap();
            println!(
                "Got tool call: {} with args: {}",
                call.name(),
                call.arguments()
            );
            assert_eq!(call.name(), "stop");
        }
    
    
    
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore]
    async fn complete_returns_real_tool_call() {
        dotenv::dotenv().ok();

        let key = std::env::var("GOOGLE_API_KEY")
            .expect("GOOGLE_API_KEY must be set to run integration tests");

        let mut client = GoogleClient {
            client: reqwest::Client::new(),
            api_key: key,
            completions_url: "https://generativelanguage.googleapis.com/v1beta/models/".into(),
        };

        let mut session = AgentSession::new(5);
        session.add_system("You are a helpful assistant. Use the get_weather tool when asked about weather.");
        session.add_user("What's the weather in Athens?");

        let model = Model::new(ModelName::Gemini2_5Flash, 0.7);

        let tools_metadata = vec![ToolMetaData {
            name: "get_weather".into(),
            description: "Get current weather for a city".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                },
                "required": ["city"]
            }),
        }];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;
        println!("{:?}", result);

        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        let call = result.unwrap();
        println!("Got tool call: {} with args: {}", call.name(), call.arguments());

        // Forces the actual functionCall parse path + to_gemini_schema conversion,
        // not just the finishReason=STOP text path.
        assert_eq!(call.name(), "get_weather");
        assert!(call.arguments().get("city").is_some());
    }
}
