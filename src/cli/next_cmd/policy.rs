use crate::agent::request::{AgentRequest , AgentPolicy , AgentIntent};
use crate::core::capability::{ToolNames};
use crate::utils::FlatSchema;
use crate::cli::context::shell::ShellEnv;
use schemars::{JsonSchema};
use serde::{Deserialize};

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
impl FlatSchema for NextCommand {}


struct DefaultPolicy;

impl AgentPolicy for DefaultPolicy {
    fn create_req(&self , itend:AgentIntent)->AgentRequest{

        let terminal_ctx = ShellEnv::gather();
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
    fn print_command_schema() {
        println!("{}", serde_json::to_string_pretty(&NextCommand::schema()).unwrap());
    }
}
