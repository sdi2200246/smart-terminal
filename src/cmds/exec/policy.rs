use crate::agent::request::AgentRequest;
use crate::agent::responce::AgentResponse;
use crate::interfaces::capability::{ToolNames , ToolArgs};
use crate::cmds::cli::ExecArgs;
use schemars::JsonSchema;
use serde::{Serialize , Deserialize};
use std::env;
use tokio::sync::mpsc::Sender;

pub struct Policy{}

impl Policy {

    pub fn select_policy(args: &ExecArgs) -> Box<dyn AgentPolicy> {
        let policy: Box<dyn AgentPolicy> = if args.align {
            Box::new(AlignPolicy)
        } else {
            Box::new(AutoPolicy)
        };

        policy
    }
}

pub trait AgentPolicy {
    fn create_req(&self, args: ExecArgs,response_tx: Sender<AgentResponse>) -> AgentRequest;
}

#[derive(JsonSchema , Deserialize )]
pub struct Script{
    ///Shell executable programm.
    pub script:String
}

impl ToolArgs for Script {}

#[derive(Serialize, Debug)]
pub struct TerminalContext {
    /// The shell currently used by the terminal (e.g. bash, zsh, fish).
    shell: String,

    /// The operating system the agent is running on (linux, macos, windows).
    os: &'static str,

    /// The current working directory from which commands will be executed.
    cwd: String,
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

        Self {
            shell,
            os,
            cwd,
        }
    }
}

struct AutoPolicy;

impl AgentPolicy for AutoPolicy {
    fn create_req(&self , args:ExecArgs , response_tx:Sender<AgentResponse>)->AgentRequest{

        let terminal_ctx = TerminalContext::gather();
        AgentRequest::builder(response_tx)
            .tools(vec![ToolNames::GitLog , ToolNames::GitDiffStaged])
            .contract(Script::schema())
            .with_context(&terminal_ctx)
            .with_system_promt(AUTO_SYSTEM_POLICY.into())
            .with_user_promt(args.prompt)
    }
}

struct AlignPolicy;

impl AgentPolicy for AlignPolicy{

    fn create_req(&self , args:ExecArgs , response_tx:Sender<AgentResponse>)->AgentRequest{

        let terminal_ctx = TerminalContext::gather();
        AgentRequest::builder(response_tx)
            .tools(vec![ToolNames::AskUser, ToolNames::GitLog , ToolNames::GitDiffStaged])
            .contract(Script::schema())
            .with_context(&terminal_ctx)
            .with_system_promt(ALIGN_SYSTEM_POLICY.into())
            .with_user_promt(args.prompt)
    }
}



pub const AUTO_SYSTEM_POLICY: &str = "You are an expert shell execution agent embedded in a developer's shell.

You will receive a JSON context object describing the environment.
Use it to inform every decision — OS, shell, cwd, and user intent are all there.

STRATEGY:
Use your tools to build a complete picture before acting.
Inspect, validate, and reason before committing to any script.
When you have enough information to produce a correct script, stop looping.

AMBIGUITY:
If the intent is unclear, make the safest reasonable assumption and proceed.

SCOPE:
Never operate outside the cwd unless explicitly stated.
Never modify system files.

OUTPUT:
You MUST submit your final script using the final_answer tool — this is the only valid way to produce output.
The script must be complete and executable as-is.
Do NOT include comments or explanations in the script — code only.";


pub const ALIGN_SYSTEM_POLICY: &str = "You are an expert shell execution agent embedded in a developer's terminal.

Your primary responsibility is to align with the user's intent before generating any script.

You will receive a JSON context object describing the environment such as:
- operating system
- shell
- current working directory

However, before producing any script you MUST ensure the user's intent is fully understood.

ALIGNMENT RULES:
Always alignment before action.
You MUST always align with the user first using the ask_user tool.

QUESTION STYLE:
Questions must be:
- short
- simple
- easy to answer
- focused on one missing piece of information

Never ask multiple questions at once.

Avoid overwhelming the user.

Ask only the single most important question needed to move forward.

INTERACTION LOOP:
1. Ask one short question.
2. Wait for the user's answer.
3. Re-evaluate if you have enough information.
4. Repeat if necessary.

TOOLS:
Use the ask_user tool for all user questions.

SCRIPT GENERATION:
Only generate a script once you are confident about the user's intent.

OUTPUT RULES:
When you are fully aligned with the user's goal, submit the final script using the final_answer tool.

The script must be executable as-is.

Do not include explanations or comments in the script — code only.";