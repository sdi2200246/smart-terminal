use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::agents::Agent;
use crate::agent::error::AgentError;
use crate::agent::patterns::react::ReactLoop;
use crate::core::capability::Capability;
use crate::core::llm_client::LLMProvider;
use crate::core::model::{Model, ModelName};
use crate::utils::FlatSchema;

pub trait InvestigatorToolFactory {
    fn planner_tools(&self) -> Vec<Box<dyn Capability>>;
    fn executor_tools(&self) -> Vec<Box<dyn Capability>>;
}

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
    ///A direct text with out special characters report answering the user's question. Not a description of what was done.
    pub report: String,
}
impl FlatSchema for Report {}
pub struct Investigator<P: LLMProvider + Clone, F: InvestigatorToolFactory> {
    runner: ReactLoop<P>,
    factory: F,
}

impl<'a, P: LLMProvider + Clone, F: InvestigatorToolFactory> Investigator<P, F> {
    pub fn new(runner: ReactLoop<P>, factory: F) -> Self {
        Self { runner, factory }
    }

    pub async fn run(&mut self, question: impl Into<String>) -> Result<(Plan, Report), AgentError> {
        let question = question.into();

        let mut planner_agent = Agent::planner(
            Model::with_default_temp(ModelName::GptOss120B),
            self.factory.planner_tools(),
        );
        let mut planner_session = planner_agent.build_session(format!("Question:\n{}", question));

        let plan: Plan = self
            .runner
            .run(&mut planner_agent, &mut planner_session)
            .await?;

        let plan_json = serde_json::to_string_pretty(&plan).expect("plan serializes");
        let user_prompt = format!(
            "Question: {}\n\nInvestigation plan:\n{}",
            question, plan_json
        );

        let mut executor_agent = Agent::executor(
            Model::creative(ModelName::GptOss120B),
            self.factory.executor_tools(),
        );
        let mut executor_session = executor_agent.build_session(user_prompt);

        let report: Report = self
            .runner
            .run(&mut executor_agent, &mut executor_session)
            .await?;

        Ok((plan, report))
    }
}