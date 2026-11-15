use crate::agent::agents::{AgentEvent, AgentState};
use crate::agent::contracts::{NextCommand, Report};
use crate::agent::patterns::tool_engine::PermissionDecision;
use crate::core::memory::Interaction;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub mod render;

const SPINNER_TICK: Duration = Duration::from_millis(80);
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "];

pub struct Presenter {
    rx: UnboundedReceiver<AgentEvent>,
}

impl Presenter {
    pub fn new() -> (Self, UnboundedSender<AgentEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { rx }, tx)
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let spinner = ProgressBar::new_spinner();
            spinner.set_draw_target(ProgressDrawTarget::stdout());
            spinner.set_style(
                ProgressStyle::with_template("  {spinner:.cyan} {msg}")
                    .expect("valid spinner template")
                    .tick_strings(SPINNER_FRAMES),
            );
            spinner.set_message(state_message(AgentState::Thinking));
            spinner.enable_steady_tick(SPINNER_TICK);

            while let Some(event) = self.rx.recv().await {
                match event {
                    AgentEvent::StateUpdate(state) => {
                        spinner.set_message(state_message(state));
                    }
                    AgentEvent::ToolCall(call) => {
                        let line = render::format_call(&call);
                        spinner.suspend(|| println!("{line}"));
                    }
                    AgentEvent::ToolBackgrounded { tool, job_id } => {
                        let line = render::format_backgrounded(&tool, &job_id);
                        spinner.suspend(|| println!("{line}"));
                    }
                    AgentEvent::PlanReady(plan) => {
                        let panel = render::format_plan(&plan);
                        spinner.suspend(|| {
                            println!();
                            println!("{panel}");
                            println!();
                        });
                    }
                    AgentEvent::ToolApprovalRequest { call, responder } => {
                        let panel = render::format_approval_request(&call);
                        let decision = spinner.suspend(|| {
                            println!();
                            println!("{panel}");
                            prompt_for_approval(&call)
                        });
                        let _ = responder.send(decision);
                    }
                }
            }

            spinner.finish_and_clear();
        })
    }

    /// The plan is rendered live via [`AgentEvent::PlanReady`]; this renders what remains.
    pub fn render_investigation_result(report: &Report, show_recommendation: bool) {
        println!();
        println!("{}", render::format_report(report));
        if show_recommendation {
            println!();
            println!("{}", render::format_recommendation(&report.recommended_command));
        }
        println!();
    }

    pub fn render_session_cleared(cwd: &Path) {
        println!(
            "{}",
            render::format_success(
                "cleared investigation session for",
                &cwd.display().to_string()
            )
        );
    }

    pub fn render_cancelled(message: &str) {
        println!("{}", render::format_info(message));
    }

    pub fn render_memory_registered(cwd: &Path) {
        println!(
            "{}",
            render::format_success("registered", &cwd.display().to_string())
        );
    }

    pub fn render_memory_deleted(cwd: &Path) {
        println!(
            "{}",
            render::format_success("deleted", &format!("memory for {}", cwd.display()))
        );
    }

    pub fn render_memory_cleared(cwd: &Path) {
        println!(
            "{}",
            render::format_success("cleared", &format!("interactions for {}", cwd.display()))
        );
    }

    pub fn render_memory_unregistered() {
        println!(
            "{}",
            render::format_info("not registered — run `memory init` first")
        );
    }

    pub fn render_memory_interactions(cwd: &Path, interactions: &[Interaction]) {
        if interactions.is_empty() {
            println!(
                "{}",
                render::format_info(&format!("no interactions yet for {}", cwd.display()))
            );
            return;
        }
        println!("{}", render::format_memory_interactions(cwd, interactions));
    }

    pub fn render_prediction(prediction: &NextCommand) -> io::Result<()> {
        let stdout = io::stdout();
        let mut output = stdout.lock();
        Self::write_prediction(&mut output, prediction)?;
        output.flush()
    }

    pub fn write_prediction(output: &mut impl Write, prediction: &NextCommand) -> io::Result<()> {
        render::write_prediction(output, prediction)
    }

    pub fn render_error(message: impl AsRef<str>) {
        eprintln!("{}", render::format_error(message.as_ref()));
    }
}

fn state_message(state: AgentState) -> String {
    let detail = match state {
        AgentState::Thinking => "waiting for model response",
        AgentState::ExecutingTool => "running tools",
        AgentState::StructuringOutput => "formatting final answer",
        AgentState::Completed => "done",
        AgentState::Failed => "agent failed",
    };
    format!("{}  {}", state.label().bold(), detail.dimmed())
}

fn read_line() -> Option<String> {
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    Some(input.trim().to_string())
}

fn prompt_for_approval(call: &crate::core::responce::AgentToolCall) -> PermissionDecision {
    println!();
    print!("{}", render::format_prompt(&format!("Allow {}? [y/N]", call.name())));

    let Some(response) = read_line() else {
        return PermissionDecision::Denied(format!(
            "Tool '{}' requires approval but the approval input could not be read.",
            call.name()
        ));
    };

    if matches!(response.to_ascii_lowercase().as_str(), "y" | "yes") {
        println!("  {}", render::format_success("approved", ""));
        return PermissionDecision::Approved;
    }

    print!("{}", render::format_prompt("Reason (optional):"));
    let reason = read_line()
        .filter(|reason| !reason.is_empty())
        .unwrap_or_else(|| "no reason provided".to_string());

    println!("  {}", render::format_error("denied"));
    PermissionDecision::Denied(format!(
        "User denied approval for tool '{}': {}",
        call.name(),
        reason
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::contracts::{Plan, PlanStep, Reversibility};

    #[test]
    fn backgrounded_tool_line_shows_tool_and_job_id() {
        let line = render::format_backgrounded("bash", "job-42");
        assert!(line.contains("bash"));
        assert!(line.contains("job-42"));
        assert!(line.contains("background"));
    }

    #[test]
    fn state_message_shows_label_and_detail() {
        let message = state_message(AgentState::ExecutingTool);
        assert!(message.contains("Executing tool"));
        assert!(message.contains("running tools"));
    }

    #[test]
    fn formats_plan_report_and_recommendation_as_filled_panels() {
        let plan = Plan {
            goal: "Verify repository status".into(),
            steps: vec![PlanStep {
                action: "Run git status".into(),
                rationale: "Check working tree".into(),
            }],
        };
        let report = Report {
            report: "The working tree is clean.".into(),
            recommended_command: NextCommand {
                cmd: "git status --short".into(),
                man: "Inspect working tree.".into(),
                scale: Reversibility::Full,
            },
        };

        let rendered_plan = render::format_plan(&plan);
        assert!(rendered_plan.contains("PLAN"));
        assert!(rendered_plan.contains("Verify repository status"));
        assert!(rendered_plan.contains("Run git status"));
        assert!(rendered_plan.contains("▎"));
        assert!(rendered_plan.contains("48;2;"), "panel should have a filled background");

        let rendered_report = render::format_report(&report);
        assert!(rendered_report.contains("REPORT"));
        assert!(rendered_report.contains("The working tree is clean."));

        let rendered_cmd = render::format_recommendation(&report.recommended_command);
        assert!(rendered_cmd.contains("NEXT"));
        assert!(rendered_cmd.contains("git status --short"));
        assert!(rendered_cmd.contains("Full reversibility"));
    }
}
