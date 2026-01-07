use crossterm::{
    cursor::{MoveToColumn},
    style::{Print, Stylize , Color},
    terminal::{Clear, ClearType},
    queue,
};
use std::io::{stdout, Write};
use hostname;
use super::terminal::TerminalView;

pub fn redraw_command_line(view:TerminalView) {
    let cwd = view.cwd;
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let hostname = hostname::get().unwrap().to_string_lossy().to_string();

    let mut out = stdout();
    queue!(out, MoveToColumn(0)).unwrap();

    queue!(out, Clear(ClearType::CurrentLine)).unwrap();

    queue!(
        out,
        Print("(agent)".with(Color::Rgb { r: 255, g: 165, b: 0 })),
        Print(format!("{}@{}:{}$ ", user, hostname, cwd)),
        Print(view.user_buffer)
    )
    .unwrap();

    out.flush().unwrap();
}

pub fn render_terminal(view:TerminalView){
    redraw_command_line(view);
}