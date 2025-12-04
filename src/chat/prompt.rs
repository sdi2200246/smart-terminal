use indoc::indoc;

pub struct Promt{
    inst:String,
    promt:String,
    context:String,
    //options:ENUM SOME OPTIONS.
}

impl Promt{

    pub fn new(inst:String , promt:String)->Promt{
        return Promt{
            inst,
            promt,
            context:"".to_string(),
        }
    }
     pub fn with_context(mut self, ctx: String) -> Promt {
        self.context = ctx;
        self
    }

   pub fn to_smartlog_prompt(&self) -> String {
        format!(
            indoc! {"
            ### SYSTEM ###
            {}

            ### TASK ###
            {}

            ### CONTEXT ###
            {}

            ### FORMAT ###
            Return strictly only a JSON sructure with ONLY !ONE! object on the first detection you make to fill my systems struct with:
            - message: string
            - kind: FATAL_ERR | LOGICAL_ERR | OPT_PROPOSITION | RUNTIME_ERR | NONE
            -line: i64

            If the code contains no real issues, return exactly this JSON:

            -message: No issues detected ,
            -kind:NONE, 
            -line: -1,

            DONT ADD ANY OTHER TEXT!
            Reporting an issue that does not clearly exist in the code is considered a FAILURE!!."
            },
            self.inst,
            self.promt,
            self.context,
        )
    }    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_smartlog_prompt() {
        let suspicious_code = r#"for (int i = 0 ; i < 10 ; i++){}"#;

        let p = Promt::new(
                "You are a Rust".to_string(),
                "Analyze".to_string()
            ).with_context(suspicious_code.to_string());
        
        let output = p.to_smartlog_prompt();
        assert!(output.contains("### SYSTEM ###"));
        assert!(output.contains("You are a Rust"));

        assert!(output.contains("### TASK ###"));
        assert!(output.contains("Analyze"));

        assert!(output.contains("### CONTEXT ###"));
        assert!(output.contains("for (int i = 0 ; i < 10 ; i++){}"));

        assert!(output.contains("### FORMAT ###"));     


    }
}

