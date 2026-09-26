use crate::agent::patterns::react::ReactLoop;
use crate::agent::workflows::investigator::Investigator;
use crate::cli::agent_setup::CliToolProvider;
use crate::cli::cli::InvestigateArgs;
use crate::cli::presenters::Presenter;
use crate::providers::google::client::GoogleClient;
use std::error::Error;

pub async fn run(args: InvestigateArgs) {
    let (presenter, tx) = Presenter::new();
    let handle = presenter.spawn();

    let provider = GoogleClient::pooled();
    let outcome = {
        let runner = ReactLoop::new(provider);
        let mut workflow =
            Investigator::new(runner, CliToolProvider).with_events_streaming(tx.clone());
        tokio::select! {
            result = workflow.run(args.question) => Ok(Some(result)),
            signal = tokio::signal::ctrl_c() => signal.map(|()| None),
        }
    };
    drop(tx);

    if let Err(error) = handle.await {
        eprintln!("Presenter task failed: {error}");
    }

    match outcome {
        Ok(Some(Ok((plan, report)))) => {
            println!("─── Plan ───");
            println!("Goal: {}\n", plan.goal);
            for (i, step) in plan.steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step.action);
                println!("     {}", step.rationale);
            }

            println!("\n─── Report ───");
            println!("{}", report.report);
        }
        Ok(Some(Err(e))) => {
            println!("Investigation failled {:?}", e.source());
            std::process::exit(1);
        }
        Ok(None) => {
            println!("Investigation cancelled");
            std::process::exit(130);
        }
        Err(error) => {
            eprintln!("Failed to listen for Ctrl+C: {error}");
            std::process::exit(1);
        }
    }
}
