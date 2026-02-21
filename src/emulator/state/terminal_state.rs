use super::cmd_line::CmdLineState;
use super::terminal::{Context , TerminalAction , TermState};

pub trait TerminalState:Send{
    fn handle_input(&mut self,event: UseCase,ctx: &Context,cmdline: &mut CmdLineState,) -> Vec<TerminalAction>;
    fn handle_output(&mut self,bytes: &[u8],) -> Vec<TerminalAction>;
    fn state(&mut self)->TermState;
}

#[derive(Debug, PartialEq)]
pub enum UseCase{
    JobControl(u8),
    CmdExecution,
    LineDicipline(Key),
    Passthrough(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum Key{
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Char(String),
}