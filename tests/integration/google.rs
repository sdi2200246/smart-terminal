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


    #[cfg(test)]
    mod integration_structured_responces {
        use schemars::JsonSchema;
        use serde::Deserialize;
        use smart_terminal::core::llm_client::LLMProvider;
        use smart_terminal::core::session::AgentSession;
        use smart_terminal::google::client::GoogleClient;
        use smart_terminal::utils::FlatSchema;

        fn client() -> GoogleClient {
            dotenv::dotenv().ok();
            GoogleClient {
                client: reqwest::Client::new(),
                api_key: std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY must be set"),
                completions_url: "https://generativelanguage.googleapis.com/v1beta/models/".into(),
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
        #[ignore = "requires GOOGLE_API_KEY"]
        async fn structured_returns_valid_next_command() {
            let mut client = client();
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

}
