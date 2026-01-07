use super::cmd_line::CmdLineState;
use super::terminal::{Context , InputEvent , TerminalAction};

pub trait TerminalState:Send{
   fn handle_input(
        &mut self,
        event: InputEvent,
        ctx: &Context,
        cmdline: &mut CmdLineState,
    ) -> Vec<TerminalAction>;

    fn handle_output(
        &mut self,
        bytes: &[u8],
    ) -> Vec<TerminalAction>;
}
