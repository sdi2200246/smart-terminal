use std::env;
use std::error::Error;
use tempfile::TempDir;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::agent::memory::FolderMemory;
use smart_terminal::agent::workflows::next_cmd::{NextCmd, NextCommand, Reversibility};
use smart_terminal::core::memory::{Memory , Interaction};
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

    // Isolated memory store per test — nothing leaks into the user's real store.
    let tmp = TempDir::new().expect("tempdir");
    let mut memory = FolderMemory::new(tmp.path());
    let cwd = env::current_dir().expect("cwd");
    memory.register(&cwd).expect("register cwd");

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);

    let prediction = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        workflow
            .run(input)
            .await
            .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e.source()))
    };

    println!("\n═══ {label} ═══");
    println!("input:  {}", input);
    println!("cmd:    {}", prediction.cmd);
    println!("man:    {}", prediction.man);
    println!("scale:  {:?}", prediction.scale);

    assert!(!prediction.cmd.is_empty(), "[{label}] cmd empty");
    assert!(!prediction.man.is_empty(), "[{label}] man empty");

    // Memory plumbing assertion — proves the workflow actually persisted the call.
    let conv = memory.current().expect("memory should be loaded");
    assert_eq!(conv.interactions.len(), 1, "[{label}] interaction not persisted");
    assert_eq!(conv.interactions[0].user_input, input);
    assert_eq!(conv.interactions[0].predicted_cmd, prediction.cmd);

    // tmp stays alive until end of function — disk-backed memory is valid for the duration.
    drop(tmp);
    prediction
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn completes_partial_git_commit() {
    let _ = run_case("partial_commit", "git commit ").await;
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn translates_natural_language_docker() {
    let pred = run_case("natural_docker", "restart my db container").await;
    assert!(pred.cmd.contains("docker"));
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn destructive_command_is_flagged() {
    let pred = run_case("destructive", "git switch").await;
    assert!(matches!(pred.scale, Reversibility::Hard | Reversibility::Irreversible));
}

async fn run_case_with_history(
    label: &str,
    seeded: &[(&str, &str)],
    input: &str,
) -> NextCommand {
    init_test_tracing();

    let tmp = TempDir::new().expect("tempdir");
    let mut memory = FolderMemory::new(tmp.path());
    let cwd = env::current_dir().expect("cwd");
    memory.register(&cwd).expect("register cwd");

    for (user, cmd) in seeded {
        memory.append(Interaction {
            user_input: (*user).into(),
            predicted_cmd: (*cmd).into(),
            timestamp: 0,
        }).expect("seed");
    }

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);

    let prediction = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        workflow.run(input).await
            .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e.source()))
    };

    println!("\n═══ {label} ═══");
    println!("seeded: {} interactions", seeded.len());
    println!("input:  {}", input);
    println!("cmd:    {}", prediction.cmd);
    println!("man:    {}", prediction.man);

    drop(tmp);
    prediction
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn resolves_anaphora_to_prior_command() {
    let pred = run_case_with_history(
        "anaphora_undo",
        &[("whats the status ?", "git status'")],
        "git commit with appropiate meesage",
    ).await;

    let cmd = pred.cmd.to_lowercase();
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn reuses_prior_tool_choice() {
    let pred = run_case_with_history(
        "tool_continuity_rg",
        &[("search for fix", "rg fix" )],
        "find all files that have '//' on the first line ",
    ).await;

    assert!(
        pred.cmd.starts_with("rg"),
        "expected rg continuation, got: {}", pred.cmd
    );
}