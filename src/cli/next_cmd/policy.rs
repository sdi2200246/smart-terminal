use crate::agent::request::{AgentRequest , AgentPolicy , AgentIntent};
use crate::interfaces::capability::{ToolNames , ToolArgs};

use schemars::JsonSchema;
use serde::{Serialize , Deserialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Policy{}

impl Policy {

    pub fn select_policy() -> Box<dyn AgentPolicy> {
        Box::new(DefaultPolicy)
    }
}

#[derive(JsonSchema , Deserialize )]
pub struct Command{
    ///Shell executable command.
    pub cmd:String,
    ///Very compressed description of the shell command
    pub man:String
}

impl ToolArgs for Command {}

#[derive(Serialize, Debug)]
pub struct TerminalContext {
    /// The shell currently used by the terminal (e.g. bash, zsh, fish).
    shell: String,

    /// The operating system the agent is running on (linux, macos, windows).
    os: &'static str,

    /// The current working directory from which commands will be executed.
    cwd: String,

     /// Recent terminal commands executed by the user (most recent last).
    history: Vec<String>,
}

impl TerminalContext {
    pub fn gather() -> Self {

        let shell = env::var("SHELL")
            .ok()
            .and_then(|s| s.split('/').last().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let os = std::env::consts::OS;

        let cwd = env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let history = Self::load_shell_history(&shell, 20);

        Self {
            shell,
            os,
            cwd,
            history
        }
    }
     fn load_shell_history(shell: &str, limit: usize) -> Vec<String> {
        let home = env::var("HOME").unwrap_or_default();

        let history_path = match shell {
            "bash" => PathBuf::from(format!("{home}/.bash_history")),
            "zsh" => PathBuf::from(format!("{home}/.zsh_history")),
            _ => return Vec::new(),
        };

        let content = fs::read_to_string(history_path).unwrap_or_default();

        let mut lines: Vec<String> = content
            .lines()
            .map(|l| {
                // zsh history has timestamps like ": 1670000000:0;git status"
                if shell == "zsh" {
                    l.split(';').last().unwrap_or(l).to_string()
                } else {
                    l.to_string()
                }
            })
            .collect();

        if lines.len() > limit {
            lines = lines.split_off(lines.len() - limit);
        }

        lines
    }

}

struct DefaultPolicy;

impl AgentPolicy for DefaultPolicy {
    fn create_req(&self , itend:AgentIntent)->AgentRequest{

        let terminal_ctx = TerminalContext::gather();
        AgentRequest::builder()
            .tools(vec![ToolNames::GitLog , ToolNames::GitDiffStaged , ToolNames::ReadDir])
            .contract(Command::schema())
            .with_context(&terminal_ctx)
            .with_system_promt(DEFAULT_SYSTEM_POLICY.into())
            .with_user_promt(itend.prompt
            )
    }
}

pub const DEFAULT_SYSTEM_POLICY: &str = "You are an expert shell command completion agent.

You will receive a JSON context object describing the environment — OS, shell, cwd, and the user's input.
The input is either a partial command or a natural language description of what the user wants to do.

STRATEGY:
Infer the user's intent from their input and the environment.
When the input is ambiguous, use your tools to observe the current state of the environment and let it resolve the ambiguity — the environment almost always tells you what the user is about to do next.
Only call tools that are relevant to the command being completed.

COMPLETION:
Always complete the command fully — never return a partial command or a placeholder.
Pick the most probable interpretation based on what you observe.
A syntactically complete command is not enough — it must be semantically complete.
example :`git commit` without a `-m` is not a valid completion.
Always use tools to fill in arguments, flags, and values that the user would have to type anyway.

OUTPUT:
You MUST submit your answer using the final_answer tool — this is the only valid output.
Return a single runnable command. No explanation, no alternatives, no comments.";

#[tokio::test]
async fn test_gather_context() {
    let ctx = TerminalContext::gather();
    println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
}