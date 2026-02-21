use super::tab::TabState;
use super::histrory::HistoryState;
use super::buffer::Buffer;

pub struct CmdLineState{
    pub buffer:Buffer,
    pub tab_state:TabState,
    pub history_state:HistoryState
}

impl  CmdLineState {
    pub fn clear(&mut self ){
        self.buffer.clear_buffer();
        self.tab_state.clear_state();
        self.history_state.clear_state();
    }

    pub fn insert_char(&mut self , c:String){
        self.buffer.push(&c);
    }

    pub fn navigate_history_up(&mut self){
        if let Some(cmd) = self.history_state.get_next_cmd(){
            self.buffer.clear_buffer();
            self.buffer.push(&cmd);
        } 
    }
    pub fn navigate_history_down(&mut self){
        if let Some(cmd) = self.history_state.get_prev_cmd(){
            self.buffer.clear_buffer();
            self.buffer.push(&cmd);
        } 
    }
    pub fn move_cursor_right(&mut self){
        self.buffer.cursor_right();
    }

    pub fn move_cursor_left(&mut self){
        self.buffer.cursor_left();
    }

    pub fn apply_tab(&mut self , cwd:String){
        self.history_state.clear_state();
            let suggestions = self.tab_state.run_tab(self.buffer.get_user_buffer() , &cwd);
            match suggestions{
                Ok(vec) => {
                    if vec.len() == 1 {
                        self.buffer.push(&vec[0]);
                    }
                }
                _=>{}
            }
    }
    pub fn restore_history_state(&mut self){
        self.history_state.clear_state();
    }

    pub fn restore_tab_state(&mut self){
        self.tab_state.clear_state();
    }

    pub fn apply_backsapce(&mut self){
        self.buffer.pop();
    }



}

impl Default for CmdLineState{
    fn default()->Self{
        Self{
            buffer:Buffer::default(),
            tab_state:TabState::default(),
            history_state:HistoryState::default(), 
        }
    }
}