use super::policy::{PermissionDecision, ToolExecutionPolicy};
use crate::agent::agents::Agent;
use crate::agent::signals::ToolInvocation;
use crate::core::capability::ToolSafety;
use crate::core::responce::AgentToolCall;
use crate::core::session::{AgentSession, ToolResult};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Default)]
pub struct ToolDispatcher {
    policy: ToolExecutionPolicy,
}

impl ToolDispatcher {
    pub fn new(policy: ToolExecutionPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &ToolExecutionPolicy {
        &self.policy
    }

    /// Sequential, main thread. The existing permission check, including interactive prompts.
    /// Returns the calls allowed to run, and immediate results for the denied ones.
    pub async fn authorize(
        &self,
        tools: Vec<ToolInvocation>,
        calls: &[AgentToolCall],
        _session: &mut AgentSession,
        agent: &mut Agent,
    ) -> (Vec<ToolInvocation>, Vec<(usize, ToolResult)>) {
        let mut permitted = Vec::new();
        let mut denied = Vec::new();

        for inv in tools {
            let call = &calls[inv.pos];
            match self.check_tool_permissions(&inv, agent, call).await {
                PermissionDecision::Approved => permitted.push(inv),
                PermissionDecision::Denied(reason) => {
                    let msg = format!("Tool '{}' failed: {}", call.name(), reason);
                    agent.hooks.on_tool_failed(call, &reason);
                    denied.push((inv.pos, ToolResult::new(call.name(), msg, call.id())));
                }
            }
        }

        (permitted, denied)
    }

    /// In this plan: runs the permitted tools synchronously, one after another, in call
    /// order, exactly as today. Plan 3 replaces the inside with background jobs.
    pub async fn execute(
        &self,
        permitted: Vec<ToolInvocation>,
        calls: &[AgentToolCall],
        agent: &mut Agent,
    ) -> Vec<(usize, ToolResult)> {
        let mut out = Vec::with_capacity(permitted.len());
        for inv in permitted {
            let call = &calls[inv.pos];
            let payload = match inv.cap.execute(inv.args) {
                Ok(result) => result,
                Err(e) => {
                    let err_str = e.to_string();
                    agent.hooks.on_tool_failed(call, &err_str);
                    let msg = format!("Tool '{}' failed: {}", call.name(), err_str);
                    agent.hooks.on_tool_failed(call, &msg);
                    msg
                }
            };
            out.push((inv.pos, ToolResult::new(call.name(), payload, call.id())));
        }
        out
    }

    async fn check_tool_permissions(
        &self,
        inv: &ToolInvocation,
        agent: &mut Agent,
        call: &AgentToolCall,
    ) -> PermissionDecision {
        let metadata = inv.cap.metadata();
        if self
            .policy
            .blacklisted_tools
            .iter()
            .any(|name| name == call.name())
        {
            return PermissionDecision::Denied(format!(
                "Tool '{}' is disabled by policy.",
                call.name()
            ));
        }

        if metadata.safety == ToolSafety::Safe && self.policy.allow_safe_tools_without_prompt {
            return PermissionDecision::Approved;
        }

        if matches!(metadata.safety, ToolSafety::Mutating) {
            if self.policy.deny_mutating_tools {
                return PermissionDecision::Denied(format!(
                    "Tool '{}' is blocked by the current run permission policy.",
                    call.name()
                ));
            }

            if self.policy.auto_execute_mutating_tools {
                return PermissionDecision::Approved;
            }

            let (tx, rx) = oneshot::channel();
            agent.request_tool_approval(call, tx);
            match rx.await {
                Ok(decision) => decision,
                Err(_) => PermissionDecision::Denied(format!(
                    "Tool '{}' requires approval but the approval request was cancelled.",
                    call.name()
                )),
            }
        } else {
            PermissionDecision::Approved
        }
    }
}

