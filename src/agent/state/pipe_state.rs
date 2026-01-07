use crossterm::event::{KeyCode,KeyModifiers , KeyEvent};
use super::terminal::{TermState ,TerminalAction ,InputEvent,ContextUpdate,Context};
use super::terminal_state::TerminalState;
use super::cmd_line::CmdLineState;
pub struct PipeState;

impl TerminalState for PipeState{

    fn handle_input(&mut self, event: InputEvent ,_ctx:&Context, _cmdline: &mut CmdLineState,)->Vec<TerminalAction>{
        match event{
            InputEvent::User(key) => vec![TerminalAction::SendPty(key_to_bytes(key).unwrap())],

            _=> vec![TerminalAction::NOop]
        }   
    }
    fn handle_output(&mut self, bytes:&[u8])->Vec<TerminalAction>{
        let mut actions:Vec<TerminalAction> = Vec::new();
        let output = String::from_utf8_lossy(&bytes);

        actions.push(TerminalAction::Flush(bytes.to_vec()));
        output_interpreter(&output , &mut actions);
        return actions;
    }

}

fn key_to_bytes(event: KeyEvent) -> Option<Vec<u8>> {
    let KeyEvent { code, modifiers, .. } = event;

    Some(match (code, modifiers) {
        (KeyCode::Char(c), m) if m.contains(KeyModifiers::CONTROL) => {
            vec![(c as u8) & 0x1F]
        }
        (KeyCode::Char(c), _) => vec![c as u8],
        (KeyCode::Enter, _) => vec![b'\r'],
        (KeyCode::Backspace, _) => vec![0x7F],
        (KeyCode::Tab, _) => vec![b'\t'],
        (KeyCode::Esc, _) => vec![0x1B],
        (KeyCode::Up, _) => b"\x1b[A".to_vec(),
        (KeyCode::Down, _) => b"\x1b[B".to_vec(),
        (KeyCode::Left, _) => b"\x1b[D".to_vec(),
        (KeyCode::Right, _) => b"\x1b[C".to_vec(),

        _ => return None,
    })
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
                        history:None,
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
        assert!(update.history.is_none(), "history should not be updated");
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
      #[test]
    fn key_to_bytes_basic_mappings_test() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        // normal char
        let k = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_to_bytes(k), Some(vec![b'a']));

        // ctrl char (Ctrl+C)
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_to_bytes(k), Some(vec![3]));

        // enter
        let k = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_to_bytes(k), Some(vec![b'\r']));

        // backspace
        let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(key_to_bytes(k), Some(vec![0x7F]));

        // arrow key
        let k = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_to_bytes(k), Some(b"\x1b[A".to_vec()));
    }

}
