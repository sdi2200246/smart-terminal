use std::env;
use std::io;
use std::path::PathBuf;

use crate::agent::error::AgentError;
use crate::agent::patterns::react::ReactLoop;
use crate::agent::workflows::investigator::{Investigator, Plan, Report};
use crate::cli::presenters::Presenter;
use crate::cli::suggestion_handoff::write_recommendation;
use crate::cli::{
    agent_setup::CliToolProvider,
    cli::{InvestigateAction, InvestigateArgs, InvestigationSessionAction},
};
use crate::core::memory::{InvestigationSession, InvestigationSessionStore};
use crate::persistence::{FileInvestigationSessionStore, ProjectRootResolver};
use crate::providers::google::client::GoogleClient;
use tokio::sync::mpsc::UnboundedSender;

enum RunError {
    Agent(AgentError),
    CtrlC(io::Error),
}

pub async fn run(args: InvestigateArgs) {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to read current directory: {error}");
            std::process::exit(1);
        }
    };
    let project_roots = ProjectRootResolver::project_local();
    let project_root = match project_roots.resolve_root(&cwd) {
        Ok(project_root) => project_root,
        Err(error) => {
            eprintln!("Failed to resolve investigation session root: {error}");
            std::process::exit(1);
        }
    };
    let project_key = project_root.to_string_lossy().into_owned();
    let store = FileInvestigationSessionStore::project_local();

    if let Some(InvestigateAction::Session(session_args)) = args.action {
        match session_args.action {
            InvestigationSessionAction::Clear => {
                if let Err(error) = store.load_for_project_root(&project_root, &project_roots) {
                    eprintln!("Failed to migrate investigation session: {error}");
                    std::process::exit(1);
                }
                if let Err(error) = store.clear(&project_key) {
                    eprintln!("Failed to clear investigation session: {error}");
                    std::process::exit(1);
                }
                println!("✓ cleared investigation session for {}", cwd.display());
            }
        }
        return;
    }

    let Some(question) = args.question else {
        eprintln!("Provide an investigation question, or use `investigate session clear`.");
        std::process::exit(2);
    };

    let snapshot = if args.one_off {
        InvestigationSession::default()
    } else {
        match load_snapshot(&store, &project_root, &project_roots) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    };
    let outcome = run_workflow(&question, snapshot).await;

    match outcome {
        Ok(Some((plan, report, updated_session))) => {
            if !args.one_off {
                if let Err(error) = store.save(&project_key, &updated_session) {
                    eprintln!("Failed to save investigation session: {error}");
                    std::process::exit(1);
                }
            }
            print_result(&plan, &report);
        }
        Ok(None) => {
            println!("Investigation cancelled");
            std::process::exit(130);
        }
        Err(RunError::Agent(error)) => {
            eprintln!("Investigation failed: {error:?}");
            std::process::exit(1);
        }
        Err(RunError::CtrlC(error)) => {
            eprintln!("Failed to listen for Ctrl+C: {error}");
            std::process::exit(1);
        }
    }
}

fn load_snapshot(
    store: &FileInvestigationSessionStore,
    project_root: &std::path::Path,
    resolver: &ProjectRootResolver,
) -> Result<InvestigationSession, String> {
    store
        .load_for_project_root(project_root, resolver)
        .map(|snapshot| snapshot.unwrap_or_default())
        .map_err(|error| format!("Failed to load investigation session: {error}"))
}

async fn run_workflow(
    question: &str,
    saved_session: InvestigationSession,
) -> Result<Option<(Plan, Report, InvestigationSession)>, RunError> {
    let (presenter, tx) = Presenter::new();
    let handle = presenter.spawn();
    let outcome = invoke_workflow(question, saved_session, tx.clone()).await;
    drop(tx);

    if let Err(error) = handle.await {
        eprintln!("Presenter task failed: {error}");
    }

    outcome
}

async fn invoke_workflow(
    question: &str,
    saved_session: InvestigationSession,
    tx: UnboundedSender<crate::agent::agents::AgentEvent>,
) -> Result<Option<(Plan, Report, InvestigationSession)>, RunError> {
    let provider = GoogleClient::pooled();
    let runner = ReactLoop::new(provider);
    let mut workflow = Investigator::new(runner, CliToolProvider).with_events_streaming(tx);
    tokio::select! {
        result = workflow.run_with_session(question, saved_session) => {
            result.map(Some).map_err(RunError::Agent)
        }
        signal = tokio::signal::ctrl_c() => {
            signal.map(|()| None).map_err(RunError::CtrlC)
        }
    }
}

fn print_result(plan: &Plan, report: &Report) {
    println!("─── Plan ───");
    println!("Goal: {}\n", plan.goal);
    for (index, step) in plan.steps.iter().enumerate() {
        println!("  {}. {}", index + 1, step.action);
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
        Ok(None) => print_recommendation(command),
        Err(error) => {
            eprintln!("Failed to hand recommendation to zsh: {error}");
            print_recommendation(command);
        }
    }
}

fn print_recommendation(command: &crate::agent::workflows::next_cmd::NextCommand) {
    println!("\n─── Suggested Command ───");
    println!("{}", command.cmd);
    println!("{} ({:?})", command.man, command.scale);
}
