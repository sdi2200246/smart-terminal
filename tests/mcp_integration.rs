use schemars::JsonSchema;
use serde_json::Value;
use smart_terminal::agent::responce::AgentResponse;
use tokio::sync::mpsc;
use smart_terminal::agent::service::AgentService;
use smart_terminal::agent::request::{AgentRequest , ToolNames};
use smart_terminal::protocol::message::Message;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use rand::{seq::SliceRandom, Rng};
use rand::thread_rng;

#[derive(Deserialize , Debug , JsonSchema)]
struct FinalAnswer {
    cmd: String,
}
#[derive(Deserialize , Debug , Serialize)]
struct Context{
    buffer:String,
    cwd:String,
    history:Vec<String>,
}

#[tokio::test]
#[ignore]
async fn test_real_model_end_to_end_2() {

    dotenv::dotenv().ok();

    if std::env::var("GROQ_API_KEY").is_err() {
        println!("Skipping test: GROQ_API_KEY not set");
        return;
    }

    let (tx, mut rx) = mpsc::channel(8);

    let service = AgentService::spawn();

    let context = Context{
        buffer:"git commit -m ?".into(),
        cwd:"home/smart_terminal".into(),
        history:vec!["ls".into() , "ls".into() , "cargo".into()]
    };

    let root = schemars::schema_for!(FinalAnswer);

    let properties: Value = serde_json::to_value(
        &root.schema.object.as_ref().unwrap().properties
    ).unwrap();

    let sys_promt =
                "You are an intelligent Bash command prediction engine embedded inside a Smart Terminal.
                Your task is to predict the most likely next complete command the user is trying to execute.

                Rules:
                - Output ONLY the full Bash command. No explanations. No markdown.
                - If the command depends on filesystem state, git state, running processes, or other dynamic context, you MUST call the appropriate tool to increase prediction accuracy.
                - Never guess when a tool can reduce uncertainty.
                - If tools are available that improve confidence, use them.
                - The prediction must be executable in a real shell and complete.
                - Every tool should be used only at most once!
                - The tool with name:final_answer must be used as the final tool and it must be called!
                Your goal is to provide the highest-confidence, context-aware prediction possible."
                .to_string();

    let request = 
                AgentRequest::new(vec![ToolNames::GitDiffStaged] , vec![] , properties , tx)
                .with_system_promt(sys_promt)
                .with_context(&context);

    let _ = service.send(request).await;

    let res = rx.recv().await.unwrap();
    
    match res{
        AgentResponse::Success(obj) =>{
            let cmd:FinalAnswer = serde_json::from_str(obj.as_str().unwrap()).unwrap();
            println!("{:?}" , cmd);
        }        
        AgentResponse::Error(e)=>{println!("{:?}" , e);}
    }

}


#[tokio::test]
#[ignore]
async fn test_model_success_rate_randomized() {

    dotenv::dotenv().ok();

    if std::env::var("GROQ_API_KEY").is_err() {
        println!("Skipping test: GROQ_API_KEY not set");
        return;
    }

    const RUNS: usize = 25;

    let service = AgentService::spawn();

    let mut success = 0;
    let mut failure = 0;
    let mut invalid_json = 0;

   let possible_buffers = vec![
            "",
            // Git realistic partials
            "git ",
            "git com",
            "git commit -",
            "git push ",
            "git checkout ",
            "git reba",
            "git diff --",
            "git log --o",
            "git reset --h",

            // Cargo realistic partials
            "cargo ",
            "cargo bui",
            "cargo run --",
            "cargo test ",
            "cargo cl",
            "cargo fmt",
            "cargo check",

            // Process / system
            "ps ",
            "ps aux | gr",
            "top",
            "kill ",
            "htop",

            // Docker
            "docker ",
            "docker ps",
            "docker stop ",
            "docker logs ",
            "docker run -",

            // Filesystem
            "ls ",
            "ls -l",
            "ls -a",
            "cd ",
            "cd src",
            "mkdir ",
            "rm -rf ",
            "grep ",
            "grep -r main",

            // Build tools
            "make ",
            "npm ",
            "npm run ",
            "python ",
            "python3 ",
        ];


    let possible_history = vec![
            "git status",
            "git add .",
            "git add src/groq/client.rs",
            "git commit -m \"fix bug\"",
            "git push origin main",
            "git diff",
            "git diff --staged",
            "git checkout -b feature/refactor",
            "git pull",

            "cargo build",
            "cargo test",
            "cargo run",
            "cargo check",
            "cargo fmt",
            "cargo clippy",


            "docker ps",
            "docker build -t smart-terminal .",
            "docker run -p 8080:8080 app",
            "docker stop 3f21a",
            "docker logs container_id",

            "ps aux",
            "ps aux | grep smart",
            "top",
            "kill -9 1234",

            // File navigation
            "ls",
            "ls -la",
            "cd src",
            "cd ..",
            "mkdir target",
            "rm -rf target",
            "grep -r Groq .",

            // Mixed dev workflow
            "npm install",
            "npm run build",
            "python3 main.py",
            "make build",
        ];

    for i in 0..RUNS {

        println!("Run {}/{}", i + 1, RUNS);

        let (tx, mut rx) = mpsc::channel(8);

        let mut rng = thread_rng();

        let buffer = possible_buffers
            .choose(&mut rng)
            .unwrap()
            .to_string();

        let history_len = rng.gen_range(1..=4);

        let history: Vec<String> = (0..history_len)
            .map(|_| {
                possible_history
                    .choose(&mut rng)
                    .unwrap()
                    .to_string()
            })
            .collect();

        let context = Context{
            buffer,
            cwd:"home/smart_terminal".into(),
            history,
        };

        let root = schemars::schema_for!(FinalAnswer);

        let properties: serde_json::Value = serde_json::to_value(
            &root.schema.object.as_ref().unwrap().properties
        ).unwrap();

        let messages = vec![
            Message::system(Some(
                "You are an intelligent Bash command prediction engine embedded inside a Smart Terminal.
                Your task is to predict the most likely next complete command the user is trying to execute.

                Rules:
                - Output ONLY the full Bash command.
                - The tool with name:final_answer must be used as the final tool.
                - Never guess when a tool can reduce uncertainty."
                .into()
            )),
            Message::context(&context),
        ];

        let request = AgentRequest::new(
            vec![ToolNames::GitDiffStaged , ToolNames::ProcessList , ToolNames::GitStatus],
            messages,
            properties,
            tx
        );

        let _ = service.send(request).await;

        match rx.recv().await {
            Some(AgentResponse::Success(obj)) => {
                match serde_json::from_str::<FinalAnswer>(obj.as_str().unwrap()) {
                    Ok(cmd) => {
                        println!("SUCCESS: {:?}", cmd);
                        success += 1;
                    }
                    Err(_) => {
                        println!("INVALID JSON");
                        invalid_json += 1;
                    }
                }
            }
            Some(AgentResponse::Error(e)) => {
                println!("ERROR: {:?}", e);
                failure += 1;
            }
            None => {
                println!("NO RESPONSE");
                failure += 1;
            }
        }

        // ⏳ 5 second delay
        sleep(Duration::from_secs(5)).await;
    }

    println!("\n=========================");
    println!("Total Runs: {}", RUNS);
    println!("Success: {}", success);
    println!("Failures: {}", failure);
    println!("Invalid JSON: {}", invalid_json);
    println!(
        "Success Rate: {:.2}%",
        (success as f64 / RUNS as f64) * 100.0
    );
}