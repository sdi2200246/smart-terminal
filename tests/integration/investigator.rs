use smart_terminal::agent::patterns::react::ReactLoop;
use smart_terminal::agent::workflows::investigator::{Investigator, Plan, Report};
use smart_terminal::core::llm_client::LLMProvider;
use smart_terminal::providers::google::client::GoogleClient;
use smart_terminal::providers::groq::client::GroqClient;
use super::TestToolProvider;
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

// Accepts any LLMProvider so we can inject Groq or Google dynamically
async fn run_case<P: LLMProvider+Clone>(label: &str, question: &str, provider: P) -> (Plan, Report) {
    init_test_tracing();
    dotenv::dotenv().ok();

    let runner = ReactLoop::new(provider);
    let mut workflow = Investigator::new(runner ,TestToolProvider);

    let (plan, report) = workflow
        .run(question)
        .await
        .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e));

    println!("\n═══ {label} — PLAN ═══");
    println!("goal: {}", plan.goal);
    for (i, step) in plan.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step.action);
        println!("     why: {}", step.rationale);
    }

    println!("\n═══ {label} — REPORT ═══");
    println!("report: {}", report.report);

    assert!(!plan.goal.is_empty(), "[{label}] plan.goal empty");
    assert!(!plan.steps.is_empty(), "[{label}] plan.steps empty");
    assert!(!report.report.is_empty(), "[{label}] report.summary empty");

    (plan, report)
}

#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn project_overview_google() {
    let question = "i want you to check weather the factory for tools is a defenseble decision ";
    let provider = GoogleClient::pooled();
    let (_plan, _report) = run_case("overview_google", question, provider).await;
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn project_overview_groq() {
    let question = "read project structure and explain the main features of the porject";
    let provider = GroqClient::pooled();
    let (_plan, _report) = run_case("overview_groq", question, provider).await;
}