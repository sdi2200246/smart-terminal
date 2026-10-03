use crate::agent::patterns::react::ReactLoop;
use crate::agent::workflows::next_cmd::{NextCmd, NextCommand, Reversibility};
use crate::cli::agent_setup::CliToolProvider;
use crate::cli::cli::NextCmdArgs;
use crate::core::memory::Memory;
use crate::persistence::NextCmdMemory;
use crate::providers::groq::client::GroqClient;
use std::env;
use std::io::{self, Write};

const OUTPUT_PROTOCOL: &str = "SMART_TERMINAL_NEXT_CMD_V1";

pub async fn run(args: NextCmdArgs) {
    let mut memory =
        NextCmdMemory::project_local().unwrap_or_else(|_| NextCmdMemory::new(env::temp_dir()));

    if let Ok(cwd) = env::current_dir() {
        let _ = memory.load(&cwd);
    }

    let provider = GroqClient::pooled();
    let runner = ReactLoop::new(provider);

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
                let stdout = io::stdout();
                let mut output = stdout.lock();
                let _ = write_prediction(&mut output, &prediction);
                let _ = output.flush();
                return;
            }
        }
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    if let Err(error) = write_prediction(&mut output, &prediction) {
        eprintln!("Failed to write command suggestion: {error}");
    }
}

fn write_prediction(output: &mut impl Write, prediction: &NextCommand) -> io::Result<()> {
    // The command is the complete remaining payload after the versioned metadata header.
    writeln!(output, "{OUTPUT_PROTOCOL}")?;
    writeln!(
        output,
        "{}",
        prediction.man.lines().collect::<Vec<_>>().join(" ")
    )?;
    writeln!(output, "{:?}", prediction.scale)?;
    writeln!(output, "{}", prediction.cmd)
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

        write_prediction(&mut output, &prediction).unwrap();

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

        write_prediction(&mut output, &prediction).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "SMART_TERMINAL_NEXT_CMD_V1\nDomain error\nIrreversible\n< agent failed >\n"
        );
    }
}
