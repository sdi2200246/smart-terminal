use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
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
}

impl Terminal {

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

}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            state:TerminalState::CommandLine,
            cmd_line:CmdLineState::default()
        }
    }
}