use std::process::Command;
use crate::chat;

pub fn resolve(cmd:&String , args:&[String]){

  if let Ok(output) = Command::new(cmd).args(args).output() {

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit = output.status.code().unwrap_or(-1);

    println!("=== OUTPUT START ===");
    println!("{stdout}{stderr}");
    println!("=== OUTPUT END ===");

    if exit != 0 {
        println!("Error! Now sending to AI…");
        let response = tokio::runtime::Runtime::new().unwrap().block_on(chat::chat(stderr.to_string()));
             match response {
                Ok(answer) => println!("{}", answer),
                Err(e) => eprintln!("Error: {}", e),
             }
    }
    } else {
        eprintln!("Failed to run command: {}", cmd);
        return;
    }
}
pub fn detect(){    
}


