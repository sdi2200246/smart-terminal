use crate::agent::request::{AgentRequest,AgentPolicy , AgentIntent , AgentMode};
use crate::interfaces::capability::{ToolNames};
use crate::utils::FlatSchema;
use crate::cli::context::shell::ShellEnv;
use schemars::JsonSchema;
use serde::Deserialize;

pub struct Policy{}

impl Policy {
    pub fn select_policy(itend: &AgentIntent) -> Box<dyn AgentPolicy> {
        match itend.mode {
            AgentMode::Align => Box::new(AlignPolicy),
            _=> Box::new(AutoPolicy)

        }
    }
}

#[derive(JsonSchema , Deserialize )]
pub struct Script{
    ///Shell executable programm.
    pub script:String
}

impl FlatSchema for Script {}

struct AutoPolicy;

impl AgentPolicy for AutoPolicy {
    fn create_req(&self , itend:AgentIntent)->AgentRequest{

        let terminal_ctx = ShellEnv::gather();
        AgentRequest::builder()
            .tools(vec![ToolNames::ReadDir ,ToolNames::GitLog , ToolNames::GitDiffStaged])
            .contract(Script::schema())
            .with_context(&terminal_ctx)
            .with_system_promt(AUTO_SYSTEM_POLICY.into())
            .with_user_promt(itend.prompt)
    }
}

struct AlignPolicy;

impl AgentPolicy for AlignPolicy{

    fn create_req(&self , itend:AgentIntent)->AgentRequest{

        let terminal_ctx = ShellEnv::gather();
        AgentRequest::builder()
            .tools(vec![ToolNames::AskUser, ToolNames::ReadDir])
            .contract(Script::schema())
            .with_context(&terminal_ctx)
            .with_system_promt(ALIGN_SYSTEM_POLICY.into())
            .with_user_promt(itend.prompt)
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
- the supported bash tools that you must use.
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
