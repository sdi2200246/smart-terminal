use std::path::{PathBuf};
use super::traits::{Context};
use indoc::indoc;

#[derive(Debug)]
pub struct DirsState{
    cwd:PathBuf,
    files:Vec<String>,
    cmd_history:Vec<String>,
    cmd_line:String,

}
impl DirsState{

    pub fn new(cwd:PathBuf , files:Vec<String> , cmd_history:Vec<String> , cmd_line:String)->DirsState{
        DirsState {cwd, files, cmd_history , cmd_line }
    }

    pub fn update_state(&mut self , cmd_line:String)->Result<(), std::io::Error>{

        let cwd = std::env::current_dir()?;

        let entries = std::fs::read_dir(&cwd)?;

        let newfiles = entries.filter_map(|e| {
                let entry = e.ok()?;                       
                let ft = entry.file_type().ok()?;          
                if !ft.is_file() { return None; }
                Some(entry.path().to_string_lossy().to_string())
        });
        
        self.cwd = cwd;
        for file in newfiles {
            match self.files.binary_search(&file) {
                Ok(_) => {}             
                Err(pos) => self.files.insert(pos, file),
            }
        }
        self.cmd_line = cmd_line;
        Ok(())
    }
}
impl Default for DirsState {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap(),
            files: Vec::new(),
            cmd_history: Vec::new(),
            cmd_line :"".to_string(),
        }
    }
}
impl Context for DirsState{
    fn to_context_string(&self) -> String {
        let files_str = self.files.join("\n");
        let commnads_str = self.cmd_history.join("\n");
        format!(
            indoc! {"
            Current Directory: 
            {}
            
            Current discoverred Repository Files:
            {}

            Terminal Commands History:
            {}

            Command Line State:
            {}
            "},
            self.cwd.display(),
            files_str,
            commnads_str,
            self.cmd_line,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_state_uptade_state(){
        use std::env;
        let mut state = DirsState::default();
        state.update_state("".to_string()).unwrap();
        env::set_current_dir("src").unwrap();
        state.update_state("".to_string()).unwrap();
        for f in state.files{
            println!("{f}");
        }
    }
     #[test]
     fn dirs_state_to_context(){
        let mut state = DirsState::default();
        state.update_state("".to_string()).unwrap();
        println!("{}", state.to_context_string());
    
    }
    #[test]
    fn lenvec(){
        let v = vec!["iasonas","kostis","gay"];
        println!("{}" , v.len());
        let mut newv:Vec<String> = v.iter().filter_map(|f| {
            Some(f.len().to_string())
        }).collect();

        print!("{:?}" , v);
    }
}

