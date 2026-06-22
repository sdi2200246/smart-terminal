#[cfg(test)]
mod integration {
    use smart_terminal::core::capability::ToolMetaData;
    use smart_terminal::core::llm_client::{AgentRequest, LLMProvider};
    use smart_terminal::core::session::{AgentSession, Model, ModelName};
    use smart_terminal::groq::client::GroqClient;

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

        let key = std::env::var("GROQ_API_KEY")
            .expect("GROQ_API_KEY must be set to run integration tests");

        let mut client = GroqClient {
            client: reqwest::Client::new(),
            api_key: key,
            completions_url: "https://api.groq.com/openai/v1/chat/completions".into(),
        };

        let session = simple_session();
        let model = Model::new(ModelName::GptOss120B, 0.7);
        let tools_metadata: Vec<ToolMetaData> = vec![];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;

        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        let call = result.unwrap();
        println!(
            "Got tool call: {} with args: {}",
            call.name(),
            call.arguments()
        );
        assert_eq!(call.name(), "stop");
    }
}

#[cfg(test)]
mod integration_structured_responces {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use smart_terminal::core::llm_client::LLMProvider;
    use smart_terminal::core::session::AgentSession;
    use smart_terminal::groq::client::GroqClient;
    use smart_terminal::utils::FlatSchema;

    fn client() -> GroqClient {
        dotenv::dotenv().ok();
        GroqClient {
            client: reqwest::Client::new(),
            api_key: std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set"),
            completions_url: "https://api.groq.com/openai/v1/chat/completions".into(),
        }
    }

    fn session(user: &str) -> AgentSession {
        let mut s = AgentSession::new(5);
        s.add_system("You are a helpful assistant. Respond only with valid JSON matching the requested schema.");
        s.add_user(user.to_string());
        s
    }

    #[derive(JsonSchema, Deserialize, Debug)]
    #[schemars(deny_unknown_fields)]
    pub enum Reversibility {
        Full,
        Mostly,
        Partial,
        Hard,
        Irreversible,
    }

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    pub struct NextCommand {
        /// Shell executable command.
        pub cmd: String,
        /// Very compressed description of the shell command
        pub man: String,
        /// How reversible the command is given the current environment.
        pub scale: Reversibility,
    }
    impl FlatSchema for NextCommand {}

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY"]
    async fn structured_returns_valid_next_command() {
        let mut client = client();
        let session = session("git sta");
        let result = client
            .complete_structured(&session, NextCommand::schema())
            .await;

        assert!(
            result.is_ok(),
            "complete_structured failed: {:?}",
            result.err()
        );
        let value = result.unwrap();
        println!("raw value: {value}");

        let parsed: NextCommand = serde_json::from_value(value).expect("schema mismatch");
        assert!(!parsed.cmd.is_empty());
        assert!(!parsed.man.is_empty());
        println!(
            "cmd: {}\nman: {}\nscale: {:?}",
            parsed.cmd, parsed.man, parsed.scale
        );
    }
}
