use std::error::Error;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::agent::workflows::next_cmd::{NextCmd,NextCommand,Reversibility};
use smart_terminal::groq::client::GroqClient;


fn init_test_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_test_writer(),
        )
        .with(EnvFilter::new("warn,smart_terminal=debug"))
        .try_init()
        .ok();
}

async fn run_case(label: &str, input: &str) -> NextCommand {
    init_test_tracing();

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);
    let mut workflow = NextCmd::new(&mut runner);

    let prediction = workflow
        .run(input)
        .await
        .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e.source()));

    println!("\n═══ {label} ═══");
    println!("input:  {}", input);
    println!("cmd:    {}", prediction.cmd);
    println!("man:    {}", prediction.man);
    println!("scale:  {:?}", prediction.scale);
   
    assert!(!prediction.cmd.is_empty(), "[{label}] cmd empty");
    assert!(!prediction.man.is_empty(), "[{label}] man empty");

    prediction
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn completes_partial_git_commit() {
    let pred = run_case("partial_commit", "git commit ").await;
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn translates_natural_language_docker() {
    let (pred) = run_case("natural_docker", "restart my db container").await;
    assert!(pred.cmd.contains("docker"));
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn destructive_command_is_flagged() {
    let pred = run_case("destructive", "git switch").await;
    assert!(matches!(pred.scale, Reversibility::Hard | Reversibility::Irreversible));
}