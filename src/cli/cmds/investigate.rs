use crate::agent::archtectures::react::ReactLoop;
use crate::agent::workflows::investigator::Investigator;
use crate::cli::cli::InvestigateArgs;
use crate::groq::client::GroqClient;

pub async fn run(args: InvestigateArgs) {
    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);
    let mut workflow = Investigator::new(&mut runner);

    match workflow.run(args.question).await {
        Ok((plan, report)) => {
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
            eprintln!("investigation failed: {e}");
            std::process::exit(1);
        }
    }
}