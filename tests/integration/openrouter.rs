#[cfg(test)]
mod integration {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use smart_terminal::core::{
        AgentRequest, CoreAgentSession, LLMProvider, Model, ModelName, ToolMetaData, ToolSafety,
    };
    use smart_terminal::providers::OpenRouterClient;
    use smart_terminal::utils::FlatSchema;

    fn init_test_env() {
        dotenv::dotenv().ok();
    }

    fn simple_session(user_msg: &str) -> CoreAgentSession {
        CoreAgentSession::new()
            .with_system("You are a helpful assistant.")
            .with_user(user_msg)
    }

    /// Test 1: Basic completion / chat response.
    /// Runs a simple prompt and expects the model to return a conclusion.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore = "requires OPENROUTER_API_KEY"]
    async fn complete_returns_conclusion() {
        init_test_env();

        let client = OpenRouterClient::default();
        let session = simple_session("What is 2 + 2? Reply with just the number.");
        let model = Model::new(ModelName::Nemotron3_5SuperVlFree, 0.1);
        let tools_metadata: Vec<ToolMetaData> = vec![];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;
        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        let response = result.unwrap();
        assert!(!response.is_empty(), "expected non-empty response");
        let first_call = &response.calls()[0];
        println!("Response from OpenRouter: {:?}", first_call);
        assert_eq!(first_call.name(), "stop");
    }

    /// Test 2: Tool calling.
    /// Passes a function declaration and asks the model a question that triggers the tool.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore = "requires OPENROUTER_API_KEY"]
    async fn complete_returns_real_tool_call() {
        init_test_env();

        let client = OpenRouterClient::default();

        let session = CoreAgentSession::new()
            .with_system(
                "You are a helpful assistant. Use the get_weather tool when asked about weather.",
            )
            .with_user("What is the weather in Athens?");

        let model = Model::new(ModelName::Nemotron3_5SuperVlFree, 0.1);

        let tools_metadata = vec![ToolMetaData {
            name: "get_weather".into(),
            description: "Get current weather for a city".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }),
            safety: ToolSafety::Safe,
        }];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;
        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        let response = result.unwrap();
        assert!(!response.is_empty(), "expected at least one tool call");
        let call = &response.calls()[0];
        println!("Tool call from OpenRouter: {:?}", call);
        assert_eq!(call.name(), "get_weather");
        assert!(
            call.arguments().get("city").is_some(),
            "expected 'city' argument in tool call"
        );
    }

    /// Test 3: Structured JSON output.
    /// Requests a strongly-typed schema and deserializes the JSON response.
    #[derive(JsonSchema, Deserialize, Debug)]
    #[schemars(deny_unknown_fields)]
    pub enum Reversibility {
        Full,
        Mostly,
        Partial,
        Hard,
        Irreversible,
    }

    #[derive(JsonSchema, Deserialize, Debug)]
    #[schemars(deny_unknown_fields)]
    pub struct NextCommand {
        pub cmd: String,
        pub man: String,
        pub scale: Reversibility,
    }

    impl FlatSchema for NextCommand {}

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore = "requires OPENROUTER_API_KEY"]
    async fn structured_returns_valid_next_command() {
        init_test_env();

        let client = OpenRouterClient::default();

        let session = CoreAgentSession::new()
            .with_system("You are a helpful shell assistant. Respond with JSON matching schema.")
            .with_user("git sta");

        let result = client
            .complete_structured(&session, NextCommand::schema())
            .await;

        assert!(
            result.is_ok(),
            "complete_structured failed: {:?}",
            result.err()
        );

        let value = result.unwrap();
        println!("Structured output JSON: {}", value);
        let parsed: NextCommand =
            serde_json::from_value(value).expect("failed to deserialize into NextCommand");
        assert!(!parsed.cmd.is_empty());
        assert!(!parsed.man.is_empty());
    }
}
