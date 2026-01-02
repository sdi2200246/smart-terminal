use std::sync::{Arc, Mutex};
use std::io::{Read, Write };
use super::terminal::{Terminal , TerminalState};
use super::render::draw_prompt;

#[derive(PartialEq , Debug)]
enum OutputActions{
    RenderPromt,
    Flush,
}
fn next_action_is(term:&std::sync::MutexGuard<'_, Terminal>)->Vec<OutputActions>{

    match term.mode_is(){
        TerminalState::CommandLine => vec![OutputActions::Flush , OutputActions::RenderPromt],
        _ => vec![OutputActions::Flush],
    }
}

fn pty_output_handler(terminal:&Arc<Mutex<Terminal>> , buffer:&mut [u8], stdout:&mut dyn Write , bytes_read:usize){

    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut term = terminal.lock().unwrap();
    term.update_terminal_state(&output);

    for a in next_action_is(&term){
        match a{
            OutputActions::Flush =>{ 
                let _ = stdout.write_all(&buffer[..bytes_read]);
                let _ = stdout.flush();
            }
            OutputActions::RenderPromt => {
                draw_prompt(term.cwd.clone());
            }
        }
    }
}
pub fn read_pty_output(mut reader: Box<dyn Read + Send> ,terminal:Arc<Mutex<Terminal>> , mut stdout: Box<dyn Write + Send>,) {
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                pty_output_handler(&terminal, &mut buffer, &mut stdout, n);
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {}", e);
                break;
            }
        }
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn cmdline_outputs_flush_and_prompt() {
//         let mut term = Terminal::default();
//         term._set_state_to(TerminalState::CommandLine);

//         let actions = next_action_is(std::sync::Mutex::new(term).lock().unwrap());

//         assert!(actions.contains(&OutputActions::Flush));
//         assert!(actions.contains(&OutputActions::RenderPromt));
//     }
//     #[test]
//     fn non_cmdline_outputs_flush_only() {
//         let mut term = Terminal::default();
//         term._set_state_to(TerminalState::FullScreen);

//         let actions = next_action_is(std::sync::Mutex::new(term).lock().unwrap());

//         assert_eq!(actions, vec![OutputActions::Flush]);
//     }
//    #[test]
//     fn pty_output_handler_writes_to_stdout() {
//         let mut term = Terminal::default();
//         term._set_state_to(TerminalState::FullScreen);
//         let terminal = Arc::new(Mutex::new(term));

//         let mut buffer = *b"hello";
//         let mut out = Vec::<u8>::new(); 

//         pty_output_handler(&terminal, &mut buffer, &mut out, 5);

//         assert_eq!(out, b"hello");
//     }



// }