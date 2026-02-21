use crate::emulator::state::terminal_state::UseCase;
use super::terminal::{TermState ,TerminalAction,ContextUpdate,Context};
use super::terminal_state::TerminalState;
use super::cmd_line::CmdLineState;
use std::fs::OpenOptions;
use std::io::Write;
pub struct PipeState;

impl TerminalState for PipeState{

    fn handle_input(&mut self, event:UseCase ,_ctx:&Context, _cmdline: &mut CmdLineState,)->Vec<TerminalAction>{
        match event{
            UseCase::Passthrough(bytes) => vec![TerminalAction::SendPty(bytes)],
            _=> vec![TerminalAction::NOop]
        }   
    }
    fn handle_output(&mut self, bytes:&[u8])->Vec<TerminalAction>{
        let mut actions:Vec<TerminalAction> = Vec::new();

        // if let Ok(mut file) = OpenOptions::new()
        //     .create(true)
        //     .append(true)
        //     .open("pty_dump.bin")
        // {
        //     let _ = file.write_all(bytes);
        //     let _ = file.flush();
        // }

        let output = String::from_utf8_lossy(&bytes);

        actions.push(TerminalAction::Flush(bytes.to_vec()));
        output_interpreter(&output , &mut actions);
        return actions;
    }
    fn state(&mut self)->TermState {
        TermState::Pipe
    }
}

fn extract_cwd(output: &str) -> Option<String> {
        output
            .lines()
            .find_map(|line| line.strip_prefix("__AGENT_DONE__:"))
            .map(|cwd| cwd.to_string())
    }

fn output_interpreter(output:&str , actions:&mut Vec<TerminalAction>){
    if output.contains("\x1b[?1049l") || output.contains("\x1b[?47l") || output.contains("\x1b[?1047l"){
        actions.push(TerminalAction::SwitchState(TermState::Cmdline));
    }

    if output.contains("__AGENT_DONE__") {
        if let Some(cwd) = extract_cwd(output) {
            let new_context = ContextUpdate{
                cwd:Some(cwd),
                cmd:None,
                files:None,
            };
            actions.push(TerminalAction::UpdateContext(new_context));
        }
        actions.push(TerminalAction::SwitchState(TermState::Cmdline));
        actions.push(TerminalAction::Render);
    }
}


#[cfg(test)]
mod pipe_state_tests {
    use super::*;

    #[test]
    fn handle_output_always_flushes_bytes() {
        let mut state = PipeState;
        let bytes = b"hello world";
        let actions = state.handle_output(bytes);

        assert!(
            actions.iter().any(|a| matches!(
                a,
                TerminalAction::Flush(b) if b == bytes
            )),
            "Expected Flush action with original bytes"
        );
    }

    #[test]
    fn handle_output_detects_agent_done() {
        let mut state = PipeState;

        let bytes = b"__AGENT_DONE__:/home/test\n";
        let actions = state.handle_output(bytes);

        assert!(
            actions.iter().any(|a| matches!(
                a,
                TerminalAction::UpdateContext(_)
            )),
            "Expected UpdateContext"
        );

        assert!(
            actions.iter().any(|a| matches!(
                a,
                TerminalAction::SwitchState(TermState::Cmdline)
            )),
            "Expected SwitchState(Cmdline)"
        );
    }
    
    #[test]
    fn agent_done_produces_correct_context_update() {
        let mut actions = Vec::new();
        let output = "__AGENT_DONE__:/home/user";

        output_interpreter(output, &mut actions);

        let update = actions.into_iter().find_map(|action| {
            if let TerminalAction::UpdateContext(update) = action {
                Some(update)
            } else {
                None
            }
        }).expect("Expected UpdateContext action");

        assert_eq!(update.cwd.as_deref(), Some("/home/user"));
        assert!(update.cmd.is_none(), "history should not be updated");
        assert!(update.files.is_none(), "files should not be updated");
    }

    fn has_switch(actions: &[TerminalAction], state: TermState) -> bool {
        actions.iter().any(|a| matches!(
            a,
            TerminalAction::SwitchState(s) if *s == state
        ))
}

    #[test]
    fn pipe_mode_alt_screen_exit_switches_to_cmdline() {
        let exit_codes = ["\x1b[?1049l", "\x1b[?47l", "\x1b[?1047l"];

        for code in exit_codes {
            let mut actions = Vec::new();

            output_interpreter(code, &mut actions);

            assert!(
                has_switch(&actions, TermState::Cmdline),
                "Expected SwitchState(Cmdline) for exit code {:?}",
                code
            );
        }
    }
}
