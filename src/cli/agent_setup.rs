use crate::agent::workflows::investigator::InvestigatorToolFactory;
use crate::agent::workflows::next_cmd::NextCmdToolFactory;
use crate::core::capability::Capability;

use crate::tools::bash::Bash;
use crate::tools::docker::Docker;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::last_error::ReadLastError;
use crate::tools::read_dir::ReadDir;
use crate::tools::read_file::ReadFile;

pub struct CliToolProvider;

impl InvestigatorToolFactory for CliToolProvider {
    fn planner_tools(&self) -> Vec<Box<dyn Capability>> {
        vec![Box::new(ReadDir)]
    }

    fn executor_tools(&self) -> Vec<Box<dyn Capability>> {
        vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)]
    }
}

impl NextCmdToolFactory for CliToolProvider {
    fn cmd_predictor_tools(&self) -> Vec<Box<dyn Capability>> {
        vec![
            Box::new(GitDiffStaged),
            Box::new(Docker),
            Box::new(ReadLastError),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::{Agent, AgentBuilder};
    use crate::agent::workflows::investigator::{Plan, Report};
    use crate::agent::workflows::next_cmd::NextCommand;
    use crate::core::model::{Model, ModelName};
    use crate::core::session::ConversationEvent;
    use crate::utils::FlatSchema;

    fn print_complete_prompt(name: &str, agent: Agent) {
        let session = agent.build_session("Prompt preview");
        println!("\n========== {name} ==========");
        for event in session.events() {
            if let ConversationEvent::System(message) = event {
                println!("{message}");
            }
        }
    }

    #[test]
    fn print_complete_active_role_prompts_for_inspection() {
        let provider = CliToolProvider;
        let planner = AgentBuilder::planner(
            Model::with_default_temp(ModelName::GptOss120B),
            provider.planner_tools(),
            Plan::schema(),
        )
        .build();
        let executor = AgentBuilder::executor(
            Model::with_default_temp(ModelName::GptOss120B),
            provider.executor_tools(),
            Report::schema(),
        )
        .build();
        let predictor = AgentBuilder::cmd_predictor(
            Model::with_default_temp(ModelName::GptOss120B),
            provider.cmd_predictor_tools(),
            NextCommand::schema(),
        )
        .build();

        for (agent, schema_name) in [
            (&planner, "Plan"),
            (&executor, "Report"),
            (&predictor, "NextCommand"),
        ] {
            assert!(agent.system_prompt.contains("[FINAL-ANSWER SCHEMA]"));
            assert!(
                agent
                    .system_prompt
                    .contains(&format!("\"title\": \"{schema_name}\""))
            );
            assert!(agent.system_prompt.contains("[AVAILABLE TOOLS]"));
        }
        assert!(executor.system_prompt.contains("`update_scratchpad`"));
        assert!(executor.system_prompt.contains("[AVAILABLE-TOOL GUIDANCE]"));
        assert!(!planner.system_prompt.contains("`update_scratchpad`"));
        assert!(!predictor.system_prompt.contains("`update_scratchpad`"));
        assert!(executor.system_prompt.contains("complete updated state"));
        for tool in [
            "git_diff_staged",
            "docker",
            "final_answer",
            "read_last_error",
        ] {
            assert!(
                predictor.system_prompt.contains(&format!("`{tool}`")),
                "predictor prompt is missing the registered {tool} tool"
            );
        }
        assert!(predictor.system_prompt.contains("staged changes"));
        assert!(predictor.system_prompt.contains("captured stderr"));

        print_complete_prompt("PLANNER", planner);
        print_complete_prompt("EXECUTOR", executor);
        print_complete_prompt("COMMAND PREDICTOR", predictor);
    }

    #[test]
    fn executor_prompt_requires_scratchpad_updates_before_final_answer() {
        let provider = CliToolProvider;
        let executor = AgentBuilder::executor(
            Model::with_default_temp(ModelName::GptOss120B),
            provider.executor_tools(),
            Report::schema(),
        )
        .build();

        assert!(executor.action_space.get("update_scratchpad").is_some());
        assert!(
            executor
                .system_prompt
                .contains("Before calling `final_answer`, you MUST call `update_scratchpad`")
        );
        assert!(executor.system_prompt.contains("complete updated state"));
        assert!(
            !executor
                .system_prompt
                .contains("runtime rejects a final answer")
        );
    }
}
