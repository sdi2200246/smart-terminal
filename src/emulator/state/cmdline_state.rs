use super::terminal::{TermState ,TerminalAction ,ContextUpdate ,Context};
use super::terminal_state::{TerminalState,UseCase ,Key};
use super::cmd_line::CmdLineState;

pub struct CmdState;

impl TerminalState for CmdState{

    fn handle_input(&mut self , event:UseCase, ctx:&Context, cmdline: &mut CmdLineState,)->Vec<TerminalAction>{
        match event{
            UseCase::LineDicipline(key) => line_dicipline(key, cmdline, ctx),
            UseCase::JobControl(byte) => job_control(byte),
            UseCase::CmdExecution => cmd_execution(cmdline),
            _=>{vec![]}
        }
    }
    fn handle_output(&mut self, bytes:&[u8])->Vec<TerminalAction>{
        let mut actions:Vec<TerminalAction> = Vec::new();
        let output = String::from_utf8_lossy(&bytes);
        actions.push(TerminalAction::Flush(bytes.to_vec()));
        output_interpreter(&output , &mut actions);
        actions.push(TerminalAction::Render);
        return actions;
    }
    fn state(&mut self)->TermState {
        TermState::Cmdline
    }
}

fn line_dicipline(key:Key , cmdline:&mut CmdLineState , ctx:&Context) -> Vec<TerminalAction>{
    match key{
        Key::Backspace =>{
            cmdline.restore_tab_state();
            cmdline.apply_backsapce();
        }
        Key::Char(c) => cmdline.insert_char(c),

        Key::Tab =>{
            cmdline.restore_history_state();
            cmdline.apply_tab(ctx.cwd.clone());
        }
        Key::Left => cmdline.move_cursor_left(),

        Key::Right => cmdline.move_cursor_right(),

        Key::Down => cmdline.navigate_history_down(),

        Key::Up => cmdline.navigate_history_up(),
    }

    vec![TerminalAction::Render]
}


fn job_control(byte:u8)->Vec<TerminalAction>{
    vec![TerminalAction::SendPty(vec![byte])]
}

fn cmd_execution(cmdline:&mut CmdLineState)->Vec<TerminalAction>{
    let context_update = ContextUpdate{
        cwd:None,
        cmd:Some(cmdline.buffer.user_buffer.clone()),
        files:None
    };
    cmdline.history_state.add_cmd(cmdline.buffer.user_buffer.clone());
    let bytes = cmdline.buffer.to_bytes();
    cmdline.buffer.push("\r\n");
    return vec![TerminalAction::Render ,TerminalAction::SendPty(bytes) ,TerminalAction::SwitchState(TermState::Pipe) , TerminalAction::UpdateContext(context_update)];

}

fn extract_cwd(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix("__AGENT_DONE__:"))
        .map(|cwd| cwd.to_string())
}

fn output_interpreter(output:&str , actions:&mut Vec<TerminalAction>){
    if output.contains("\x1b[?1049h")|| output.contains("\x1b[?47h")|| output.contains("\x1b[?1047h"){
        actions.push(TerminalAction::SwitchState(TermState::Pipe));
    }

    if output.contains("__AGENT_DONE__") {
        if let Some(cwd) = extract_cwd(output) {
            let new_context = ContextUpdate{
                cwd:Some(cwd),
                cmd:None,
                files:None,
            };
            actions.push(TerminalAction::UpdateContext(new_context));
        }
        actions.push(TerminalAction::SwitchState(TermState::Cmdline));
    }
}


// #[cfg(test)]
// mod cmdstate_tests {
//     use super::*;

//     fn has_switch(actions: &[TerminalAction], state: TermState) -> bool {
//         actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SwitchState(s) if *s == state
//         ))
//     }
//     #[test]
//     fn cmdstate_alt_screen_enter_switches_to_pipe() {
//         let mut actions = Vec::new();
//         output_interpreter("\x1b[?1049h", &mut actions);
//         assert!(has_switch(&actions, TermState::Pipe));
//     }


//     #[test]
//     fn cmdstate_agent_done_updates_context_cwd_only() {
//         let mut actions = Vec::new();
//         output_interpreter("__AGENT_DONE__:/home/user\n", &mut actions);

