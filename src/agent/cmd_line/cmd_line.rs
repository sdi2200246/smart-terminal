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
// pub struct Cursor{
//     user_index:(i64 , i64),
//     suggestion_index:(i64 , i64),
// }

pub struct Buffer{
    pub user_buffer:String,
    pub suggestion_buffer:String
}

pub struct CmdLineState{
    pub buffer:Buffer,
    pub tab_state:TabState,
}

impl Default for CmdLineState{

    fn default()->Self{
        Self{
            buffer:Buffer::default(),
            tab_state:TabState::default(),   
        }
    }
}