use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::agent::archtectures::oneshot::OneShot;
use crate::agent::archtectures::react::ReactLoop;
use crate::agent::agents::{Agent , OneShotAgent};
use crate::agent::error::AgentError;
use crate::core::llm_client::LLMProvider;
use crate::core::session::{Model, ModelName};
use crate::utils::FlatSchema;

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum Shell {
    Bash,
    Posix,
    Zsh,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum ArgKind {
    Positional,
    Flag,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum ErrorStrategy {
    /// set -euo pipefail at the top; fail on any error or undefined variable.
    Strict,
    /// No global error handling; commands are allowed to fail without halting.
    Lenient,
    /// Per-command if/then handling for specific commands that may legitimately fail.
    PerCommand,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct Argument {
    /// Argument name, e.g. "target_dir" for a positional or "verbose" for a flag.
    pub name: String,
    /// One-line description of what this argument does, shown in --help output.
    pub help: String,
    /// Whether the script must fail if this argument is not provided.
    pub required: bool,
    /// Positional argument or named flag.
    pub kind: ArgKind,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct ScriptDesign {
    /// Target shell for the script.
    pub shell: Shell,
    /// One-line summary of what the script does.
    pub purpose: String,
    /// Arguments the script accepts. Empty if the script takes no input.
    pub arguments: Vec<Argument>,
    /// External commands the script depends on. Every entry must be verified to exist on the target system.
    pub dependencies: Vec<String>,
    /// Error-handling strategy.
    pub error_handling: ErrorStrategy,
    /// Files, directories, processes, or environment the script reads, writes, deletes, or modifies. Each entry should be a concrete path or named effect.
    pub side_effects: Vec<String>,
    /// True only if the script is safe to run more than once with no additional side effects beyond the first run.
    pub idempotent: bool,
}
impl FlatSchema for ScriptDesign {}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct Script {
    /// kebab-case filename with a shell-appropriate extension (.sh, .zsh).
    pub filename: String,
    /// Full script body, including shebang line.
    pub content: String,
    /// One realistic example invocation, e.g. "./backup-logs.sh /var/log".
    pub invocation_example: String,
}
impl FlatSchema for Script {}

pub struct ScriptGenerator<'a, P: LLMProvider> {
    react_runner: &'a mut ReactLoop<P>,
    one_shooter :&'a mut OneShot<P>,
}

impl<'a, P: LLMProvider> ScriptGenerator<'a, P> {
    pub fn new(reaction: &'a mut ReactLoop<P> , one_shot :&'a mut OneShot<P> ) -> Self {
        Self {
            react_runner:reaction,
            one_shooter:one_shot,
        }
    }
    pub async fn run(
        &mut self,
        prompt: impl Into<String>,
    ) -> Result<(ScriptDesign, Script), AgentError> {
        let prompt = prompt.into();

        let design: ScriptDesign = {
            let mut architect =
                Agent::architect(&mut *self.react_runner, Model::creative(ModelName::GptOss120B));
            architect
                .run(format!("Script request:\n{}", prompt))
                .await?
        };

        let design_json =
            serde_json::to_string_pretty(&design).expect("design serializes");
        let user_prompt = format!(
            "Script request: {}\n\nApproved design:\n{}",
            prompt, design_json
        );

        let script: Script = {
            let mut generator =
                OneShotAgent::script_generator(&mut *self.one_shooter);
            generator.run(user_prompt).await?
        };

        Ok((design, script))
    }
}