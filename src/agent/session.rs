use crate::core::{ConversationEvent, CoreAgentSession, Scratchpad, ToolCall, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_STEPS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub events: Vec<ConversationEvent>,
    pub steps: usize,
    pub final_answer: Option<Value>,
    pub scratchpad: Option<Scratchpad>,
}

impl AgentSession {
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }

    pub fn with_steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    pub fn replace_initial_system_prompt(&mut self, message: impl Into<String>) {
        let message = message.into();
        match self.events.first_mut() {
            Some(ConversationEvent::System(existing)) => *existing = message,
            _ => self.events.insert(0, ConversationEvent::System(message)),
        }
    }

    pub fn add_user(&mut self, message: impl Into<String>) {
        self.events.push(ConversationEvent::User(message.into()));
    }

    pub fn add_assistant(&mut self, message: impl Into<String>) {
        self.events
            .push(ConversationEvent::Assistant(message.into()));
    }

    pub fn add_tool_calls(&mut self, calls: Vec<ToolCall>) {
        debug_assert!(!calls.is_empty(), "empty tool call batch");
        self.events.push(ConversationEvent::ToolCalls(calls));
    }

    pub fn add_tool_results(&mut self, results: Vec<ToolResult>) {
        debug_assert_eq!(
            self.pending_call_count(),
            Some(results.len()),
            "tool results must pair 1:1 with the preceding call batch"
        );
        self.events.push(ConversationEvent::ToolResults(results));
    }

    pub fn add_error(&mut self, er: String) {
        self.events
            .push(ConversationEvent::System(format!("[ERROR]:{}", er)));
    }

    fn pending_call_count(&self) -> Option<usize> {
        match self.events.last() {
            Some(ConversationEvent::ToolCalls(calls)) => Some(calls.len()),
            _ => None,
        }
    }

    fn current_steps(&self) -> usize {
        let start = self
            .events
            .iter()
            .rposition(|event| matches!(event, ConversationEvent::User(_)))
            .map_or(0, |index| index + 1);
        self.events[start..]
            .iter()
            .filter(|e| matches!(e, ConversationEvent::ToolCalls(_)))
            .count()
    }

    pub fn steps_exhausted(&self) -> bool {
        self.current_steps() >= self.steps
    }

    pub fn events(&self) -> &[ConversationEvent] {
        &self.events
    }

    pub fn set_final_answer(&mut self, value: Value) {
        self.final_answer = Some(value);
    }

    pub fn take_final_answer(&mut self) -> Option<Value> {
        self.final_answer.take()
    }

    pub fn update_scratchpad(&mut self, new: Scratchpad) {
        self.scratchpad = Some(new);
    }
}

impl From<CoreAgentSession> for AgentSession {
    fn from(core: CoreAgentSession) -> Self {
        Self {
            events: core.events,
            steps: DEFAULT_STEPS,
            final_answer: None,
            scratchpad: core.scratchpad,
        }
    }
}

impl From<&AgentSession> for CoreAgentSession {
    fn from(session: &AgentSession) -> Self {
        Self {
            events: session.events.clone(),
            scratchpad: session.scratchpad.clone(),
        }
    }
}

impl From<AgentSession> for CoreAgentSession {
    fn from(session: AgentSession) -> Self {
        Self {
            events: session.events,
            scratchpad: session.scratchpad,
        }
    }
}

pub struct SessionBuilder {
    events: Vec<ConversationEvent>,
    steps: usize,
}

impl SessionBuilder {
    fn new() -> Self {
        Self {
            events: vec![],
            steps: DEFAULT_STEPS,
        }
    }

    pub fn system(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::System(message.into()));
        self
    }

    pub fn user(mut self, message: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::User(message.into()));
        self
    }

    pub fn formatted_context(mut self, json: impl Into<String>) -> Self {
        self.events.push(ConversationEvent::System(format!(
            "Context:\n{}",
            json.into()
        )));
        self
    }

    pub fn steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    pub fn build(self) -> AgentSession {
        AgentSession {
            events: self.events,
            steps: self.steps,
            final_answer: None,
            scratchpad: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_initial_system_prompt_preserves_the_rest_of_the_transcript() {
        let mut session = AgentSession::builder()
            .system("old prompt")
            .formatted_context("{\"cwd\":\"project\"}")
            .user("first question")
            .build();
        session.add_assistant("prior answer");
        let transcript = session.events[1..].to_vec();

        session.replace_initial_system_prompt("updated prompt");

        assert_eq!(
            session.events.first(),
            Some(&ConversationEvent::System("updated prompt".into()))
        );
        assert_eq!(&session.events[1..], transcript.as_slice());
    }

    #[test]
    fn resumed_session_counts_steps_after_latest_user_event() {
        let mut session = AgentSession::builder().steps(1).user("first run").build();
        session.add_tool_calls(vec![ToolCall::new("old", serde_json::Value::Null, "old")]);
        assert!(session.steps_exhausted());

        session.add_user("resumed run");
        assert!(!session.steps_exhausted());

        session.add_tool_calls(vec![ToolCall::new("new", serde_json::Value::Null, "new")]);
        assert!(session.steps_exhausted());
    }

    #[test]
    fn translates_round_trip_between_agent_and_core_sessions() {
        let mut agent_session = AgentSession::builder()
            .steps(12)
            .system("sys")
            .user("hello")
            .build();
        agent_session.update_scratchpad(Scratchpad {
            main_goal: "goal".into(),
            completed_milestones: vec!["m1".into()],
            current_focus: "focus".into(),
            vital_findings: vec!["f1".into()],
        });

        let core_session = CoreAgentSession::from(&agent_session);
        assert_eq!(core_session.events, agent_session.events);
        assert_eq!(core_session.scratchpad, agent_session.scratchpad);

        let restored = AgentSession::from(core_session).with_steps(12);
        assert_eq!(restored.steps, 12);
        assert_eq!(restored.events, agent_session.events);
        assert_eq!(restored.scratchpad, agent_session.scratchpad);
        assert!(restored.final_answer.is_none());
    }
}
