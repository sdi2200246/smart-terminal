use std::sync::{Arc, Mutex};
use std::{env};
use crossterm::event::{KeyCode,KeyModifiers , KeyEvent};
use super::cmd_line::CmdLineState;
use super::render::RenderActions;

#[derive(PartialEq , Debug)]
pub enum  TerminalActions{
    Render(RenderActions),
    SendPty(Vec<u8>),
    UpdateState,
    Ignone,
}

#[derive(PartialEq)]
pub enum LastInputEvent{
    UserKey(KeyEvent),
    AgentSuggestion(String),
    PtyInput(Vec<u8>),
    None
}
#[derive(PartialEq)]
pub enum TerminalState{
    CommandLine,
    FullScreen, 
    Passive,
}
pub struct Terminal {
    state:TerminalState,
    pub cmd_line:CmdLineState,
    pub cwd:String,
}

impl Terminal {


    pub fn extract_cwd(output: &str) -> Option<String> {
        output
            .lines()
            .find_map(|line| line.strip_prefix("__AGENT_DONE__:"))
            .map(|cwd| cwd.to_string())
    }

    pub fn update_terminal_state(&mut self , output:&str){
        if     output.contains("\x1b[?1049h")
            || output.contains("\x1b[?47h")
            || output.contains("\x1b[?1047h")
        {
            self.state = TerminalState::FullScreen;
            return;
        }

        if     output.contains("\x1b[?1049l")
            || output.contains("\x1b[?47l")
            || output.contains("\x1b[?1047l")
        {
            self.state = TerminalState::CommandLine;
            return;
        }

        if output.contains("__AGENT_DONE__") {
            self.state = TerminalState::CommandLine;
            if let Some(cwd) = Self::extract_cwd(output) {
                self.cwd = cwd;
            }
            return;
        }

    }

    pub fn update_terminal_state_from_input(&mut self){
        self.state = TerminalState::Passive
    }
    
    pub fn _set_state_to(&mut self , state:TerminalState){
        self.state = state;
    }

    pub fn mode_is(&self)->TerminalState{
            match self.state{
                TerminalState::CommandLine => TerminalState::CommandLine,
                TerminalState::FullScreen => TerminalState::FullScreen,
                TerminalState::Passive=>TerminalState::Passive,
            }
    }

    fn input_event_reducer(&mut self  , event:LastInputEvent)->Vec<TerminalActions>{
        match event {   
            LastInputEvent::UserKey(key)=>{

                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char(c) = key.code {
                        self.cmd_line.tab_state.clear_state();
                        self.cmd_line.buffer.clear_buffer();
                        let ctrl_byte = (c as u8) & 0x1F;
                        return vec![TerminalActions::SendPty(vec![ctrl_byte])];
                    }
                }
                match key.code {
                    KeyCode::Backspace =>{
                        self.cmd_line.tab_state.clear_state();
                        self.cmd_line.buffer.pop();
                        return vec![TerminalActions::Render(RenderActions::Backspace)];
                    }
                    KeyCode::Enter=>{
                        self.cmd_line.tab_state.clear_state();
                        let mut bytes = self.cmd_line.buffer.take_user_bytes();
                        bytes.push(b'\n');
                        return vec![TerminalActions::SendPty(bytes) ,TerminalActions::UpdateState];
                    }

                    KeyCode::Char(c) => {
                        self.cmd_line.buffer.push(&c.to_string());
                        return vec![TerminalActions::Render(RenderActions::Char(c))];
                    }

                    KeyCode::Tab =>{
                        let suggestions = self.cmd_line.tab_state.run_tab(self.cmd_line.buffer.get_user_buffer() , &self.cwd);
                        match suggestions{
                            Ok(vec) => {
                                if vec.len() == 1 {
                                    self.cmd_line.buffer.push(&vec[0]);
                                    return vec![TerminalActions::Render(RenderActions::Tab(self.cmd_line.buffer.get_user_buffer().to_string()))];
                                }
                                return vec![TerminalActions::Ignone];
                            }

                            _=>{ return vec![TerminalActions::Ignone];}
                        }
                    }
                    _=>{return vec![TerminalActions::Ignone];}
                }
            }
            LastInputEvent::PtyInput(bytes)=>{ return vec![TerminalActions::SendPty(bytes)];}

            _=>{vec![TerminalActions::Ignone]}
            
        }

    }

    pub fn event_reducer(&mut self , event:LastInputEvent)->Vec<TerminalActions>{
        return  self.input_event_reducer(event);
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            state:TerminalState::CommandLine,
            cmd_line:CmdLineState::default(),
            cwd : env::current_dir()
            .expect("failed to get cwd")
            .display()
            .to_string()
        }
    }
}