//         let update = actions.iter().find_map(|a| {
//             if let TerminalAction::UpdateContext(u) = a { Some(u) } else { None }
//         }).expect("Expected UpdateContext");

//         assert_eq!(update.cwd.as_deref(), Some("/home/user"));
//         assert!(update.cmd.is_none());
//         assert!(update.files.is_none());
//     }


//     #[test]
//     fn cmdstate_agent_done_does_not_switch_to_pipe() {
//         let mut actions = Vec::new();
//         output_interpreter("__AGENT_DONE__:/tmp", &mut actions);
//         assert!(!has_switch(&actions, TermState::Pipe));
//     }

//     #[test]
//     fn cmdstate_handle_output_flushes_and_renders() {
//         let mut state = CmdState;
//         let bytes = b"hello";
//         let actions = state.handle_output(bytes);

//         assert!(actions.iter().any(|a| matches!(
//             a, TerminalAction::Flush(b) if b == bytes
//         )));
//         assert!(actions.iter().any(|a| matches!(a, TerminalAction::Render)));
//     }
// }


// #[cfg(test)]
// mod cmd_actions_tests {
//     use super::*;
//     use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

//     fn ctx() -> Context {
//         Context {
//             cwd: "/tmp".into(),
//             history: vec![],
//             files: vec![],
//         }
//     }

//     #[test]
//     fn char_input_updates_buffer_and_renders() {
//         let mut cmdline = CmdLineState::default();
//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
//             &mut cmdline,
//             &ctx(),
//         );

//         assert_eq!(cmdline.buffer.get_user_buffer(), "a");
//         assert!(actions.contains(&TerminalAction::Render));
//     }

//     #[test]
//     fn backspace_pops_buffer_and_renders() {
//         let mut cmdline = CmdLineState::default();
//         cmdline.buffer.push("a");

//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
//             &mut cmdline,
//             &ctx(),
//         );

//         assert_eq!(cmdline.buffer.get_user_buffer(), "");
//         assert!(actions.contains(&TerminalAction::Render));
//     }

//     #[test]
//     fn enter_sends_buffer_and_switches_to_pipe() {
//         let mut cmdline = CmdLineState::default();
//         cmdline.buffer.push("ls");

//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
//             &mut cmdline,
//             &ctx(),
//         );

//         assert!(actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SendPty(bytes) if bytes == b"ls\n"
//         )));

//         assert!(actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SwitchState(TermState::Pipe)
//         )));
//         assert_eq!(cmdline.buffer.get_user_buffer(), "");
//     }
//     #[test]
//     fn enter_executes_command_updates_context_and_switches_to_pipe() {
//         let mut cmdline = CmdLineState::default();
//         cmdline.buffer.push("ls -la");

//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
//             &mut cmdline,
//             &ctx(),
//         );
//         assert!(actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SendPty(bytes) if bytes == b"ls -la\r"
//         )));
//         let ctx_update = actions.iter().find_map(|a| {
//             if let TerminalAction::UpdateContext(u) = a {
//                 Some(u)
//             } else {
//                 None
//             }
//         }).expect("Expected ContextUpdate action");

//         assert_eq!(ctx_update.cmd.as_deref(), Some("ls -la"));
//         assert!(actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SwitchState(TermState::Pipe)
//         )));
//         assert!(actions.contains(&TerminalAction::Render));
//     }


//     #[test]
//     fn ctrl_char_sends_control_byte_and_clears_buffer() {
//         let mut cmdline = CmdLineState::default();
//         cmdline.buffer.push("test");

//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
//             &mut cmdline,
//             &ctx(),
//         );

//         assert!(actions.iter().any(|a| matches!(
//             a,
//             TerminalAction::SendPty(bytes) if *bytes == vec![3]
//         )));

//         assert_eq!(cmdline.buffer.get_user_buffer(), "");
//     }

//     #[test]
//     fn tab_single_match_completes_and_renders() {
//         let mut cmdline = CmdLineState::default();
//         cmdline.buffer.push("gre");

//         let actions = cmd_actions(
//             KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
//             &mut cmdline,
//             &ctx(),
//         );

//         assert_eq!(cmdline.buffer.get_user_buffer(), "grep");
//         assert!(actions.contains(&TerminalAction::Render));
//     }
// }

