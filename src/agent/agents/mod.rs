pub mod builder;
mod contexts;
pub mod events;
pub mod hooks;

pub use builder::AgentBuilder;
pub use events::{AgentEvent, AgentState};

use crate::agent::patterns::hook::AgentLoopHook;
use crate::agent::patterns::tool_engine::PermissionDecision;
use crate::agent::signals::AgentActionSpace;
use crate::core::model::Model;
use crate::core::responce::AgentToolCall;
use crate::core::session::AgentSession;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

pub struct Agent {
    pub action_space: AgentActionSpace,
    pub system_prompt: String,
    pub model: Model,
    pub hooks: Box<dyn AgentLoopHook>,
    pub context: Option<String>,
    pub(crate) event_stream: Option<UnboundedSender<AgentEvent>>,
}

impl Agent {
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

    pub fn build_session(&self, user_prompt: impl Into<String>) -> AgentSession {
        let mut builder = AgentSession::builder().system(self.system_prompt.clone());
        if let Some(ctx) = &self.context {
            builder = builder.context(ctx);
        }
        builder.user(user_prompt).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{Model, ModelName};

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
}
