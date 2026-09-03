#[cfg(test)]
mod integration {
    use smart_terminal::core::capability::ToolMetaData;
    use smart_terminal::core::llm_client::{AgentRequest, LLMProvider};
    use smart_terminal::core::model::{Model, ModelName};
    use smart_terminal::core::session::AgentSession;
    use smart_terminal::providers::groq::client::GroqClient;

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

        let mut client = GroqClient::default();

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
        assert_eq!(call.name(), "stop");
    }
}

#[cfg(test)]
mod integration_structured_responces {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use smart_terminal::core::llm_client::LLMProvider;
    use smart_terminal::core::session::AgentSession;
    use smart_terminal::providers::groq::client::GroqClient;
    use smart_terminal::utils::FlatSchema;

    fn session(user: &str) -> AgentSession {
        let mut s = AgentSession::new(5);
        s.add_system("You are a helpful assistant. Respond only with valid JSON matching the requested schema.");
        s.add_user(user.to_string());
        s
    }

    #[derive(JsonSchema, Deserialize, Debug)]
    #[schemars(deny_unknown_fields)]
    pub enum Reversibility { Full, Mostly, Partial, Hard, Irreversible }

    #[derive(JsonSchema, Deserialize)]
    #[schemars(deny_unknown_fields)]
    pub struct NextCommand {
        pub cmd: String,
        pub man: String,
        pub scale: Reversibility,
    }
    impl FlatSchema for NextCommand {}

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY"]
    async fn structured_returns_valid_next_command() {
        dotenv::dotenv().ok();
        let mut client = GroqClient::default();

        let session = session("git sta");
        let result = client
            .complete_structured(&session, NextCommand::schema())
            .await;

        assert!(result.is_ok(), "complete_structured failed: {:?}", result.err());
        let value = result.unwrap();
        let parsed: NextCommand = serde_json::from_value(value).expect("schema mismatch");
        assert!(!parsed.cmd.is_empty());
        assert!(!parsed.man.is_empty());
    }
}