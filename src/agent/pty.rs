use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write };
use std::sync::mpsc::channel;
use std::thread;

use crate::agent::terminal::TerminalView;
use super::input;
use super::output;
use super::terminal::{Event, Terminal, TerminalAction};
use super::render::render_terminal;

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

    enable_raw_mode()?;

    let (reader, writer) = spawn_shell()?;

    let (tx, rx) = channel::<Vec<u8>>();
    let (event_writter , events_reader) = channel::<Event>();
    init_shell(&tx);
    let terminal = Terminal::default();

    let events = event_writter.clone();
    let pty_reader = thread::spawn(move || {
        output::read_pty_output(reader, events);
    });

    let pty_writer = thread::spawn(move || {
        handle_input_stream(rx, writer);
    });

    let user_reader = thread::spawn(move || {
        input::read_user_input(event_writter);
    });

    let events_reader = thread::spawn(move|| {
        read_events(tx, events_reader , terminal);
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
fn read_events(tx: std::sync::mpsc::Sender<Vec<u8>> , events:std::sync::mpsc::Receiver<Event> , mut terminal:Terminal){

    while let Ok(event) =  events.recv(){
        event_handler(event, &tx , &mut terminal);
    }
    drop(events);
}

fn event_handler(event:Event ,tx:& std::sync::mpsc::Sender<Vec<u8>> , terminal:&mut Terminal){

    let actions:Vec<TerminalAction>;

    match event{
        Event::Input(ievent)=>{
            let ctx = &terminal.context;
            let cmdline = &mut terminal.cmd_line;
            let state = &mut terminal.state;

            actions = state.handle_input(ievent, ctx, cmdline);
        }
        Event::Output(bytes) =>{
            actions = terminal.state.handle_output(&bytes);

        }
    }
    for action in actions{
         match action{
            TerminalAction::Flush(output) => terminal.flush(output),
            TerminalAction::Render => render_terminal(TerminalView::from((&terminal.context , &terminal.cmd_line))),
            TerminalAction::SwitchState(state) =>terminal.switch_state_to(state),
            TerminalAction::UpdateContext(new_ctx) =>terminal.update_context(new_ctx),
            TerminalAction::SendPty(input)=>terminal.send_pty(tx, input),
            _=>{}
        }
    }
}