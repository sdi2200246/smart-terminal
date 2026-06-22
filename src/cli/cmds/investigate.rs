use std::error::Error;

use crate::agent::agents::hooks::ToolsRegulator;
use crate::agent::archtectures::react::ReactLoop;
use crate::agent::workflows::investigator::Investigator;
use crate::cli::cli::InvestigateArgs;
use crate::cli::presenters::Presenter;
use crate::groq::client::GroqClient;
pub async fn run(args: InvestigateArgs) {
    let (presenter, tx) = Presenter::new();
    let handle = presenter.spawn();

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider)
        .with_events_streaming(tx)
        .with_hook(Box::new(ToolsRegulator::new()));
    let mut workflow = Investigator::new(&mut runner);

    match workflow.run(args.question).await {
        Ok((plan, report)) => {
            drop(runner);
            handle.await.ok();
            println!("─── Plan ───");
            println!("Goal: {}\n", plan.goal);
            for (i, step) in plan.steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step.action);
                println!("     {}", step.rationale);
            }

            println!("\n─── Report ───");
            println!("{}", report.report);
        }
        Err(e) => {
            println!("Investigation failled {:?}", e.source());
            std::process::exit(1);
        }
    }
}
