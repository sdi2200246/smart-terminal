use crossterm::event::{KeyCode, KeyModifiers , KeyEvent};
use super::terminal::InputEvent;
use super::terminal_state::{UseCase , Key};
use super::terminal::TermState;

fn key_to_bytes(event: &KeyEvent) -> Option<Vec<u8>> {
    let KeyEvent { code, modifiers, .. } = event;

    Some(match (code, modifiers) {
        (KeyCode::Char(c), m) if m.contains(KeyModifiers::CONTROL) => {
            vec![(*c as u8) & 0x1F]
        }
        (KeyCode::Char(c), _) => vec![*c as u8],
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
pub struct InputInterprter;

impl InputInterprter{

    pub fn map(event: InputEvent , state:TermState) -> Option<UseCase> {

    match event {
            InputEvent::User(key) => {

            if state == TermState::Pipe{
                let bytes = key_to_bytes(&key)?;
                return Some(UseCase::Passthrough(bytes));
            } 

            if key.modifiers.contains(KeyModifiers::CONTROL){
                if let KeyCode::Char(c) = key.code {
                    let ctrl_byte = (c as u8) & 0x1F;
                    return Some(UseCase::JobControl(ctrl_byte))
                }

            }
            match key.code {
                KeyCode::Backspace => Some(UseCase::LineDicipline(Key::Backspace)),
                KeyCode::Enter => Some(UseCase::CmdExecution),
                KeyCode::Char(c) => Some(UseCase::LineDicipline(Key::Char(c.to_string()))),
                KeyCode::Tab => Some(UseCase::LineDicipline(Key::Tab)),
                KeyCode::Left => Some(UseCase::LineDicipline(Key::Left)),
                KeyCode::Right => Some(UseCase::LineDicipline(Key::Right)),
                KeyCode::Up => Some(UseCase::LineDicipline(Key::Up)),
                KeyCode::Down => Some(UseCase::LineDicipline(Key::Down)),
                _ => None,
                }
            },

        InputEvent::Agent(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            Some(UseCase::LineDicipline(Key::Char(text.to_string())))
        }

        _=>{None}
    }
}

}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn pipe_state_passthrough() {
        let event = InputEvent::User(key(KeyCode::Enter));

        let result = InputInterprter::map(event, TermState::Pipe);

        assert_eq!(
            result,
            Some(UseCase::Passthrough(vec![b'\r']))
        );
    }

    #[test]
    fn cmdline_enter_executes() {
        let event = InputEvent::User(key(KeyCode::Enter));

        let result = InputInterprter::map(event, TermState::Cmdline);

        assert_eq!(
            result,
            Some(UseCase::CmdExecution)
        );
    }

    #[test]
    fn ctrl_c_maps_to_job_control() {
        let event = InputEvent::User(ctrl_char('c'));

        let result = InputInterprter::map(event, TermState::Cmdline);

        assert_eq!(
            result,
            Some(UseCase::JobControl(0x03)) // Ctrl+C
        );
    }
}
