use crossterm::event::{KeyCode, KeyEvent, KeyModifiers , Event , read};
use std::sync::{Arc, Mutex};
use std::io::{Error , ErrorKind};
use super::terminal::{Terminal , LastInputEvent , TerminalState};

#[derive(PartialEq , Debug)]
pub enum InputAction {
    CmdService,
    SendToPty,
}

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

pub  fn next_action_is(term:std::sync::MutexGuard<'_, Terminal>) ->Vec<InputAction>{
    match term.mode_is(){
        TerminalState::CommandLine =>vec![InputAction:: CmdService], 
        
        _ => vec![InputAction::SendToPty]
    }
}

pub fn handle_user_input(events_writter:&std::sync::mpsc::Sender<LastInputEvent> , terminal:&Arc<Mutex<Terminal>> , key:KeyEvent) -> Result<(),Error>{

    if let Some(bytes) = key_to_bytes(&key) {
        for a in next_action_is(terminal.lock().unwrap()).iter(){
            match a{
                InputAction::SendToPty =>{
                    if events_writter.send(LastInputEvent::PtyInput(bytes.clone())).is_err(){
                        return Err(Error::new(ErrorKind::Other, "Error sending event to event loop"));
                    }
                }
                InputAction::CmdService=>{
                     if events_writter.send(LastInputEvent::UserKey(key)).is_err(){
                        return Err(Error::new(ErrorKind::Other, "Error sending event to event loop"));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn read_user_input(events_writter: std::sync::mpsc::Sender<LastInputEvent> , terminal:Arc<Mutex<Terminal>>) {
    loop {
        if let Event::Key(key) = read().unwrap() {
            match handle_user_input(&events_writter , &terminal, key){
                Err(_) => break,
                _=>{}
            }
        }
    }
    drop(events_writter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_state_test() {
        let mut term = Terminal::default();
        term._set_state_to(TerminalState::CommandLine);

        let actions = next_action_is(std::sync::Mutex::new(term).lock().unwrap());

        assert_eq!(actions, vec![InputAction::CmdService]);
    }

    #[test]
    fn  fullscreen_state_test() {
        let mut term = Terminal::default();
        term._set_state_to(TerminalState::FullScreen);

        let actions = next_action_is(
            std::sync::Mutex::new(term).lock().unwrap());

        assert!(actions.contains(&InputAction::SendToPty));
    }

    #[test]
fn key_to_bytes_basic_mappings_test() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // normal char
    let k = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    assert_eq!(key_to_bytes(&k), Some(vec![b'a']));

    // ctrl char (Ctrl+C)
    let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(key_to_bytes(&k), Some(vec![3]));

    // enter
    let k = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(key_to_bytes(&k), Some(vec![b'\r']));

    // backspace
    let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(key_to_bytes(&k), Some(vec![0x7F]));

    // arrow key
    let k = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(key_to_bytes(&k), Some(b"\x1b[A".to_vec()));
}

}