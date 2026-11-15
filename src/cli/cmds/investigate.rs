use std::env;
use std::io;
use std::path::PathBuf;

use crate::agent::contracts::{Plan, Report};
use crate::agent::error::AgentError;
use crate::agent::patterns::react::ReactLoop;
use crate::agent::patterns::tool_engine::ToolPermissionMode;
use crate::agent::workflows::investigator::Investigator;
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
            Presenter::render_error(format!("Failed to read current directory: {error}"));
            std::process::exit(1);
        }
    };
    let project_roots = ProjectRootResolver::project_local();
    let project_root = match project_roots.resolve_root(&cwd) {
        Ok(project_root) => project_root,
        Err(error) => {
            Presenter::render_error(format!(
                "Failed to resolve investigation session root: {error}"
            ));
            std::process::exit(1);
        }
    };
    let project_key = project_root.to_string_lossy().into_owned();
    let store = FileInvestigationSessionStore::project_local();

    if let Some(InvestigateAction::Session(session_args)) = args.action {
        match session_args.action {
            InvestigationSessionAction::Clear => {
                if let Err(error) = store.load_for_project_root(&project_root, &project_roots) {
                    Presenter::render_error(format!(
                        "Failed to migrate investigation session: {error}"
                    ));
                    std::process::exit(1);
                }
                if let Err(error) = store.clear(&project_key) {
                    Presenter::render_error(format!(
                        "Failed to clear investigation session: {error}"
                    ));
                    std::process::exit(1);
                }
                Presenter::render_session_cleared(&cwd);
            }
        }
        return;
    }

    let Some(question) = args.question else {
        Presenter::render_error(
            "Provide an investigation question, or use `investigate session clear`.",
        );
        std::process::exit(2);
    };

    let snapshot = if args.one_off {
        InvestigationSession::default()
    } else {
        match load_snapshot(&store, &project_root, &project_roots) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                Presenter::render_error(error);
                std::process::exit(1);
            }
        }
    };
    let outcome = run_workflow(&question, snapshot, args.tool_permission.into()).await;

    match outcome {
        Ok(Some((_plan, report, updated_session))) => {
            if !args.one_off {
                if let Err(error) = store.save(&project_key, &updated_session) {
                    Presenter::render_error(format!(
                        "Failed to save investigation session: {error}"
                    ));
                    std::process::exit(1);
                }
            }
            present_result(&report);
        }
        Ok(None) => {
            Presenter::render_cancelled("Investigation cancelled");
            std::process::exit(130);
        }
        Err(RunError::Agent(error)) => {
            Presenter::render_error(format!("Investigation failed: {error:?}"));
            std::process::exit(1);
        }
        Err(RunError::CtrlC(error)) => {
            Presenter::render_error(format!("Failed to listen for Ctrl+C: {error}"));
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
    permission_mode: ToolPermissionMode,
) -> Result<Option<(Plan, Report, InvestigationSession)>, RunError> {
    let (presenter, tx) = Presenter::new();
    let handle = presenter.spawn();
    let outcome = invoke_workflow(question, saved_session, tx.clone(), permission_mode).await;
    drop(tx);

    if let Err(error) = handle.await {
        Presenter::render_error(format!("Presenter task failed: {error}"));
    }

    outcome
}

async fn invoke_workflow(
    question: &str,
    saved_session: InvestigationSession,
    tx: UnboundedSender<crate::agent::agents::AgentEvent>,
    permission_mode: ToolPermissionMode,
) -> Result<Option<(Plan, Report, InvestigationSession)>, RunError> {
    let provider = GoogleClient::pooled();
    let runner = ReactLoop::new_with_policy(provider, permission_mode.to_policy());
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

fn present_result(report: &Report) {
    let command = &report.recommended_command;
    let handed_to_shell = env::var_os("SMART_TERMINAL_SUGGESTION_DIR")
        .map(|directory| write_recommendation(&PathBuf::from(directory), command))
        .transpose();
    let show_recommendation = match handed_to_shell {
        Ok(Some(())) => false,
        Ok(None) => true,
        Err(error) => {
            Presenter::render_error(format!("Failed to hand recommendation to zsh: {error}"));
            true
        }
    };
    Presenter::render_investigation_result(report, show_recommendation);
}
