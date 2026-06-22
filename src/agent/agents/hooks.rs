use crate::agent::archtectures::hook::{HookAction, LoopHook};
use crate::agent::error::AgentError;
use crate::core::session::{AgentSession, AgentToolCall};
use std::collections::HashMap;

pub struct ToolsRegulator {
    seen_tools: HashMap<String, AgentToolCall>,
    errors: Vec<AgentError>,
}

impl ToolsRegulator {
    pub fn new() -> Self {
        Self {
            seen_tools: HashMap::new(),
            errors: Vec::new(),
        }
    }
    pub fn mark_as_seen(&mut self, tool: AgentToolCall, key: String) {
        self.seen_tools.insert(key, tool);
    }
}

impl LoopHook for ToolsRegulator {
    fn pre_call(
        &mut self,
        session: &mut AgentSession,
        call: &AgentToolCall,
    ) -> Result<HookAction, AgentError> {
        let key = format!("{}::{}", call.name(), call.arguments());

        if let Some(prev_call) = self.seen_tools.get(&key) {
            session.add_error(format!(
                "You already called the tool named \"{}\" with the same arguments and id: \"{}\".Think step by step how to continue your trajectory to finish your task.",
                prev_call.name() , prev_call.id()
            ));
            Ok(HookAction::Skip)
        } else {
            self.mark_as_seen(call.clone(), key);
            Ok(HookAction::Continue)
        }
    }

    fn clear_state(&mut self) {
        self.seen_tools.clear();
        self.errors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::ConversationEvent;
    use serde_json::json;

    fn fake_call(name: &str, args: serde_json::Value) -> AgentToolCall {
        AgentToolCall::new(name.into(), "call_1".into(), args)
    }

    #[test]
    fn first_call_continues() {
        let mut hook = ToolsRegulator::new();
        let mut session = AgentSession::new(5);
        let call = fake_call("bash", json!({"command": "ls"}));

        let action = hook.pre_call(&mut session, &call).unwrap();
        assert!(matches!(action, HookAction::Continue));
    }

    #[test]
    fn duplicate_call_skips() {
        let mut hook = ToolsRegulator::new();
        let mut session = AgentSession::new(5);
        let call = fake_call("bash", json!({"command": "ls"}));

        hook.pre_call(&mut session, &call).unwrap();
        let action = hook.pre_call(&mut session, &call).unwrap();

        assert!(matches!(action, HookAction::Skip));
        println!("{:?}", session);
    }

    #[test]
    fn same_tool_different_args_continues() {
        let mut hook = ToolsRegulator::new();
        let mut session = AgentSession::new(5);

        let call_a = fake_call("read_file", json!({"path": "src/main.rs"}));
        let call_b = fake_call("read_file", json!({"path": "src/lib.rs"}));

        hook.pre_call(&mut session, &call_a).unwrap();
        let action = hook.pre_call(&mut session, &call_b).unwrap();

        assert!(matches!(action, HookAction::Continue));
    }

    #[test]
    fn duplicate_injects_error_into_session() {
        let mut hook = ToolsRegulator::new();
        let mut session = AgentSession::new(5);
        let call = fake_call("bash", json!({"command": "ls"}));

        hook.pre_call(&mut session, &call).unwrap();
        hook.pre_call(&mut session, &call).unwrap();

        let has_error = session.events().iter().any(|e| {
            matches!(
                e,
                ConversationEvent::System(s) if s.contains("already called")
            )
        });
        assert!(has_error, "should inject error message on duplicate");
    }
}
