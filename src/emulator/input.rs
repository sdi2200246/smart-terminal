use crossterm::{event::{Event, KeyEvent, read}};
use std::io::{Error , ErrorKind};
use super::terminal::{InputEvent};
use super::terminal;

pub fn handle_user_input(events_writter:&std::sync::mpsc::Sender<terminal::Event> , key:KeyEvent) -> Result<(),Error>{

       if events_writter.send(terminal::Event::Input(InputEvent::User(key))).is_err(){
            return Err(Error::new(ErrorKind::Other, "Error sending event to event loop"));
        }
    Ok(())
}
pub fn read_user_input(events_writter: std::sync::mpsc::Sender<terminal::Event>) {
    loop {
        if let Event::Key(key) = read().unwrap() {
            match handle_user_input(&events_writter,key){
                Err(_) => break,
                _=>{}
            }
        }
    }
    drop(events_writter);
}