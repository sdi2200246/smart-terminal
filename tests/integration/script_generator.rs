use smart_terminal::agent::archtectures::oneshot::OneShot;
use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::agent::workflows::script_gen::{Script, ScriptDesign, ScriptGenerator};
use smart_terminal::groq::client::GroqClient;
use std::error::Error;
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

async fn run_case(label: &str, prompt: &str) -> (ScriptDesign, Script) {
    init_test_tracing();

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider.clone());
    let mut one_shooter = OneShot::new(provider);
    let mut workflow = ScriptGenerator::new(&mut runner, &mut one_shooter);

    let (design, script) = workflow
        .run(prompt)
        .await
        .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e.source()));

    println!("\n═══ {label} — DESIGN ═══");
    println!("shell:          {:?}", design.shell);
    println!("purpose:        {:?}", design.purpose);
    println!("error_handling: {:?}", design.error_handling);
    println!("idempotent:     {:?}", design.idempotent);
    println!("arguments:");
    for a in &design.arguments {
        println!(
            "  • {} ({:?}, required={}) — {}",
            a.name, a.kind, a.required, a.help
        );
    }
    println!("dependencies:");
    for d in &design.dependencies {
        println!("  • {}", d);
    }
    println!("side_effects:");
    for s in &design.side_effects {
        println!("  • {}", s);
    }

    println!("evidence:");
    for s in &design.coding_decisions {
        println!("  • {:?}", s);
    }

    println!("\n═══ {label} — SCRIPT ═══");
    println!("filename:   {}", script.filename);
    println!("invocation: {}", script.invocation_example);
    println!("\n--- {} ---", script.filename);
    println!("{}", script.content);

    // Shape checks every script must pass — keeps the per-test asserts focused on semantics.
    assert!(!design.purpose.is_empty(), "[{label}] design.purpose empty");
    assert!(
        !script.filename.is_empty(),
        "[{label}] script.filename empty"
    );
    assert!(!script.content.is_empty(), "[{label}] script.content empty");
    assert!(
        script.content.starts_with("#!"),
        "[{label}] script must start with a shebang"
    );

    (design, script)
}

// ── Case 1: pure design, no code context ────────────────────────────────
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn timestamped_directory_backup() {
    let prompt = "Create a script that takes user voice input and prints what they said";

    let (design, script) = run_case("backup", prompt).await;
}

// ── Case 2: code-aware — architect should probe the repo ────────────────
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn rust_project_quality_gate() {
    let prompt = "Create a script for this project that runs cargo check, cargo test, \
                  and cargo clippy in sequence. Stop on the first failure and exit \
                  non-zero. Print which step failed.";

    let (design, script) = run_case("quality_gate", prompt).await;

    assert!(
        design.dependencies.iter().any(|d| d.contains("cargo")),
        "design should declare cargo as a dependency"
    );
    assert!(script.content.contains("cargo"), "script should call cargo");
}

// ── Case 3: argument handling, defaults, no side effects ────────────────
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn find_largest_files() {
    let prompt = "give me a script for setting this project up for users not devs";

    let (design, _) = run_case("largest_files", prompt).await;

    assert!(
        !design.arguments.is_empty(),
        "script takes a directory argument — should have at least one"
    );
    assert!(
        design.idempotent,
        "read-only file enumeration should be idempotent"
    );
}
