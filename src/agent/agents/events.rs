use crate::agent::patterns::tool_engine::PermissionDecision;
use crate::core::responce::AgentToolCall;
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentState {
    Thinking,
    ExecutingTool,
    StructuringOutput,
    Completed,
    Failed,
}

pub enum AgentEvent {
    StateUpdate(AgentState),
    ToolCall(AgentToolCall),
    ToolApprovalRequest {
        call: AgentToolCall,
        responder: oneshot::Sender<PermissionDecision>,
    },
}

impl AgentState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Thinking => "Thinking",
            Self::ExecutingTool => "Executing tool",
            Self::StructuringOutput => "Structuring output",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
        }
    }
}

