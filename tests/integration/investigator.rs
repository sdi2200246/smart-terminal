use std::error::Error;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use smart_terminal::agent::agents::hooks::ToolsRegulator;
use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::agent::workflows::investigator::{Investigator, Plan, Report};
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

async fn run_case(label: &str, question: &str) -> (Plan, Report) {
    init_test_tracing();

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider).with_hook(Box::new(ToolsRegulator::new()));
    let mut workflow = Investigator::new(&mut runner);

    let (plan, report) = workflow
        .run(question)
        .await
        .unwrap_or_else(|e| panic!("[{label}] workflow failed: {:?}", e.source()));

    println!("\n═══ {label} — PLAN ═══");
    println!("goal: {}", plan.goal);
    for (i, step) in plan.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step.action);
        println!("     why: {}", step.rationale);
    }

    println!("\n═══ {label} — REPORT ═══");
    println!("report: {}", report.report);

    // Shape checks every investigation must pass — keeps the per-test asserts focused on semantics.
    assert!(!plan.goal.is_empty(), "[{label}] plan.goal empty");
    assert!(!plan.steps.is_empty(), "[{label}] plan.steps empty");
    assert!(!report.report.is_empty(), "[{label}] report.summary empty");

    (plan, report)
}

// ── Case 1: broad project overview — planner must orient, executor must synthesize ──
#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn project_overview() {
    let question = "can you search google for any news about agents? or hacks? ";

    let (_plan, report) = run_case("overview", question).await;
}
