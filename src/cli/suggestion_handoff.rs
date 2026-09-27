use std::io::{self, Write};
use std::path::Path;

use crate::agent::workflows::next_cmd::NextCommand;

const HANDOFF_FILE: &str = "investigator-command";
const OUTPUT_PROTOCOL: &str = "SMART_TERMINAL_NEXT_CMD_V1";

pub fn write_recommendation(directory: &Path, command: &NextCommand) -> io::Result<()> {
    if command.cmd.trim().is_empty() || command.man.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "investigator recommendation must contain a non-empty command and description",
        ));
    }

    let mut handoff = tempfile::NamedTempFile::new_in(directory)?;
    writeln!(
        handoff,
        "{}\n{}\n{:?}\n{}",
        OUTPUT_PROTOCOL,
        command.man.lines().collect::<Vec<_>>().join(" "),
        command.scale,
        command.cmd
    )?;
    handoff
        .persist(directory.join(HANDOFF_FILE))
        .map(|_| ())
        .map_err(|error| error.error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::workflows::next_cmd::Reversibility;

    fn command(cmd: &str, man: &str) -> NextCommand {
        NextCommand {
            cmd: cmd.to_string(),
            man: man.to_string(),
            scale: Reversibility::Full,
        }
    }

    #[test]
    fn writes_command_fields_to_one_time_handoff() {
        let directory = tempfile::tempdir().unwrap();
        let recommendation = command(
            "git status --short",
            "Show the remaining working-tree changes.",
        );

        write_recommendation(directory.path(), &recommendation).unwrap();

        let handoff = std::fs::read_to_string(directory.path().join(HANDOFF_FILE)).unwrap();
        assert_eq!(
            handoff,
            "SMART_TERMINAL_NEXT_CMD_V1\nShow the remaining working-tree changes.\nFull\ngit status --short\n"
        );
    }

    #[test]
    fn replaces_an_unconsumed_recommendation_atomically() {
        let directory = tempfile::tempdir().unwrap();

        write_recommendation(
            directory.path(),
            &command("pwd", "Show the current directory."),
        )
        .unwrap();
        write_recommendation(
            directory.path(),
            &command("git status --short", "Show working-tree changes."),
        )
        .unwrap();

        let handoff = std::fs::read_to_string(directory.path().join(HANDOFF_FILE)).unwrap();
        assert_eq!(
            handoff,
            "SMART_TERMINAL_NEXT_CMD_V1\nShow working-tree changes.\nFull\ngit status --short\n"
        );
    }

    #[test]
    fn supports_multiline_recommendation_commands() {
        let directory = tempfile::tempdir().unwrap();

        write_recommendation(
            directory.path(),
            &command("cat <<EOF\nhello\nEOF", "Show a result."),
        )
        .unwrap();

        let handoff = std::fs::read_to_string(directory.path().join(HANDOFF_FILE)).unwrap();
        assert_eq!(
            handoff,
            "SMART_TERMINAL_NEXT_CMD_V1\nShow a result.\nFull\ncat <<EOF\nhello\nEOF\n"
        );
    }

    #[test]
    fn rejects_empty_command_or_description() {
        let directory = tempfile::tempdir().unwrap();

        for recommendation in [command(" ", "Explain it."), command("pwd", " ")] {
            let error = write_recommendation(directory.path(), &recommendation).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        }
    }
}
