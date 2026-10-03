use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::agents::Agent;
use crate::agent::agents::AgentEvent;
use crate::agent::error::AgentError;
use crate::agent::patterns::react::ReactLoop;
use crate::agent::workflows::next_cmd::NextCommand;
use crate::core::capability::Capability;
use crate::core::llm_client::LLMProvider;
use crate::core::memory::InvestigationSession;
use crate::core::model::{Model, ModelName};
use crate::utils::FlatSchema;
use tokio::sync::mpsc::UnboundedSender;

pub trait InvestigatorToolFactory {
    fn planner_tools(&self, schema: Value) -> Vec<Box<dyn Capability>>;
    fn executor_tools(&self, schema: Value) -> Vec<Box<dyn Capability>>;
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
    /// The most likely next command the user wants to run, grounded in the investigation.
    pub recommended_command: NextCommand,
}
impl FlatSchema for Report {}

pub struct Investigator<P: LLMProvider + Clone, F: InvestigatorToolFactory> {
    runner: ReactLoop<P>,
    factory: F,
    event_stream: Option<UnboundedSender<AgentEvent>>,
}

impl<'a, P: LLMProvider + Clone, F: InvestigatorToolFactory> Investigator<P, F> {
    pub fn new(runner: ReactLoop<P>, factory: F) -> Self {
        Self {
            runner,
            factory,
            event_stream: None,
        }
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<AgentEvent>) -> Self {
        self.event_stream = Some(tx);
        self
    }

    pub async fn run(&mut self, question: impl Into<String>) -> Result<(Plan, Report), AgentError> {
        let (plan, report, _) = self
            .run_with_session(question, InvestigationSession::default())
            .await?;
        Ok((plan, report))
    }

    pub async fn run_with_session(
        &mut self,
        question: impl Into<String>,
        mut saved_session: InvestigationSession,
    ) -> Result<(Plan, Report, InvestigationSession), AgentError> {
        let question = question.into();

        let mut planner_agent = Agent::planner(
            Model::with_default_temp(ModelName::GptOss120B),
            self.factory.planner_tools(Plan::schema()),
        );
        if let Some(stream) = &self.event_stream {
            planner_agent = planner_agent.with_events_streaming(stream.clone());
        }
        let planner_prompt = planner_prompt(&question, &saved_session.completed_reports);
        let mut planner_session = match saved_session.planner.take() {
            Some(mut session) => {
                session.add_user(planner_prompt);
                session
            }
            None => planner_agent.build_session(planner_prompt),
        };

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
            self.factory.executor_tools(Report::schema()),
        );
        if let Some(stream) = &self.event_stream {
            executor_agent = executor_agent.with_events_streaming(stream.clone());
        }
        let mut executor_session = match saved_session.executor.take() {
            Some(mut session) => {
                session.add_user(user_prompt);
                session
            }
            None => executor_agent.build_session(user_prompt),
        };

        let report: Report = self
            .runner
            .run(&mut executor_agent, &mut executor_session)
            .await?;

        saved_session
            .completed_reports
            .push(serde_json::to_string_pretty(&report).expect("report serializes"));
        saved_session.planner = Some(planner_session);
        saved_session.executor = Some(executor_session);

        Ok((plan, report, saved_session))
    }
}

fn planner_prompt(question: &str, completed_reports: &[String]) -> String {
    if completed_reports.is_empty() {
        return format!("Question:\n{}", question);
    }

    let previous_reports = completed_reports
        .iter()
        .enumerate()
        .map(|(index, report)| format!("Execution {}:\n{}", index + 1, report))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Previous execution reports and findings are included below. Use them to interpret references to earlier results, and re-check facts that may have changed.\n\n{}\n\nCurrent question:\n{}",
        previous_reports, question
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::ProviderError;
    use crate::core::llm_client::AgentRequest;
    use crate::core::responce::{AgentResponse, AgentToolCall};
    use crate::core::session::ConversationEvent;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    struct EmptyToolFactory;

    impl InvestigatorToolFactory for EmptyToolFactory {
        fn planner_tools(&self, _schema: Value) -> Vec<Box<dyn Capability>> {
            Vec::new()
        }

        fn executor_tools(&self, _schema: Value) -> Vec<Box<dyn Capability>> {
            Vec::new()
        }
    }

    #[derive(Clone)]
    struct SessionCaptureProvider(Arc<Mutex<Vec<Vec<ConversationEvent>>>>);

    impl LLMProvider for SessionCaptureProvider {
        async fn complete(
            &self,
            request: AgentRequest<'_>,
        ) -> Result<AgentResponse, ProviderError> {
            self.0.lock().unwrap().push(request.session.events.clone());
            Ok(AgentResponse::single(AgentToolCall::new(
                "stop".into(),
                String::new(),
                json!({"complete":true}),
                None,
            )))
        }

        async fn complete_structured(
            &self,
            _session: &crate::core::session::AgentSession,
            schema: Value,
        ) -> Result<Value, ProviderError> {
            if schema["properties"].get("goal").is_some() {
                Ok(json!({"goal":"test goal","steps":[]}))
            } else {
                Ok(json!({
                    "report":"test report",
                    "recommended_command": {
                        "cmd":"echo done",
                        "man":"Print done.",
                        "scale":"Full"
                    }
                }))
            }
        }
    }

    #[tokio::test]
    async fn continues_planner_and_executor_conversations_on_follow_up() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let runner = ReactLoop::new(SessionCaptureProvider(captured.clone()));
        let mut investigator = Investigator::new(runner, EmptyToolFactory);

        let (_, _, saved_session) = investigator
            .run_with_session("first question", InvestigationSession::default())
            .await
            .unwrap();
        let (second_plan, _, updated_session) = investigator
            .run_with_session("follow-up question", saved_session)
            .await
            .unwrap();

        assert_eq!(second_plan.goal, "test goal");
        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 4);
        for resumed_request in [&requests[2], &requests[3]] {
            assert!(
                resumed_request
                    .iter()
                    .any(|event| matches!(event, ConversationEvent::Assistant(_)))
            );
            assert!(resumed_request.iter().any(|event| matches!(
                event,
                ConversationEvent::User(question) if question.contains("follow-up question")
            )));
        }
        assert!(requests[2].iter().any(|event| matches!(
            event,
            ConversationEvent::User(question)
                if question.contains("test report") && question.contains("echo done")
        )));
        assert!(updated_session.planner.is_some());
        assert!(updated_session.executor.is_some());
        assert_eq!(updated_session.completed_reports.len(), 2);
    }

    #[test]
    fn report_requires_a_recommended_command() {
        let report = serde_json::json!({
            "report": "The working tree has no changes.",
            "recommended_command": {
                "cmd": "git status --short",
                "man": "Verify that the working tree remains clean.",
                "scale": "Full"
            }
        });
        assert!(serde_json::from_value::<Report>(report).is_ok());

        let report_without_command = serde_json::json!({
            "report": "The working tree has no changes."
        });
        assert!(serde_json::from_value::<Report>(report_without_command).is_err());
    }
}
