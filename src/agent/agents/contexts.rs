use serde::{Serialize};
use std::env;

#[derive(Serialize, Debug)]
pub struct ToolVersions {
    zsh: String,
    bash: String,
    awk: String,
    sed: String,
    grep: String,
    find: String,
}

impl ToolVersions {
    pub fn gather() -> Self {
        Self {
            zsh:  get_version("zsh" , "--version"),
            bash: get_version("bash", "--version"),
            awk: get_version("awk", "--version"),
            sed: get_version("sed", "--version"),
            grep: get_version("grep", "--version"),
            find: get_version("find", "--version"),
        }
    }
}
fn get_version(cmd: &str, flag: &str) -> String {
    std::process::Command::new(cmd)
        .arg(flag)
        .output()
        .ok()
        .and_then(|o| {
            let stdout = String::from_utf8(o.stdout).ok().unwrap_or_default();
            let stderr = String::from_utf8(o.stderr).ok().unwrap_or_default();
            let out = if !stdout.is_empty() { stdout } else { stderr };
            out.lines().next().map(|l| l.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}


#[derive(Serialize, Debug)]
pub struct ShellEnv{
    /// The shell currently used by the terminal.
    shell: String,

    ///All supported tools and version you must use for you predictions.
    shell_tools:ToolVersions,

    /// The operating system the agent is running on (linux, macos, windows).
    os: &'static str,

    /// The current working directory from which commands will be executed.
    cwd: String,

    /// Top-level entries in the current working directory.
    cwd_contents:Vec<String>,

     /// Recent terminal commands executed by the user (most recent last).
    history: Vec<String>,
}

impl ShellEnv{
    pub fn gather() -> Self {
         let shell_tools= ToolVersions::gather();

        let shell = env::var("SHELL")
            .ok()
            .and_then(|s| s.split('/').last().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let os = std::env::consts::OS;

        let cwd = env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let mut history = Self::gather_history();
        if history.len() > 10 {
            history.drain(0..history.len() - 10);
        }
        let cwd_contents = std::fs::read_dir(&cwd)
            .ok()
            .map(|entries| {
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        // append / to directories so the model can tell them apart
                        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            format!("{name}/")
                        } else {
                            name
                        }
                    })
                    .collect();
                names.sort();
                names
            })
            .unwrap_or_default();

        Self {
            shell,
            shell_tools,
            os,
            cwd,
            cwd_contents,
            history
        }
    }
   fn gather_history() -> Vec<String> {
    // Check the environment variable pushed by the shell script
    std::env::var("AI_CONTEXT_HISTORY")
        .unwrap_or_default()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
    }
}



#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_gather_context_from_env() {
        // Manually push the "fake" history into the test process
      unsafe {
        std::env::set_var("AI_CONTEXT_HISTORY", "ls\ncd src\ncargo build");
    }
        
        let history = ShellEnv::gather_history();
        
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "ls");
    }
    #[tokio::test]
    async fn test_gather_context() {
        let ctx = ShellEnv::gather();
        println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
    }
}
