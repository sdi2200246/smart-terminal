pub mod builder;
mod contexts;
pub mod events;
pub mod hooks;

pub use builder::AgentBuilder;
pub use events::{AgentEvent, AgentState};

use crate::agent::patterns::hook::AgentLoopHook;
use crate::agent::patterns::tool_engine::PermissionDecision;
use crate::agent::session::AgentSession;
use crate::agent::signals::AgentActionSpace;
use crate::core::model::Model;
use crate::core::responce::AgentToolCall;
use crate::core::session::ConversationEvent;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

pub struct Agent {
    pub action_space: AgentActionSpace,
    pub model: Model,
    pub hooks: Box<dyn AgentLoopHook>,
    pub session: AgentSession,
    pub(crate) event_stream: Option<UnboundedSender<AgentEvent>>,
}

impl Agent {
    pub fn system_prompt(&self) -> &str {
        match self.session.events().first() {
            Some(ConversationEvent::System(prompt)) => prompt.as_str(),
            _ => "",
        }
    }

    pub(crate) fn stream_tool_call(&self, call: &AgentToolCall) {
        if let Some(stream) = &self.event_stream {
            if let Err(error) = stream.send(AgentEvent::ToolCall(call.clone())) {
                tracing::warn!(tool = %call.name(), error = %error, "failed to stream agent tool event");
            }
        }
    }

    pub(crate) fn update_state(&self, state: AgentState) {
        if let Some(stream) = &self.event_stream {
            if let Err(error) = stream.send(AgentEvent::StateUpdate(state)) {
                tracing::warn!(state = state.label(), error = %error, "failed to stream agent state");
            }
        }
    }

    pub(crate) fn request_tool_approval(
        &self,
        call: &AgentToolCall,
        responder: oneshot::Sender<PermissionDecision>,
    ) {
        if let Some(stream) = &self.event_stream {
            if let Err(error) = stream.send(AgentEvent::ToolApprovalRequest {
                call: call.clone(),
                responder,
            }) {
                tracing::warn!(tool = %call.name(), error = %error, "failed to stream tool approval request");
            }
        } else {
            let _ = responder.send(PermissionDecision::Denied(format!(
                "Tool '{}' requires approval but no approval presenter is connected.",
                call.name()
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{Model, ModelName};
    use crate::core::session::CoreAgentSession;

    #[test]
    fn state_updates_are_sent_to_the_presenter() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let agent = AgentBuilder::base(
            "test prompt",
            Model::with_default_temp(ModelName::GptOss120B),
        )
        .with_events_streaming(tx)
        .build();

        agent.update_state(AgentState::ExecutingTool);

        assert!(matches!(
            rx.try_recv(),
            Ok(AgentEvent::StateUpdate(AgentState::ExecutingTool))
        ));
    }

    #[test]
    fn builder_initializes_internal_session_with_prompt_context_and_user_input() {
        let agent = AgentBuilder::base(
            "system role",
            Model::with_default_temp(ModelName::GptOss120B),
        )
        .with_context(&serde_json::json!({"cwd": "/repo"}))
        .with_user_prompt("run tests")
        .with_steps(15)
        .build();

        assert_eq!(agent.system_prompt(), "system role");
        assert_eq!(agent.session.steps, 15);
        assert_eq!(
            agent.session.events(),
            &[
                ConversationEvent::System("system role".into()),
                ConversationEvent::System("Context:\n{\n  \"cwd\": \"/repo\"\n}".into()),
                ConversationEvent::User("run tests".into()),
            ]
        );
    }

    #[test]
    fn builder_resumes_core_session_updating_system_prompt_and_appending_user_prompt() {
        let mut prior = CoreAgentSession::new()
            .with_system("old system")
            .with_user("first question");
        prior
            .events
            .push(ConversationEvent::Assistant("first answer".into()));

        let agent = AgentBuilder::base(
            "new system",
            Model::with_default_temp(ModelName::GptOss120B),
        )
        .with_session(Some(prior))
        .with_user_prompt("follow-up")
        .build();

        assert_eq!(agent.system_prompt(), "new system");
        assert_eq!(
            agent.session.events(),
            &[
                ConversationEvent::System("new system".into()),
                ConversationEvent::User("first question".into()),
                ConversationEvent::Assistant("first answer".into()),
                ConversationEvent::User("follow-up".into()),
            ]
        );
    }
}
