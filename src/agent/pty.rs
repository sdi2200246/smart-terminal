use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write ,};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

use super::input;
use super::output;
use super::terminal::{Terminal , LastInputEvent , TerminalActions};
use super::render;

fn spawn_shell() -> Result<(Box<dyn Read + Send>, Box<dyn Write + Send>), std::io::Error> {
    let pty_system = NativePtySystem::default();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let cmd = CommandBuilder::new("bash");
    pair.slave
        .spawn_command(cmd)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok((reader, Box::new(writer)))
}

fn init_shell(tx: &std::sync::mpsc::Sender<Vec<u8>>) {
    let _ = tx.send("export PROMPT_COMMAND='echo __AGENT_DONE__:$PWD'\n".into());
    let _ = tx.send("export PS1=''\n".into());
    let _ = tx.send("stty -echo\n".into());
}


pub fn pty_agent() -> Result<(), std::io::Error> {
    let terminal = Arc::new(Mutex::new(Terminal::default()));

    enable_raw_mode()?;

    let (reader, writer) = spawn_shell()?;

    let (tx, rx) = channel::<Vec<u8>>();
    let (event_writter , events_reader) = channel::<LastInputEvent>();
    init_shell(&tx);

    let term = Arc::clone(&terminal);
    let pty_reader = thread::spawn(move || {
        output::read_pty_output(reader, term , Box::new(std::io::stdout()));
    });

    let pty_writer = thread::spawn(move || {
        handle_input_stream(rx, writer);
    });

    let term = Arc::clone(&terminal);
    let user_reader = thread::spawn(move || {
        input::read_user_input(event_writter, term);
    });

    let term = Arc::clone(&terminal);
    let events_reader = thread::spawn(move|| {
        read_events(tx, events_reader , term);
    });


    user_reader.join().unwrap();
    pty_writer.join().unwrap();
    pty_reader.join().unwrap();
    events_reader.join().unwrap();

    disable_raw_mode()?;
    Ok(())
}


fn handle_input_stream(rx: std::sync::mpsc::Receiver<Vec<u8>>, mut pty_writer: Box<dyn Write + Send>) {

    for input in rx.iter() {
        if pty_writer.write_all(&input).is_err() {
            eprintln!("Error writing to PTY");
            break;
        }
    }
}
fn read_events(tx: std::sync::mpsc::Sender<Vec<u8>> , events:std::sync::mpsc::Receiver<LastInputEvent> , terminal:Arc<Mutex<Terminal>>){

        while let Ok(event) =  events.recv(){
            event_handler(event, &tx, &terminal);
        }
    drop(events);
}

fn event_handler(event:LastInputEvent ,tx:& std::sync::mpsc::Sender<Vec<u8>> , term:&Arc<Mutex<Terminal>>){
        
        let mut terminal = term.lock().unwrap();

        for action in terminal.event_reducer(event){
            match action{
                TerminalActions::SendPty(bytes) =>{
                    tx.send(bytes).unwrap();
                }
                TerminalActions::Render(render_action)=>{
                    render::render_handler(render_action);
                }
                TerminalActions::UpdateState=>{
                    terminal.update_terminal_state_from_input();
                }
                _=>{}
            }
        }
}