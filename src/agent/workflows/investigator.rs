use crate::agent::agents::{AgentBuilder, AgentEvent};
use crate::agent::contracts::{Plan, Report};
use crate::agent::error::AgentError;
use crate::agent::patterns::react::ReactLoop;
use crate::core::capability::Capability;
use crate::core::llm_client::LLMProvider;
use crate::core::memory::InvestigationSession;
use crate::core::model::{Model, ModelName};
use crate::utils::FlatSchema;
use tokio::sync::mpsc::UnboundedSender;

pub trait InvestigatorToolFactory {
    fn planner_tools(&self) -> Vec<Box<dyn Capability>>;
    fn executor_tools(&self) -> Vec<Box<dyn Capability>>;
}

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

        let mut planner_agent = AgentBuilder::planner(
            Model::with_default_temp(ModelName::GptOss120B),
            self.factory.planner_tools(),
            Plan::schema(),
        )
        .with_session(saved_session.planner.take())
        .with_user_prompt(format!("Question:\n{}", question))
        .with_events_streaming_opt(self.event_stream.clone())
        .build();

        let plan: Plan = self.runner.run(&mut planner_agent).await?;

        let user_prompt = format!("Question: {}\n\nInvestigation plan:\n{}", question, plan);

        let mut executor_agent = AgentBuilder::executor(
            Model::creative(ModelName::GptOss120B),
            self.factory.executor_tools(),
            Report::schema(),
        )
        .with_session(saved_session.executor.take())
        .with_user_prompt(user_prompt)
        .with_events_streaming_opt(self.event_stream.clone())
        .build();

        let report: Report = self.runner.run(&mut executor_agent).await?;

        planner_agent
            .session
            .add_user(format!("Execution report:\n{}", report));

        saved_session.planner = Some(planner_agent.session.into());
        saved_session.executor = Some(executor_agent.session.into());

        Ok((plan, report, saved_session))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::ProviderError;
    use crate::core::llm_client::AgentRequest;
    use crate::core::responce::{AgentResponse, AgentToolCall};
    use crate::core::session::{ConversationEvent, CoreAgentSession};
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    struct EmptyToolFactory;

    impl InvestigatorToolFactory for EmptyToolFactory {
        fn planner_tools(&self) -> Vec<Box<dyn Capability>> {
            Vec::new()
        }

        fn executor_tools(&self) -> Vec<Box<dyn Capability>> {
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
            _session: &CoreAgentSession,
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
            ConversationEvent::User(msg)
                if msg.contains("Execution report:")
                    && msg.contains("test report")
                    && msg.contains("echo done")
        )));
        assert!(updated_session.planner.is_some());
        assert!(updated_session.executor.is_some());
    }
}
