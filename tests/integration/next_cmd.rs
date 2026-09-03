use smart_terminal::agent::memory::FolderMemory;
use smart_terminal::agent::patterns::react::ReactLoop;
use smart_terminal::agent::workflows::next_cmd::{NextCmd, NextCommand};
use smart_terminal::core::llm_client::LLMProvider;
use smart_terminal::core::memory::{Interaction, Memory};
use smart_terminal::providers::google::client::GoogleClient;
use smart_terminal::providers::groq::client::GroqClient;
use std::env;
use tempfile::TempDir;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

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

async fn run_case<P: LLMProvider>(label: &str, input: &str, provider: P) -> NextCommand {
    init_test_tracing();
    dotenv::dotenv().ok();

    let tmp = TempDir::new().expect("tempdir");
    let mut memory = FolderMemory::new(tmp.path());
    let cwd = env::current_dir().expect("cwd");
    memory.register(&cwd).expect("register cwd");

    let mut runner = ReactLoop::new(provider);

    let prediction = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        workflow
            .run(input)
            .await
            .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e))
    };

    println!("\n═══ {label} ═══");
    println!("input:  {}", input);
    println!("cmd:    {}", prediction.cmd);
    println!("man:    {}", prediction.man);
    println!("scale:  {:?}", prediction.scale);

    assert!(!prediction.cmd.is_empty(), "[{label}] cmd empty");
    assert!(!prediction.man.is_empty(), "[{label}] man empty");

    drop(tmp);
    prediction
}

async fn run_case_with_history<P: LLMProvider>(
    label: &str,
    seeded: &[(&str, &str)],
    input: &str,
    provider: P,
) -> NextCommand {
    init_test_tracing();
    dotenv::dotenv().ok();

    let tmp = TempDir::new().expect("tempdir");
    let mut memory = FolderMemory::new(tmp.path());
    let cwd = env::current_dir().expect("cwd");
    memory.register(&cwd).expect("register cwd");

    for (user, cmd) in seeded {
        memory
            .append(Interaction {
                user_input: (*user).into(),
                predicted_cmd: (*cmd).into(),
                timestamp: 0,
            })
            .expect("seed");
    }

    let mut runner = ReactLoop::new(provider);

    let prediction = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        workflow
            .run(input)
            .await
            .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e))
    };

    println!("\n═══ {label} ═══");
    println!("seeded: {} interactions", seeded.len());
    println!("input:  {}", input);
    println!("cmd:    {}", prediction.cmd);
    println!("man:    {}", prediction.man);

    drop(tmp);
    prediction
}

// ── Partial Git Commit ──
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn completes_partial_git_commit_groq() {
    let _ = run_case("partial_commit_groq", "git commit ", GroqClient::pooled()).await;
}

#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn completes_partial_git_commit_google() {
    let _ = run_case("partial_commit_google", "git commit ", GoogleClient::pooled()).await;
}

// ── Natural Language Translation ──
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn translates_natural_language_docker_groq() {
    let pred = run_case("natural_docker_groq", "restart my db container", GroqClient::pooled()).await;
    assert!(pred.cmd.contains("docker"));
}

#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn translates_natural_language_docker_google() {
    let pred = run_case("natural_docker_google", "restart my db container", GoogleClient::pooled()).await;
    assert!(pred.cmd.contains("docker"));
}

// ── History & Error Context (Last Error Push) ──
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn reads_last_error_push_upstream_groq() {
    let err_file = tempfile::NamedTempFile::new().expect("err file");
    std::fs::write(
        err_file.path(),
        "fatal: The current branch feature/token-usage has no upstream branch.\n\
         To push the current branch and set the remote as upstream, use\n\n\
         \tgit push --set-upstream origin feature/token-usage\n",
    )
    .expect("write err");

    unsafe { env::set_var("ERR_LAST", err_file.path()); }

    let pred = run_case_with_history(
        "last_error_push_upstream_groq",
        &[("push my changes", "git push")],
        "it failed",
        GroqClient::pooled()
    )
    .await;

    let cmd = pred.cmd.to_lowercase();
    assert!(cmd.contains("set-upstream") || cmd.contains("-u "));
}

#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn reads_last_error_push_upstream_google() {
    let err_file = tempfile::NamedTempFile::new().expect("err file");
    std::fs::write(
        err_file.path(),
        "fatal: The current branch feature/token-usage has no upstream branch.\n\
         To push the current branch and set the remote as upstream, use\n\n\
         \tgit push --set-upstream origin feature/token-usage\n",
    )
    .expect("write err");

    unsafe { env::set_var("ERR_LAST", err_file.path()); }

    let pred = run_case_with_history(
        "last_error_push_upstream_google",
        &[("push my changes", "git push")],
        "it failed",
        GoogleClient::pooled()
    )
    .await;

    let cmd = pred.cmd.to_lowercase();
    assert!(cmd.contains("set-upstream") || cmd.contains("-u "));
}