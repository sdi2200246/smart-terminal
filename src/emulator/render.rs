use crossterm::{
    cursor::MoveToColumn,
    queue,
    style::{Color, Print , Stylize},
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};
use hostname;
use super::terminal::TerminalView;


pub fn redraw_command_line(view: TerminalView) {
    let mut out = stdout();

    let cwd = &view.cwd;
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    
    let hostname = hostname::get().unwrap().to_string_lossy().to_string();
    let agent = "(agent)".with(Color::Rgb { r: 255, g: 165, b: 0 });
    let prompt_tail = format!("{}@{}:{}$ ", user, hostname, cwd);

    queue!(
        out,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(agent),
        Print(&prompt_tail),
        Print(&view.user_buffer),
    )
    .unwrap();

    let prompt_cols =
        "(agent)".chars().count() as u16 +
        prompt_tail.chars().count() as u16;

    let buf_cols = view.user_buffer[..view.cursor].chars().count() as u16;
    queue!(out,MoveToColumn(prompt_cols + buf_cols)).unwrap();
    out.flush().unwrap();

    if view.user_buffer.contains('\r'){
        queue!(out,MoveToColumn(0),).unwrap();
    }

}
pub fn render_terminal(view:TerminalView){
    redraw_command_line(view);
}