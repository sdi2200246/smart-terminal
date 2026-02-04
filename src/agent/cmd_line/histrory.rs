use super::cmd_line::HistoryState;


impl HistoryState{

    pub fn add_cmd(&mut self , cmd:String){
        self.cmds.push(cmd);
        self.index = self.cmds.len()-1;
    }

    pub fn get_next_cmd(&mut self)->Option<String>{
        if self.cmds.len() == 0{
            return None;
        } 

        let next = self.cmds[self.index].clone();
        if self.index > 0 { 
            self.index -= 1 ;
        }
        return Some(next);
    }

    pub fn get_prev_cmd(&mut self)-> Option<String> {
        if self.cmds.len() == 0{
            return None;
        } 

        if self.index < self.cmds.len()-1{
            self.index+=1;
        }
        return Some(self.cmds[self.index].clone());
    }

    pub fn clear_state(&mut self){
        if (self.cmds.len() == 0){
            return
        }
        self.index = self.cmds.len()-1; 
    }

}

impl Default for HistoryState{
    fn default() -> Self {
        Self { 
            cmds:Vec::new(), 
            index:0 
        }
    }
}
