#[cfg(test)]
mod integration {
    use std::print;

use smart_terminal::core::capability::ToolMetaData;
    use smart_terminal::core::llm_client::{AgentRequest, LLMProvider};
    use smart_terminal::core::model::{Model, ModelName};
    use smart_terminal::core::session::AgentSession;
    use smart_terminal::providers::google::client::GoogleClient;

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

        // Direct instantiation using provider defaults
        let mut client = GoogleClient::default();

        let session = simple_session();
        let model = Model::new(ModelName::Gemini2_5Flash, 0.7);
        let tools_metadata: Vec<ToolMetaData> = vec![];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;
        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[ignore]
    async fn complete_returns_real_tool_call() {
        dotenv::dotenv().ok();

        let mut client = GoogleClient::default();

        let mut session = AgentSession::new(5);
        session.add_system(
            "You are a helpful assistant. Use the get_weather tool when asked about weather.",
        );
        session.add_user("What's the weather in Athens?");

        let model = Model::new(ModelName::Gemini2_5Flash, 0.7);

        let tools_metadata = vec![ToolMetaData {
            name: "get_weather".into(),
            description: "Get current weather for a city".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }),
        }];

        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &tools_metadata,
        };

        let result = client.complete(request).await;
        assert!(result.is_ok(), "complete() failed: {:?}", result.err());

        let call = result.unwrap();
        print!("{:?}" , call);
    }

    #[cfg(test)]
    mod integration_structured_responces {
        use schemars::JsonSchema;
        use serde::Deserialize;
        use smart_terminal::core::llm_client::LLMProvider;
        use smart_terminal::core::session::AgentSession;
        use smart_terminal::providers::google::client::GoogleClient;
        use smart_terminal::utils::FlatSchema;

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
            pub cmd: String,
            pub man: String,
            pub scale: Reversibility,
        }
        impl FlatSchema for NextCommand {}

        #[tokio::test]
        #[ignore = "requires GOOGLE_API_KEY"]
        async fn structured_returns_valid_next_command() {
            dotenv::dotenv().ok();
            let mut client = GoogleClient::default();

            let session = session("give me an appropiriate commit message uisng your tools");
            let result = client
                .complete_structured(&session, NextCommand::schema())
                .await;

            assert!(
                result.is_ok(),
                "complete_structured failed: {:?}",
                result.err()
            );
            let value = result.unwrap();
            let parsed: NextCommand = serde_json::from_value(value).expect("schema mismatch");
            assert!(!parsed.cmd.is_empty());
            assert!(!parsed.man.is_empty());
        }
    }
}
