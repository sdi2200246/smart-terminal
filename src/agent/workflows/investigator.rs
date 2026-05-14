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
pub struct PlanStep {
    /// Concrete investigation step: what to look at, what command to run, or what file to read. One file, one command, or one directory per step.
    pub action: String,
    /// Why this step advances the answer to the user's question.
    pub rationale: String,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct Plan {
    /// One-line restatement of the user's question.
    pub goal: String,
    /// ordered, atomic investigation steps grounded in directory paths verified to exist.
    pub steps: Vec<PlanStep>,
}
impl FlatSchema for Plan {}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct Report {
    ///A direct report answering the user's question. Not a description of what was done.
    pub report: String,
}
impl FlatSchema for Report {}
pub struct Investigator<'a, P: LLMProvider> {
    runner: &'a mut ReactLoop<P>,
}

impl<'a, P: LLMProvider> Investigator<'a, P> {
    pub fn new(runner: &'a mut ReactLoop<P>) -> Self {
        Self {
            runner,
        }
    }
    pub async fn run(&mut self,question: impl Into<String>,) -> Result<(Plan, Report), AgentError> {
        let question = question.into();

        let plan: Plan = {
            let mut planner =
                Agent::planner(&mut *self.runner, Model::creative(ModelName::GptOss120B));
            planner
                .run(format!("Question:\n{}", question))
                .await?
        };

        let plan_json = serde_json::to_string_pretty(&plan).expect("plan serializes");
        let user_prompt = format!(
            "Question: {}\n\nInvestigation plan:\n{}",
            question, plan_json
        );
        let report: Report = {
            let mut executor =
                Agent::executor(&mut *self.runner, Model::creative(ModelName::GptOss120B));
            executor.run(user_prompt).await?
        };
        Ok((plan, report))
    }
}