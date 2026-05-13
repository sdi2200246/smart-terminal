use std::error::Error;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use smart_terminal::agent::agents::Agent;
use smart_terminal::agent::archtectures::react::ReactLoop;
use smart_terminal::core::session::{Model, ModelName};
use smart_terminal::groq::client::GroqClient;
use smart_terminal::utils::FlatSchema;

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

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
struct PlanStep {
    pub action: String,
    pub rationale: String,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}
impl FlatSchema for Plan {}

#[derive(JsonSchema, Deserialize, Debug)]
#[schemars(deny_unknown_fields)]
struct Report {
    pub summary: String,
    pub findings: Vec<String>,
    pub gaps: Vec<String>,
}
impl FlatSchema for Report {}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn plan_then_investigate() {
    init_test_tracing();

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);

    let question = "Give a review of the project for a senior dev to evaluate the autho and to show the htinking of the programmer strenghts and weeknesses";


    // ── Phase 1 — planner ───────────────────────────────────────────────
    let plan = {
        let mut planner = Agent::planner(
            &mut runner,
            Model::with_default_temp(ModelName::GptOss120B),
        );
        let user = format!("Question\n: {}", question);

        planner
            .run::<Plan>(user)
            .await
            .unwrap_or_else(|e| panic!("planner failed: {:?}", e.source()))
    };

    println!("\n═══ PLAN ═══");
    println!("goal: {}", plan.goal);
    for (i, step) in plan.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step.action);
        println!("     why: {}", step.rationale);
    }
    assert!(!plan.goal.is_empty(), "plan goal should not be empty");
    assert!(!plan.steps.is_empty(), "plan should have at least one step");

    // ── Phase 2 — investigator ──────────────────────────────────────────
    let report = {
        let mut executor = Agent::executor(
            &mut runner,
            Model::with_default_temp(ModelName::GptOss120B),
        );
        let plan_json = serde_json::to_string_pretty(&plan).expect("plan serializes");
        let user = format!(
            "Question: {}\n\nInvestigation plan:\n{}",
            question, plan_json
        );

        executor
            .run::<Report>(user)
            .await
            .unwrap_or_else(|e| panic!("investigator failed: {:?}", e.source()))
    };

    println!("\n═══ REPORT ═══");
    println!("summary: {}", report.summary);
    println!("\nfindings:");
    for f in &report.findings {
        println!("  • {}", f);
    }
    if !report.gaps.is_empty() {
        println!("\ngaps:");
        for g in &report.gaps {
            println!("  • {}", g);
        }
    }
    assert!(!report.summary.is_empty(), "report summary should not be empty");
    assert!(!report.findings.is_empty(), "report should have at least one finding");
}