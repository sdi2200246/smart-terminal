mod chat;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: smart-terminal -chat \"your message here\"");
        return;
    }

    let flag = &args[1];

    match flag.as_str() {
        // Collect the rest of the arguments as the prompt
        
        "-chat" => {
            let prompt = args[2..].join(" ");
            let response = tokio::runtime::Runtime::new().unwrap().block_on(chat::chat(prompt));
             match response {
                Ok(answer) => println!("{}", answer),
                Err(e) => eprintln!("Error: {}", e),
             }
        }
        _=> {
            eprintln!("Unknown option: {}", flag);
        }   
        
    }
}
