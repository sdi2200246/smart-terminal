use crate::agent::request::{AgentRequest , AgentPolicy , AgentIntent};
use crate::interfaces::capability::{ToolNames , ToolArgs};

use schemars::JsonSchema;
use serde::{Serialize , Deserialize};
use std::env;

pub struct Policy{}

impl Policy {

    pub fn select_policy() -> Box<dyn AgentPolicy> {
        Box::new(DefaultPolicy)
    }
}

#[derive(JsonSchema , Deserialize )]
pub struct NextCommand{
    ///Shell executable command.
    pub cmd:String,
    ///Very compressed description of the shell command
    pub man:String
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



impl ToolArgs for NextCommand {}

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

        Self {
            shell,
            shell_tools,
            os,
            cwd,
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

pub const DEFAULT_SYSTEM_POLICY: &str = r#"You are a shell command completion engine. Your sole output is a single, immediately runnable shell command.
## CONTEXT
You receive a `context` object with the following fields:

- `shell`: the active shell (e.g. zsh, bash) — determines syntax rules
- `shell_tools`: exact versions of available tools (awk, sed, grep, find, etc.) — use these versions to ensure flag compatibility
- `os`: the operating system — macOS and Linux differ on flags (e.g. `sed -i ''` vs `sed -i`, `ls -G` vs `ls --color`)
- `cwd`: the directory commands will run in — use it to resolve relative paths and infer project type
- `history`: the last 5 commands the user ran — use this to infer intent, reuse established paths, and understand workflow

## USER BUFFER
The user buffer is the user's raw input — either a partial command or a natural language description.

- If the buffer is **non-empty**: complete or translate it into a full command. Do not change the user's approach — extend it.
- If the buffer is **empty**: derive intent entirely from `history`. The user wants to continue their current workflow. Look at what they just did and predict the most logical next command.

## TOOLS
You have access to two tools. Only call them for `git` commands.

- `GitLog`: returns recent commit history — use when completing `git commit` (to match message style/format), `git revert`, `git diff HEAD~N`, or any command that references past commits
- `GitDiffStaged`: returns currently staged changes — use when completing `git commit` (to write an accurate `-m` message based on what is actually staged), `git stash`, or anything that acts on staged content

Do not call either tool for non-git commands.

## COMPLETION RULES
1. **No placeholders** — never output `<file>`, `[message]`, `YOUR_BRANCH`, or any stand-in. Every token must be real and resolved.
2. **Syntactically valid** for the shell and OS in `context`. Quotes must be balanced. Pipes must have both sides.
3. **Semantically complete** — the command must run to completion without prompting for further input:
   - `git commit` ✗ → `git commit -m "feat: add retry logic to fetch"` ✓
   - `find .` ✗ → `find . -name "*.rs" -type f` ✓
4. **OS-aware flags** — always check `context.os` before emitting flags that differ across systems.

## OUTPUT
Submit using `final_answer` with:
- `cmd`: the complete, runnable command
- `man`: one short phrase describing what the command does (not why you chose it)"#;

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
}