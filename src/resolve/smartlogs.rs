use serde::{Deserialize, Serialize};
use chrono::{DateTime,Utc};


#[derive(Serialize, Deserialize, Debug)]
pub enum LogType{
  FatalErr,
  LogicalErr,
  RuntimeErr,
  OptProposition,
  None,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmartLog{
  message:String,
  kind:LogType,
  line:i64,
  #[serde(default = "default_timestamp")]
  timestamp: chrono::DateTime<Utc>,
}

pub fn default_timestamp() -> DateTime<Utc> {
      Utc::now()
}


impl SmartLog{
    pub fn new(message:String , kind:LogType , line:i64)->SmartLog{ 
          return SmartLog{
            message,
            kind,
            line,
            timestamp:Utc::now(),
          }
    }
    pub fn print(&self){
        println!("[{:?}][Kind:{:?}][Message:{}][Line:{}]" , self.timestamp , self.kind , self.message , self.line);
    }
 

}