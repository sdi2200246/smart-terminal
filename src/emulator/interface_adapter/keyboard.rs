use crossterm::event::{Event, read};
use std::io::{Error , ErrorKind};
use std::sync::mpsc::Sender;
use super::terminal::{self , InputEvent};

pub struct Keyboard {
    tx: Sender<terminal::Event>,
}
impl Keyboard {
    pub fn new(tx: Sender<terminal::Event>) -> Self {
        Self { tx }
    }

    pub fn run(self) -> Result<(), Error> {
        loop {
            match read(){
                Ok(Event::Key(key)) => {
                    if self.tx
                        .send(terminal::Event::Input(InputEvent::User(key)))
                        .is_err()
                    {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Event loop disconnected",
                        ));
                    }
                }
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }
}