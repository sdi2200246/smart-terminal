pub enum TabMode{
    Cleared,
    Cycling,
    Firstmatch,
    AiCompletion
}
pub struct TabState{
    pub mode:TabMode,
    pub candidates:Vec<String>,
    pub current_option:usize,
    //to do mode.
}
pub struct HistoryState{
    pub cmds:Vec<String>,
    pub index:usize
}

pub struct Buffer{
    pub user_buffer:String,
    pub suggestion_buffer:String,
    pub cursor:usize,
}

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