mod integration {
    use smart_terminal::groq::client::GroqClient;
    use smart_terminal::agent::service::AgentService;
    use smart_terminal::agent::request::{AgentRequest, ToolNames};
    use smart_terminal::agent::responce::AgentResponse;
    use serde::Serialize;
    use serde_json::json;
    use tokio::sync::mpsc;

    #[derive(Serialize)]
    struct GitContext {
        current_branch: &'static str,
        recent_commands: Vec<&'static str>,
        project_description: &'static str,
    }

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY and live network"]
    async fn predicts_next_git_command() {
        dotenv::dotenv().ok();

        let client = GroqClient::default();
        let tx = AgentService::spawn(client);

        let (response_tx, mut response_rx) = mpsc::channel(1);

        let ctx = GitContext {
            current_branch: "feature/add-agent-service",
            recent_commands: vec![
                "git checkout -b feature/add-agent-service",
                "git add src/agent/service.rs",
            ],
            project_description: "A Rust CLI agent that uses LLMs to assist with developer tasks",
        };

        let contract = json!({
            "next_cmd": { "type": "string" }
        });

        let req = AgentRequest::builder(response_tx)
            .tools(vec![ToolNames::GitStatus, ToolNames::GitDiffStaged , ToolNames::ProcessList])
            .contract(contract)
            .with_context(&ctx)
            .with_system_promt(
                "You are an expert git assistant embedded in a developer's terminal. \
                You have access to tools to inspect the current repository state. \
                Use them to understand what has changed, then recommend the single most \
                appropriate next git command the developer should run. \
                Be concise — return only the raw command, no explanation.".into()
            );

        tx.send(req).await.unwrap();

        let response = response_rx.recv().await.unwrap();

        match response {
            AgentResponse::Success(value) => {
                println!("predicted next_cmd: {}", value["next_cmd"]);
                assert!(value["next_cmd"].is_string(), "expected next_cmd to be a string");
                assert!(!value["next_cmd"].as_str().unwrap().is_empty(), "expected next_cmd to not be empty");
            }
            AgentResponse::Error(e) => panic!("agent failed: {e:?}"),
        }
    }
}