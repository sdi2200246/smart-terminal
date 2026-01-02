use crossterm::{
    cursor::{MoveToColumn},
    style::{Print, Stylize , Color},
    terminal::{Clear, ClearType},
    queue,
};
use std::io::{stdout, Write};
use std::env;
use hostname;

#[derive(PartialEq , Debug)]
pub enum RenderActions{
    Tab(String),
    Ghost,
    Backspace,
    Char(char),
    Promt(String),
    PTYoutput(Vec<u8>),
}


pub fn draw_prompt(cwd:String) {
    let cwd = cwd;
    let user = env::var("USER").unwrap_or("user".into());
    let hostname = hostname::get().unwrap().to_string_lossy().to_string();

    let mut out = stdout();

    queue!(out, MoveToColumn(0)).unwrap();
    queue!(out, Clear(ClearType::CurrentLine)).unwrap();

    queue!(out, Print("(agent)".with(Color::Rgb { r: 255, g: 165, b: 0 }))).unwrap();

    let prompt = format!("{}@{}:{}$ ", user, hostname, cwd);
    queue!(out, Print(prompt)).unwrap();

    out.flush().unwrap();
}
pub fn draw_character(c:char){
    let mut out = stdout();
    queue!(out, Print(c)).unwrap();
    out.flush().unwrap();
    
}

pub fn draw_backspace(){
    let mut out = stdout();
    queue!(out, Print("\x08 \x08")).unwrap();
    out.flush().unwrap();
}

pub fn redraw_command_line(buffer: String) {
    let cwd = std::env::current_dir().unwrap().display().to_string();
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let hostname = hostname::get().unwrap().to_string_lossy().to_string();

    let mut out = stdout();

    // Move cursor to start of line
    queue!(out, MoveToColumn(0)).unwrap();

    // Clear entire line
    queue!(out, Clear(ClearType::CurrentLine)).unwrap();

    // Re-print prompt
    queue!(
        out,
        Print("(agent)".with(crossterm::style::Color::Rgb { r: 255, g: 165, b: 0 })),
        Print(format!("{}@{}:{}$ ", user, hostname, cwd)),
        Print(buffer)
    )
    .unwrap();

    out.flush().unwrap();
}



pub fn render_handler(action:RenderActions){
    match action{
        RenderActions::Backspace => draw_backspace(),
        
        RenderActions::Char(c) => draw_character(c),
        
        RenderActions::Tab(buffer) => redraw_command_line(buffer),

        RenderActions::Promt(cwd) => draw_prompt(cwd), 

        _=>{}
    }

}