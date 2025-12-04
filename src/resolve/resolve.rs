use std::process::Command;
use crate::chat;
use super::SmartLog;

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

  let suspicious_code = r#"
    impl Bank {
        pub fn new() -> Bank {
            Bank {
                users: Vec::new(),
                total: 0,
            }
        }

        pub fn insert_user(&mut self , user:BUsers){
            self.users.push(user);
        }   

        pub fn find_user(&self,id:i64)->Result <&BUsers , BankError>{

            for &user in &self.users{
                if user.id == id {
                    return Ok(user);
                }
            }
            return Err(BankError{ message:"user not  found".to_string() , t:BankErr::NOTFOUND});
        }
    }
      "#;

  let p = chat::Promt::new(
        "You are a Rust static analysis assistant. Identify logic , optimization bugs.".to_string(),
        "Analyze the following code and follow the FORMAT.".to_string()
    ).with_context(suspicious_code.to_string());

  println!("{}", p.to_smartlog_prompt());
  let response = tokio::runtime::Runtime::new().unwrap().block_on(chat::chat(p.to_smartlog_prompt()));
  let response_str = response.unwrap();    
  println!("{}" , response_str);
  let log: SmartLog = serde_json::from_str(&response_str).unwrap();
  log.print();
    
}


