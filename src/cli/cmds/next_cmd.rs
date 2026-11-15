use crate::agent::contracts::{NextCommand, Reversibility};
use crate::agent::patterns::react::ReactLoop;
use crate::agent::patterns::tool_engine::ToolPermissionMode;
use crate::agent::workflows::next_cmd::NextCmd;
use crate::cli::agent_setup::CliToolProvider;
use crate::cli::cli::NextCmdArgs;
use crate::cli::presenters::Presenter;
use crate::core::memory::Memory;
use crate::persistence::NextCmdMemory;
use crate::providers::groq::client::GroqClient;
use std::env;

pub async fn run(args: NextCmdArgs) {
    let mut memory = NextCmdMemory::project_local();

    if let Ok(cwd) = env::current_dir() {
        let _ = memory.load(&cwd);
    }

    let provider = GroqClient::pooled();
    let runner = ReactLoop::new_with_policy(provider, ToolPermissionMode::Allow.to_policy());

    let prediction = {
        let mut workflow = NextCmd::new(runner, &mut memory, CliToolProvider);
        match workflow.run(args.buffer).await {
            Ok(p) => p,
            Err(e) => {
                let prediction = NextCommand {
                    cmd: "< agent failed >".to_string(),
                    man: e.to_string(),
                    scale: Reversibility::Irreversible,
                };
                let _ = Presenter::render_prediction(&prediction);
                return;
            }
        }
    };
    if let Err(error) = Presenter::render_prediction(&prediction) {
        Presenter::render_error(format!("Failed to write command suggestion: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_multiline_command_after_metadata() {
        let prediction = NextCommand {
            cmd: "cat <<'EOF'\nhello\nEOF".to_string(),
            man: "Write a small sample\nfile.".to_string(),
            scale: Reversibility::Partial,
        };
        let mut output = Vec::new();

        Presenter::write_prediction(&mut output, &prediction).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "SMART_TERMINAL_NEXT_CMD_V1\nWrite a small sample file.\nPartial\ncat <<'EOF'\nhello\nEOF\n"
        );
    }

    #[test]
    fn writes_agent_error_as_red_comment_metadata() {
        let prediction = NextCommand {
            cmd: "< agent failed >".to_string(),
            man: "Domain error".to_string(),
            scale: Reversibility::Irreversible,
        };
        let mut output = Vec::new();

        Presenter::write_prediction(&mut output, &prediction).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "SMART_TERMINAL_NEXT_CMD_V1\nDomain error\nIrreversible\n< agent failed >\n"
        );
    }
}
