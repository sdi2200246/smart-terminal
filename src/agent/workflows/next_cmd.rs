use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::agent::agents::Agent;
use crate::agent::error::AgentError;
use crate::agent::archtectures::react::ReactLoop;
use crate::core::llm_client::LLMProvider;
use crate::core::session::{Model, ModelName};
use crate::utils::FlatSchema;

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum Reversibility {
    Full,
    Mostly,
    Partial,
    Hard,
    Irreversible,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct NextCommand {
    /// The exact shell command to run. No backticks, no `$ ` prefix.
    pub cmd: String,
    /// One short sentence describing what the command does and any assumption made. Under 15 words.
    pub man: String,
    /// How reversible the command is, given the current environment.
    pub scale: Reversibility,
}
impl FlatSchema for NextCommand {}

#[derive(Debug, Default)]
pub struct LoadedMemories {
    pub entries: Vec<String>,
}
#[derive(Debug, Default)]
pub struct MemoryUpdates {
    pub entries: Vec<String>,
}

pub struct NextCmd<'a, P: LLMProvider> {
    runner: &'a mut ReactLoop<P>,
}

impl<'a, P: LLMProvider> NextCmd<'a, P> {
    pub fn new(runner: &'a mut ReactLoop<P>) -> Self {
        Self { runner }
    }

    pub async fn run(
        &mut self,
        input: impl Into<String>,
    ) -> Result<NextCommand, AgentError> {

        let input = input.into();
        let prediction: NextCommand = {
            let mut predictor = Agent::cmd_predictor(
                &mut *self.runner,
                Model::deterministic(ModelName::GptOss20B),
            );
            let user_prompt = build_user_prompt(&input);
            print!("{}" , user_prompt);
            predictor.run(user_prompt).await?
        };
        Ok(prediction)
    }

    async fn load_memories(&mut self, _input: &str) -> Result<LoadedMemories, AgentError> {
        Ok(LoadedMemories::default())
    }

    async fn derive_updates(
        &mut self,
        _input: &str,
        _prediction: &NextCommand,
    ) -> Result<MemoryUpdates, AgentError> {
        Ok(MemoryUpdates::default())
    }
}

fn build_user_prompt(input: &str) -> String {
    format!("User input:\n{}", input)

}