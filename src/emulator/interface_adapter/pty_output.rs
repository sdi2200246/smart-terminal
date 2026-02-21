use std::io::{Read, Error};
use std::sync::mpsc::Sender;

use super::terminal::Event;

pub struct PtyReader {
    reader: Box<dyn Read + Send>,
    tx: Sender<Event>,
}

impl PtyReader {
    pub fn new(reader: Box<dyn Read + Send>, tx: Sender<Event>) -> Self {
        Self { reader, tx }
    }

    pub fn run(mut self) -> Result<(), Error> {
        let mut buffer = [0u8; 4096];

        loop {
            match self.reader.read(&mut buffer) {
                Ok(0) => break, 
                Ok(n) => {
                    let output = buffer[..n].to_vec();

                    if self.tx.send(Event::Output(output)).is_err() {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}