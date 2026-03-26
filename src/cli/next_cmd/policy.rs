use crate::agent::request::{AgentRequest , AgentPolicy , AgentIntent};
use crate::interfaces::capability::{ToolNames};
use crate::utils::FlatSchema;

use schemars::{JsonSchema};
use serde::{Serialize , Deserialize};
use std::env;

pub struct Policy{}

impl Policy {

    pub fn select_policy() -> Box<dyn AgentPolicy> {
        Box::new(DefaultPolicy)
    }
}
#[derive(JsonSchema , Deserialize , Debug )]
pub enum Reversibility {
    Full,        // 0-2
    Mostly,      // 3-4
    Partial,     // 5-6
    Hard,        // 7-8
    Irreversible // 9-10
}


#[derive(JsonSchema , Deserialize )]
pub struct NextCommand{
    ///Shell executable command.
    pub cmd:String,
    ///Very compressed description of the shell command
    pub man:String,
    /// How reversible the command is given the current environment. 
    /// (ex. A tracked file deletion is recoverable via git, an untracked one is gone forever) Use the available context to inform the classification.
    pub scale:Reversibility
}


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



impl FlatSchema for NextCommand {}

#[derive(Serialize, Debug)]
pub struct TerminalContext {
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

impl TerminalContext {
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
        if history.len() > 5 {
            history.drain(0..history.len() - 5);
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

struct DefaultPolicy;

impl AgentPolicy for DefaultPolicy {
    fn create_req(&self , itend:AgentIntent)->AgentRequest{

        let terminal_ctx = TerminalContext::gather();
        AgentRequest::builder()
            .tools(vec![ToolNames::GitLog , ToolNames::GitDiffStaged])
            .contract(NextCommand::schema())
            .with_system_promt(DEFAULT_SYSTEM_POLICY.into())
            .with_user_promt(itend.prompt)
            .with_context(&terminal_ctx)
    }
}

pub const DEFAULT_SYSTEM_POLICY: &str = "You are a shell command completion engine. Output a single, immediately runnable shell command.

CONTEXT
You receive a context object: shell and os (determine syntax and flag compatibility), cwd (resolve paths, infer project type), history (last commands — infer intent and workflow).

BUFFER POLICY
Non-empty: complete or translate into a full command. Do not change the user's approach, extend it combined with history.
Empty: derive intent from history. Predict the most logical next command.

TOOLS — git commands only
GitLog: use when completing git commit (match message style), git revert, or any command referencing past commits.
GitDiffStaged: use when completing git commit (write accurate -m from staged content) or anything acting on staged content.
Never call either tool for non-git commands.

RULES
- No placeholders — every token must be real and resolved.
- Syntactically valid for the shell and os in context.
- Semantically complete — must run without prompting for further input.
- OS-aware flags — check context.os before emitting flags that differ across systems.

OUTPUT
Call final_answer with cmd, man, and reversibility.";

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_gather_context_from_env() {
        // Manually push the "fake" history into the test process
      unsafe {
        std::env::set_var("AI_CONTEXT_HISTORY", "ls\ncd src\ncargo build");
    }
        
        let history = TerminalContext::gather_history();
        
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "ls");
    }
    #[tokio::test]
    async fn test_gather_context() {
        let ctx = TerminalContext::gather();
        println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
    }

    #[test]
    fn print_command_schema() {
        println!("{}", serde_json::to_string_pretty(&NextCommand::schema()).unwrap());
    }
}
