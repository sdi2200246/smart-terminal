use std::io::Read;
use super::terminal::Event;

fn pty_output_handler(buffer:&mut[u8] , event_writer:&std::sync::mpsc::Sender<Event> , bytes_read:usize){

        let buf = buffer[..bytes_read].to_vec();
        event_writer.send(Event::Output(buf)).unwrap();
}
pub fn read_pty_output(mut reader: Box<dyn Read + Send> ,event_writer:std::sync::mpsc::Sender<Event>) {
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                pty_output_handler(&mut buffer, &event_writer, n);
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {}", e);
                break;
            }
        }
    }
}