use std::io::{Write , stdout};
use std::{env};
use crossterm:: {event::{KeyEvent}};
use super::cmd_line::CmdLineState;
use super::state::terminal_state::TerminalState;
use super::state::pipe_state::PipeState;
use super::state::cmdline_state::CmdState;

#[derive(PartialEq , Debug)]
pub enum TermState{
    Pipe,
    Cmdline,
}
#[derive(PartialEq , Debug)]
pub enum TerminalAction{
    SendPty(Vec<u8>),
    SwitchState(TermState),
    UpdateContext(ContextUpdate),
    Flush(Vec<u8>), 
    Render,
    NOop,
}

pub enum InputEvent{
    User(KeyEvent),
   Agent(Vec<u8>),
}
pub enum Event{
    Input(InputEvent),
    Output(Vec<u8>),
}

#[derive(PartialEq , Debug)]
pub struct ContextUpdate {
    pub cwd: Option<String>,
    pub history:Option<Vec<String>>,
    pub files:Option<Vec<String>>,
}
pub struct Context{
    pub cwd:String,
    pub history:Vec<String>,
    pub files:Vec<String>,
}

impl Context {
    pub fn apply(&mut self, update: ContextUpdate) {
        if let Some(cwd) = update.cwd {
            self.cwd = cwd;
        }

        if let Some(files) = update.files {
            self.files = files;
        }

        if let Some(history) = update.history {
            self.history = history;
        }
    }
}
impl Default for Context{

    fn default()->Self{
        Self{
            cwd:env::current_dir()
                .expect("failed to get cwd")
                .display()
                .to_string(),
            history:Vec::new(),
            files:Vec::new()
        }
    }
}
pub struct Terminal {
    pub state:Box<dyn TerminalState>,
    pub cmd_line:CmdLineState,
    pub context:Context,
}

impl Terminal{

    pub fn switch_state_to(&mut self ,state:TermState){
        match state {
            TermState::Cmdline => {
                self.state = Box::new(CmdState);
            }
            TermState::Pipe =>{
                self.cmd_line.clear();
                self.state = Box::new(PipeState);
            }
        }
    }
    pub fn update_context(&mut self , new_ctx:ContextUpdate){
        self.context.apply(new_ctx);
    }

    pub fn flush(&self, output: Vec<u8>) {
        let mut out = stdout();
        
        out.write_all(&output).unwrap();
        out.flush().unwrap();
    }

    pub fn send_pty(&self , pty:&std::sync::mpsc::Sender<Vec<u8>> , input:Vec<u8>){
        pty.send(input).unwrap();
    }

    pub fn get_ctx(&self)->&Context{
        &self.context
    }
    pub fn get_cmdline(&mut self)->&mut  CmdLineState{
        &mut self.cmd_line
    }

}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            state:Box::new(CmdState),
            cmd_line:CmdLineState::default(),
            context:Context::default()
        }
    }
}
pub struct TerminalView{
    pub user_buffer:String,
    pub suggestion_buffer:String,
    pub cwd:String,
    pub cursor:usize,
}

impl From<(&Context, &CmdLineState)> for TerminalView {
    fn from((ctx, cmd): (&Context, &CmdLineState)) -> Self {
        TerminalView{
            user_buffer:cmd.buffer.user_buffer.clone(),
            suggestion_buffer:cmd.buffer.suggestion_buffer.clone(),           
            cwd:ctx.cwd.clone(),
            cursor:cmd.buffer.cursor,
        }
    }
}