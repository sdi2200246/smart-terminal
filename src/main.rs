mod emulator;
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: smart-terminal -chat \"your message here\"");
        return;
    }
    let flag = &args[1];

    match flag.as_str() {

        "-agent"=>{
            emulator::pty::pty_agent().unwrap();
        }
        _=> {
            eprintln!("Unknown option: {}", flag);
        }   
        
    }
}
