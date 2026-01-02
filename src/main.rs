mod chat;
mod context;
mod resolve;
mod agent;
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: smart-terminal -chat \"your message here\"");
        return;
    }
    let flag = &args[1];

    match flag.as_str() {

        "-chat" => {
            let prompt = args[2..].join(" ");
            let response = tokio::runtime::Runtime::new().unwrap().block_on(chat::chat(prompt));
             match response {
                Ok(answer) => println!("{}", answer),
                Err(e) => eprintln!("Error: {}", e),
             }
        }

        "-resolve"=>{
            resolve::detect();
        }

        "-agent"=>{
            agent::pty::pty_agent();
        }

        _=> {
            eprintln!("Unknown option: {}", flag);
        }   
        
    }
}
