use std::{process::Command};
use std::io;
use std::path::PathBuf;

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
impl TabState{

    pub fn clear_state(&mut self){
        self.mode = TabMode::Cleared;
        self.candidates.clear();
        self.current_option = 0;
    }

    pub fn get_tab_candidate(&mut self)->&str{
        match self.mode{
            TabMode::Cleared =>{
                self.mode = TabMode::Cycling;
                return &self.candidates[0];
            }

            TabMode::Cycling => {
                self.current_option = (self.current_option + 1) % self.candidates.len();
                return &self.candidates[self.current_option];
            } 
            _=> ""  //impliment logic behinfd this.
        }
    }
    pub fn run_tab(&self, buffer: &str , cwd:&String) -> io::Result<Vec<String>> {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("autocomplete.sh");

        let output = Command::new("bash")
            .arg(&script_path)
            .arg(buffer)
            .current_dir(cwd)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let suggestions = stdout
            .lines()
            .map(|s| s.to_string())
            .collect();
        Ok(suggestions)
    }

}

impl Default for TabState{

    fn default()->Self{
        Self{
            mode:TabMode::Cleared,
            candidates:Vec::new(),
            current_option:0
        }
    }
}