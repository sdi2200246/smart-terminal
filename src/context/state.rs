use std::path::{PathBuf};
use super::traits::Context;
use indoc::indoc;

#[derive(Debug)]
pub struct DirsState{
    pub cwd:PathBuf,
    pub files:Vec<String>,
}
impl DirsState{

    pub fn update_state(&mut self)->Result<(), std::io::Error>{

        let cwd = std::env::current_dir()?;

        let entries = std::fs::read_dir(&cwd)?;

        let newfiles = entries.filter_map(|e| {
                let entry = e.ok()?;                       
                let ft = entry.file_type().ok()?;          
                if !ft.is_file() { return None; }
                Some(entry.file_name().to_string_lossy().to_string())
        });


        self.cwd = cwd;
        for file in newfiles {
            match self.files.binary_search(&file) {
                Ok(_) => {}             
                Err(pos) => self.files.insert(pos, file),
            }
        }

        Ok(())
    }
}
impl Default for DirsState {
    fn default() -> Self {
        Self {
            cwd: PathBuf::new(),
            files: Vec::new(),
        }
    }
}
impl Context for DirsState{
    fn from_context_to_string(&self) -> String {
        let files_str = self.files.join("\n");
        format!(
            indoc! {"
            Current Directory: {}
            
            Current discoverred Repository Files:
            {}
            "},
            self.cwd.display(),
            files_str,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_state_uptade_state(){
        use std::env;
        env::set_current_dir("src").unwrap();

        let mut state = DirsState::default();
        state.update_state();
        println!("{:?}", state);
    
    }
     #[test]
     fn dirs_state_to_context(){
        let mut state = DirsState::default();
        println!("{}", state.from_context_to_string());
    
    }
}

