use indoc::indoc;
use crate::context::traits::Context;

pub struct Promt<T:Context>{
    inst:String,
    promt:String,
    context:T,
    //options:ENUM SOME OPTIONS.
}
impl<T: Context> Promt<T> {

    pub fn new(inst: String, promt: String, context: T) -> Self {
        Promt { inst, promt, context }
    }

   pub fn to_smartlog_prompt(&self , format:String) -> String {
        format!(
            indoc! {"
            ### SYSTEM ###
            {}

            ### TASK ###
            {}

            ### CONTEXT ###
            {}

            ### FORMAT ###
            {}
            "},
            self.inst,
            self.promt,
            self.context.to_context_string(),
            format,
        )
    }    
}

#[cfg(test)]
mod tests {
    use super::Promt;
    use crate::chat::NextCmd;
    use crate::context::state::DirsState;
    use crate::context::traits::LLMforamt;
    #[test]
    fn test_to_smartlog_prompt() {
        let suspicious_code = r#"for (int i = 0 ; i < 10 ; i++){}"#;

        let p = Promt::new(
                "You are a Rust".to_string(),
                "Analyze".to_string(),
                suspicious_code.to_string());
        
        let output = p.to_smartlog_prompt("jason".to_string());
        assert!(output.contains("### SYSTEM ###"));
        assert!(output.contains("You are a Rust"));

        assert!(output.contains("### TASK ###"));
        assert!(output.contains("Analyze"));

        assert!(output.contains("### CONTEXT ###"));
        assert!(output.contains("for (int i = 0 ; i < 10 ; i++){}"));

        assert!(output.contains("### FORMAT ###"));     
    }


    fn fake_state() -> DirsState {
        use std::path::PathBuf;
        let cwd = PathBuf::from("/home/jason/Github_Repos/smart-terminal");

        let files = vec![
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "src/main.rs".to_string(),
            "src/chat/mod.rs".to_string(),
            "src/context/state.rs".to_string(),
            "README.md".to_string(),
        ];

        let cmd_history = vec![
            "cd src".to_string(),
            "ls".to_string(),
            "git status".to_string(),
            "cargo test".to_string(),
            "cargo run".to_string(),
        ];

        DirsState::new(cwd, files, cmd_history , "car".to_string())
    }

    #[test]
    fn test_to_smartlog_prompt2(){
        let state = fake_state();
        let p = Promt::new(
                "You are a Rust".to_string(),
                "Analyze".to_string(),
                state);
        
        let output = p.to_smartlog_prompt(NextCmd::to_json_format());
        println!("{}" , output);
        assert!(output.contains("### SYSTEM ###"));
        assert!(output.contains("You are a Rust"));

        assert!(output.contains("### TASK ###"));
        assert!(output.contains("Analyze"));

        assert!(output.contains("### CONTEXT ###"));
        assert!(output.contains("### FORMAT ###"));     
    }

}

