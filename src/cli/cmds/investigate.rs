use crate::agent::patterns::react::ReactLoop;
use crate::agent::workflows::investigator::Investigator;
use crate::cli::agent_setup::CliToolProvider;
use crate::cli::cli::InvestigateArgs;
use crate::cli::presenters::Presenter;
use crate::cli::suggestion_handoff::write_recommendation;
use crate::providers::google::client::GoogleClient;
use std::env;
use std::error::Error;
use std::path::PathBuf;

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
            let command = &report.recommended_command;
            let handed_to_shell = env::var_os("SMART_TERMINAL_SUGGESTION_DIR")
                .map(|directory| write_recommendation(&PathBuf::from(directory), command))
                .transpose();
            match handed_to_shell {
                Ok(Some(())) => {}
                Ok(None) => {
                    print_recommendation(command);
                }
                Err(error) => {
                    eprintln!("Failed to hand recommendation to zsh: {error}");
                    print_recommendation(command);
                }
            }
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

    fn print_recommendation(command: &crate::agent::workflows::next_cmd::NextCommand) {
        println!("\n─── Suggested Command ───");
        println!("{}", command.cmd);
        println!("{} ({:?})", command.man, command.scale);
    }
}
