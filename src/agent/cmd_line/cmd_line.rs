use crossterm::event::{KeyCode,KeyModifiers};

use crate::agent::terminal::{LastInputEvent, TerminalActions};
use crate::agent::render::RenderActions;

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
    buffer:Buffer,
    tab_state:TabState,
}

impl CmdLineState{

    pub fn cmd_line_reducer(&mut self , event:LastInputEvent)->Vec<TerminalActions>{
        match event {   
            LastInputEvent::UserKey(key)=>{

                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char(c) = key.code {
                        self.tab_state.clear_state();
                        self.buffer.clear_buffer();
                        let ctrl_byte = (c as u8) & 0x1F;
                        return vec![TerminalActions::SendPty(vec![ctrl_byte])];
                    }
                }
                match key.code {
                    KeyCode::Backspace =>{
                        self.tab_state.clear_state();
                        self.buffer.pop();
                        return vec![TerminalActions::Render(RenderActions::Backspace)];
                    }
                    KeyCode::Enter=>{
                        self.tab_state.clear_state();
                        let mut bytes = self.buffer.take_user_bytes();
                        bytes.push(b'\n');
                        return vec![TerminalActions::SendPty(bytes) ,TerminalActions::UpdateState];
                    }

                    KeyCode::Char(c) => {
                        self.buffer.push(&c.to_string());
                        return vec![TerminalActions::Render(RenderActions::Char(c))];
                    }

                    _=>{ return vec![TerminalActions::Ignone];}
                }
            }
            LastInputEvent::PtyInput(bytes)=>{ return vec![TerminalActions::SendPty(bytes)];}


            _=>{vec![TerminalActions::Ignone]}
            
        }
    }

}


impl Default for CmdLineState{

    fn default()->Self{
        Self{
            buffer:Buffer::default(),
            tab_state:TabState::default(),   
        }
    }
}


#[cfg(test)]
mod tests {
    
    use crossterm::event::KeyEvent;
    use super::*;

    #[test]
    fn reducer_char_inserts_and_renders() {
        let mut cmd = CmdLineState::default();

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let actions = cmd.cmd_line_reducer(LastInputEvent::UserKey(key));

        assert_eq!(cmd.buffer.get_user_buffer(), "a");
        assert_eq!(
            actions,
            vec![TerminalActions::Render(RenderActions::Char('a'))]
        );
    }

    #[test]
    fn reducer_backspace_pops_and_renders() {
        let mut cmd = CmdLineState::default();

        // seed buffer
        cmd.buffer.push("a");

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let actions = cmd.cmd_line_reducer(LastInputEvent::UserKey(key));

        assert_eq!(cmd.buffer.get_user_buffer(), "");
        assert_eq!(
            actions,
            vec![TerminalActions::Render(RenderActions::Backspace)]
        );
    }

    #[test]
    fn reducer_enter_sends_bytes_and_clears_buffer() {
        let mut cmd = CmdLineState::default();

        cmd.buffer.push("ls");

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = cmd.cmd_line_reducer(LastInputEvent::UserKey(key));

        assert_eq!(
            actions,
            vec![TerminalActions::SendPty(b"ls\n".to_vec()) , TerminalActions::UpdateState]
        );
        assert_eq!(cmd.buffer.get_user_buffer(), "");
    }

    #[test]
    fn reducer_ctrl_c_sends_interrupt_byte() {
        let mut cmd = CmdLineState::default();

        cmd.buffer.push("sleep 10");

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let actions = cmd.cmd_line_reducer(LastInputEvent::UserKey(key));

        assert_eq!(
            actions,
            vec![TerminalActions::SendPty(vec![0x03])]
        );
        assert_eq!(cmd.buffer.get_user_buffer(), "");
    }



}