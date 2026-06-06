use crate::core::session::{AgentSession , AgentToolCall};
use crate::agent::error::AgentError;

pub enum HookAction {
    Continue,
    Skip,
}

pub trait LoopHook: Send + Sync {
    fn pre_call(
        &mut self,
        session: &mut AgentSession,
        call: &AgentToolCall,
    ) -> Result<HookAction, AgentError> {
        Ok(HookAction::Continue)
    }

    fn post_call(
        &mut self,
        session: &mut AgentSession,
        call: &AgentToolCall,
    ) -> Result<(), AgentError> {
        Ok(())
    }
    fn clear_state(&mut self);
}