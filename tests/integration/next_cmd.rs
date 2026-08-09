use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::agent::memory::FolderMemory;
use smart_terminal::agent::workflows::next_cmd::{NextCmd, NextCommand, Reversibility};
use smart_terminal::core::memory::{Interaction, Memory};
use smart_terminal::groq::client::GroqClient;
use smart_terminal::google::client::GoogleClient;
use std::env;
use std::error::Error;
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
fn google_client() -> GoogleClient {
    dotenv::dotenv().ok();
    GoogleClient {
        client: reqwest::Client::new(),
        api_key: std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY must be set"),
        completions_url: "https://generativelanguage.googleapis.com/v1beta/models/".into(),
    }
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
    assert_eq!(
        conv.interactions.len(),
        1,
        "[{label}] interaction not persisted"
    );
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
    assert!(matches!(
        pred.scale,
        Reversibility::Hard | Reversibility::Irreversible
    ));
}

async fn run_case_with_history(label: &str, seeded: &[(&str, &str)], input: &str) -> NextCommand {
    init_test_tracing();

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
    )
    .await;

    let cmd = pred.cmd.to_lowercase();
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn reuses_prior_tool_choice() {
    let pred = run_case_with_history(
        "tool_continuity_rg",
        &[("search for fix", "rg fix")],
        "search for smilar comment devs use  ",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn reads_last_error_and_corrects_command() {
    // Same failing command ("git push") has two common errors with DIFFERENT fixes:
    //   - no upstream        -> git push --set-upstream origin <branch>
    //   - rejected non-ff    -> git pull --rebase
    // The model can only choose correctly by actually reading stderr.
    let err_file = tempfile::NamedTempFile::new().expect("err file");
    std::fs::write(
        err_file.path(),
        "fatal: The current branch feature/token-usage has no upstream branch.\n\
         To push the current branch and set the remote as upstream, use\n\n\
         \tgit push --set-upstream origin feature/token-usage\n",
    )
    .expect("write err");

    unsafe {
        env::set_var("ERR_LAST", err_file.path());
    }

    let pred = run_case_with_history(
        "last_error_push_upstream",
        &[("push my changes", "git push")],
        "it failed",
    )
    .await;

    let cmd = pred.cmd.to_lowercase();
    assert!(
        cmd.contains("set-upstream") || cmd.contains("-u "),
        "expected upstream fix, got: {}",
        pred.cmd
    );
    assert!(
        cmd.contains("feature/token-usage"),
        "branch name only exists in stderr — model didn't read the error. got: {}",
        pred.cmd
    );
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn corrects_command_when_docker_daemon_down() {
    let err_file = tempfile::NamedTempFile::new().expect("err file");
    std::fs::write(
        err_file.path(),
        "Cannot connect to the Docker daemon at unix:///var/run/docker.sock. \
         Is the docker daemon running?\n",
    )
    .expect("write err");

    unsafe {
        env::set_var("ERR_LAST", err_file.path());
    }

    let pred = run_case_with_history(
        "last_error_daemon_down",
        &[("show my containers", "docker ps")],
        "soemthing went wrong",
    )
    .await;

    let cmd = pred.cmd.to_lowercase();
    assert!(
        cmd.contains("colima start")
            || cmd.contains("open -a docker")
            || cmd.contains("systemctl start docker")
            || cmd.contains("dockerd"),
        "expected a daemon-start command, got: {}",
        pred.cmd
    );
    assert!(
        !cmd.trim_start().starts_with("docker ps"),
        "model just re-ran the failing command without reading the error: {}",
        pred.cmd
    );
}


#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn google_provider_completes_next_cmd_via_react_loop() {
    init_test_tracing();

    let tmp = TempDir::new().expect("tempdir");
    let mut memory = FolderMemory::new(tmp.path());
    let cwd = env::current_dir().expect("cwd");
    memory.register(&cwd).expect("register cwd");

    let provider = google_client();
    let mut runner = ReactLoop::new(provider);

    let prediction: NextCommand = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        workflow
            .run("git commit -m \" ")
            .await
            .unwrap_or_else(|e| panic!("workflow failed: {:?}", e.source()))
    };

    println!("cmd:   {}", prediction.cmd);
    println!("man:   {}", prediction.man);
    println!("scale: {:?}", prediction.scale);

    assert!(!prediction.cmd.is_empty(), "cmd empty");
    assert!(!prediction.man.is_empty(), "man empty");

    let conv = memory.current().expect("memory should be loaded");
    assert_eq!(conv.interactions.len(), 1, "interaction not persisted");

    drop(tmp);
}