use crate::agent::error::AgentError;
use crate::core::session::{AgentSession, AgentToolCall};

pub enum HookAction {
    Continue,
    Skip,
}

pub trait AgentLoopHook: Send + Sync {
    fn on_loop_start(&mut self) {
        tracing::info!(status = "started", "ReAct loop execution");
    }

    fn on_loop_exhausted(&mut self) {
        tracing::error!(
            status = "exhausted",
            "ReAct loop terminated: step limit reached"
        );
    }

    fn on_final_answer(&mut self) {
        tracing::info!(
            status = "success",
            "ReAct loop terminated: final answer provided"
        );
    }
    fn on_llm_call(&mut self) {
        tracing::debug!(status = "requesting", "Calling LLM provider");
    }

    fn on_invalid_tool_call(&mut self, error: &str) {
        tracing::warn!(status = "invalid_call", error = %error, "LLM returned malformed tool call");
    }

    fn on_provider_error(&mut self, error: &str) {
        tracing::error!(status = "provider_error", error = %error, "LLM provider failure");
    }

    fn on_tool_received(&mut self, call: &AgentToolCall) {
        tracing::debug!(tool = %call.name(), status = "received", "LLM provided tool call");
    }

    fn pre_call(
        &mut self,
        _session: &mut AgentSession,
        call: &AgentToolCall,
    ) -> Result<HookAction, AgentError> {
        tracing::debug!(tool = %call.name(), status = "pre_call", "Preparing to execute tool");
        Ok(HookAction::Continue)
    }

    fn on_tool_skipped(&mut self, call: &AgentToolCall) {
        tracing::warn!(tool = %call.name(), status = "skipped", "Hook requested tool skip");
    }

    fn post_call(
        &mut self,
        _session: &mut AgentSession,
        call: &AgentToolCall,
    ) -> Result<(), AgentError> {
        tracing::debug!(tool = %call.name(), status = "post_call", "Tool executed successfully");
        Ok(())
    }

    fn on_tool_failed(&mut self, call: &AgentToolCall, error: &str) {
        tracing::error!(tool = %call.name(), status = "failed", error = %error, "Tool execution failed");
    }
    fn on_structuring_start(&mut self) {
        tracing::info!(status = "structuring", "Initiating final output structure");
    }
    fn on_structuring_complete(&mut self) {
        tracing::info!(status = "complete", "Final output structure generated");
    }
}
