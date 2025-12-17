use crossterm::event::{KeyCode, KeyEvent, KeyModifiers , Event , read};
use std::sync::{Arc, Mutex};
use std::io::{Error , ErrorKind};

use super::render::{draw_backspace, draw_character};
use super::terminal::{Terminal , LastInput , TerminalState};

#[derive(PartialEq , Debug)]
pub enum InputAction {
    CmdService,
    SendToPty,
    Ignore,
    UpdateState
}

fn key_hander(code:KeyCode)-> Result<() , Error>{
    match code {
        KeyCode::Char(c) => {
           draw_character(c);
        }
        KeyCode::Backspace => {
           draw_backspace();
        }
        _ => {},
    }
    Ok(())
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

pub  fn next_action_is(mut term:std::sync::MutexGuard<'_, Terminal> , key:&KeyEvent) ->Vec<InputAction>{
    let last_input = term.update_terminal_last_input(*key);

    match term.mode_is(){
        TerminalState::FullScreen => vec![InputAction::SendToPty],

        TerminalState::CommandLine => { 
            match last_input{
                LastInput::Other =>  vec![InputAction::SendToPty , InputAction::CmdService],

                LastInput::Tab => vec![InputAction::CmdService],

                LastInput::Enter => vec![InputAction::UpdateState , InputAction::SendToPty],

                _ =>  vec![InputAction::Ignore]
            }
        }
        TerminalState::Passive => vec![InputAction::SendToPty],
    }
}

pub fn handle_user_input(pty_writter:&std::sync::mpsc::Sender<Vec<u8>> , terminal:&Arc<Mutex<Terminal>> , key:KeyEvent) -> Result<(),Error>{

    if let Some(bytes) = key_to_bytes(&key) {
        for a in next_action_is(terminal.lock().unwrap() , &key).iter(){
            match a{
                InputAction::SendToPty =>{
                    if pty_writter.send(bytes.clone()).is_err(){
                        return Err(Error::new(ErrorKind::Other, "Error sending bytes to PTY"));
                    }
                }
                InputAction::CmdService=>{
                    key_hander(key.code).unwrap();
                }
                InputAction::UpdateState =>{
                    let mut term = terminal.lock().unwrap();
                    term.update_terminal_state_from_input();
                }

                _=> {}
            }
        }
    }
    Ok(())
}

pub fn read_user_input(tx: std::sync::mpsc::Sender<Vec<u8>> , terminal:Arc<Mutex<Terminal>>) {
    loop {
        if let Event::Key(key) = read().unwrap() {
            match handle_user_input(&tx , &terminal, key){
                Err(_) => break,
                _=>{}
            }
        }
    }
    drop(tx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_test() {
        let mut term = Terminal::default();
        term.set_state_to(TerminalState::CommandLine);

        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let actions = next_action_is(std::sync::Mutex::new(term).lock().unwrap(), &key);

        assert_eq!(actions, vec![InputAction::CmdService]);
    }

    #[test]
    fn enter_test() {
        let mut term = Terminal::default();
        term.set_state_to(TerminalState::CommandLine);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = next_action_is(std::sync::Mutex::new(term).lock().unwrap(), &key);

        assert!(actions.contains(&InputAction::UpdateState));
        assert!(actions.contains(&InputAction::SendToPty));
    }

    #[test]
    fn  normal_char_test() {
        let mut term = Terminal::default();
        term.set_state_to(TerminalState::CommandLine);

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let actions = next_action_is(
            std::sync::Mutex::new(term).lock().unwrap(),
            &key
        );

        assert!(actions.contains(&InputAction::SendToPty));
        assert!(actions.contains(&InputAction::CmdService));
        assert!(!actions.contains(&InputAction::UpdateState));
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