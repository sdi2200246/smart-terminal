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
                Be concise — return only the raw command, no explanation. ".into()
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


    #[derive(Serialize)]
    struct TerminalContext {
        shell: &'static str,
        cwd: &'static str,
        buffer: &'static str,
        history: Vec<&'static str>,
    }

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY and live network"]
    async fn predicts_next_terminal_command() {
        dotenv::dotenv().ok();

        let client = GroqClient::default();
        let tx = AgentService::spawn(client);

        let (response_tx, mut response_rx) = mpsc::channel(1);

        let ctx = TerminalContext {
            shell: "zsh",
            cwd: "/Users/jason/smart-terminal",
            buffer: "",
            history: vec![
                "cargo test",
                "git add .",
            ],
        };

        let contract = json!({
            "next_cmd": { "type": "string" }
        });

        let req = AgentRequest::builder(response_tx)
            .tools(vec![ToolNames::GitStatus, ToolNames::GitDiffStaged, ToolNames::ProcessList])
            .contract(contract)
            .with_context(&ctx)
            .with_system_promt(
                "You are an intelligent terminal assistant embedded in a developer's shell. \
                You have access to tools to inspect the current state of the system and repository. \
                Use them to understand what the developer is working on, then predict the single \
                most appropriate next terminal command they should run based on their shell, \
                current directory, recent history, and what is currently in their buffer. \
                Be concise — next_cmd must be the raw command only, no explanation. \
                You MUST use the final_answer tool to submit your answer.".into()
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




    #[derive(Serialize)]
    struct CodeEvalContext {
        source_code: &'static str,
        goal: &'static str,
    }

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY and live network"]
    async fn evaluates_code_quality() {
        dotenv::dotenv().ok();

        let client = GroqClient::default();
        let tx = AgentService::spawn(client);

        let (response_tx, mut response_rx) = mpsc::channel(1);

        let ctx = CodeEvalContext {
            source_code: r#"
                #include <stdio.h>
                #include <stdlib.h>
                #include <string.h>

                #define MAX_STACK 100

                typedef struct {
                    double items[MAX_STACK];
                    int top;
                } Stack;

                void stack_init(Stack* s) {
                    s->top = 0;
                }

                void stack_push(Stack* s, double val) {
                    s->items[s->top] = val;
                    s->top++;
                }

                double stack_pop(Stack* s) {
                    s->top--;
                    return s->items[s->top];
                }

                double evaluate(const char* expr) {
                    Stack s;
                    stack_init(&s);

                    char buf[256];
                    strncpy(buf, expr, sizeof(buf));

                    char* token = strtok(buf, " ");
                    while (token != NULL) {
                        if (strcmp(token, "+") == 0 || strcmp(token, "-") == 0 ||
                            strcmp(token, "*") == 0 || strcmp(token, "/") == 0) {

                            double a = stack_pop(&s);
                            double b = stack_pop(&s);

                            if (strcmp(token, "+") == 0) stack_push(&s, a + b);
                            else if (strcmp(token, "-") == 0) stack_push(&s, a - b);
                            else if (strcmp(token, "*") == 0) stack_push(&s, a * b);
                            else if (strcmp(token, "/") == 0) stack_push(&s, a / b);
                        } else {
                            stack_push(&s, atof(token));
                        }
                        token = strtok(NULL, " ");
                    }

                    return stack_pop(&s);
                }

                int main() {
                    printf("%f\n", evaluate("3 4 + 2 *")); // expects 14
                    return 0;
                }

            "#,
            goal: " Goal: a stack-based calculator that evaluates postfix expressions , Example: 3 4 + 2 * should give 14",
        };

        let contract = json!({
            "severity": {
                "type": "string",
                "enum": ["none", "warning", "error", "critical"]
            },
            "issues": {
                "type": "array",
                "items": { "type": "string" }
            },
            "report": {
                "type": "string"
            }
        });

        let req = AgentRequest::builder(response_tx)
            .tools(vec![])
            .contract(contract)
            .with_context(&ctx)
            .with_system_promt(
                "You are an expert C code reviewer embedded in a developer's IDE. \
                You will be given source code and the goal of the program. \
                First infer the current development stage from the code itself — \
                is it a skeleton, work in progress, or complete? \
                Then evaluate only what is written, not what is missing. \
                You should not include actions on how to resolve only flag the problems.\
                I want you report wirtten in greece.\
                Assign a severity level: \
                'none' if the code is correct and progressing well, \
                'warning' if there are minor issues or bad patterns but nothing breaking, \
                'error' if there are bugs or the code will not compile, \
                'critical' if the code is fundamentally wrong or going in the wrong direction. \
                You MUST use the final_answer tool to submit your answer.".into()

            );

        tx.send(req).await.unwrap();

        let response = response_rx.recv().await.unwrap();

        match response {
            AgentResponse::Success(value) => {
                println!("severity: {}", value["severity"]);
                println!("report: {}", value["report"]);
                println!("issues: {}", value["issues"]);
                assert!(value["severity"].is_string());
                assert!(value["report"].is_string());
                assert!(value["issues"].is_array());
            }
            AgentResponse::Error(e) => panic!("agent failed: {e:?}"),
        }
    }


}


