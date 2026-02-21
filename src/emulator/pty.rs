use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write };
use std::sync::mpsc::channel;
use std::thread;
use std::sync::mpsc::{Sender , Receiver};

use super::terminal::{Event, Terminal, TerminalAction , TerminalView};
use super::render::render_terminal;
use super::interface_adapter::input_boundary::InputInterprter;
use super::interface_adapter::keyboard;
use super::interface_adapter::pty_output;

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

fn init_shell(tx: &Sender<Vec<u8>>) {
    let _ = tx.send("export PROMPT_COMMAND='echo __AGENT_DONE__:$PWD'\n".into());
    let _ = tx.send("export PS1=''\n".into());
    // let _ = tx.send("stty -echo\n".into());
}


pub fn pty_agent() -> Result<(), std::io::Error> {
    enable_raw_mode()?;

    let (reader, writer) = spawn_shell()?;

    // PTY channel (terminal → shell)
    let (pty_tx, pty_rx) = channel::<Vec<u8>>();

    // Central event channel
    let (event_tx, event_rx) = channel::<Event>();

    init_shell(&pty_tx);

    let terminal = Terminal::default();

    // PTY reader thread
    let event_tx_clone = event_tx.clone();
    let pty_reader_handle = thread::spawn(move || {
        let reader = pty_output::PtyReader::new(reader, event_tx_clone);
        let _ = reader.run();
    });

    // PTY writer thread
    let pty_writer_handle = thread::spawn(move || {
        handle_input_stream(pty_rx, writer);
    });

    // Keyboard thread
    let keyboard_handle = thread::spawn(move || {
        let keyboard = keyboard::Keyboard::new(event_tx);
        let _ = keyboard.run();
    });

    // Event loop thread
    let event_loop_handle = thread::spawn(move || {
        read_events(pty_tx, event_rx, terminal);
    });

    keyboard_handle.join().unwrap();
    pty_writer_handle.join().unwrap();
    pty_reader_handle.join().unwrap();
    event_loop_handle.join().unwrap();

    disable_raw_mode()?;

    Ok(())
}

fn handle_input_stream(rx:Receiver<Vec<u8>>, mut pty_writer: Box<dyn Write + Send>) {

    for input in rx.iter() {
        if pty_writer.write_all(&input).is_err() {
            eprintln!("Error writing to PTY");
            break;
        }
    }
}
fn read_events(tx:Sender<Vec<u8>> , events:Receiver<Event> , mut terminal:Terminal){

    while let Ok(event) =  events.recv(){
        event_handler(event, &tx , &mut terminal);
    }
    drop(events);
}

fn event_handler(event:Event ,tx:&Sender<Vec<u8>> , terminal:&mut Terminal){

    let actions:Vec<TerminalAction>;

    match event{
        Event::Input(ievent)=>{
            let ctx = &terminal.context;
            let cmdline = &mut terminal.cmd_line;
            let state = &mut terminal.state;
            if let Some(usecase) = InputInterprter::map(ievent, state.state()){
                actions = state.handle_input(usecase, ctx, cmdline);
            }
            else{
                return;
            }
        }
        Event::Output(bytes) => actions = terminal.state.handle_output(&bytes),
    
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