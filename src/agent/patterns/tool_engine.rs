use crate::agent::agents::Agent;
use crate::core::capability::ToolSafety;
use crate::core::responce::{AgentResponse, AgentToolCall};
use crate::core::session::{AgentSession, Scratchpad, ToolCall, ToolResult};
use serde_json::Value;
use tokio::sync::oneshot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionDecision {
    Approved,
    Denied(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ToolPermissionMode {
    Prompt,
    Allow,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolExecutionPolicy {
    pub allow_safe_tools_without_prompt: bool,
    pub auto_execute_mutating_tools: bool,
    pub deny_mutating_tools: bool,
    pub blacklisted_tools: Vec<String>,
}

impl Default for ToolExecutionPolicy {
    fn default() -> Self {
        Self {
            allow_safe_tools_without_prompt: true,
            auto_execute_mutating_tools: false,
            deny_mutating_tools: false,
            blacklisted_tools: Vec::new(),
        }
    }
}

impl ToolPermissionMode {
    pub fn to_policy(self) -> ToolExecutionPolicy {
        match self {
            Self::Prompt => ToolExecutionPolicy::default(),
            Self::Allow => ToolExecutionPolicy {
                allow_safe_tools_without_prompt: true,
                auto_execute_mutating_tools: true,
                deny_mutating_tools: false,
                blacklisted_tools: Vec::new(),
            },
            Self::Deny => ToolExecutionPolicy {
                allow_safe_tools_without_prompt: true,
                auto_execute_mutating_tools: false,
                deny_mutating_tools: true,
                blacklisted_tools: Vec::new(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolExecutionEngine {
    policy: ToolExecutionPolicy,
}

impl ToolExecutionEngine {
    pub fn new() -> Self {
        Self::new_with_policy(ToolExecutionPolicy::default())
    }

    pub fn new_with_policy(policy: ToolExecutionPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &ToolExecutionPolicy {
        &self.policy
    }

    pub async fn dispatch_tool_batch(
        &self,
        session: &mut AgentSession,
        agent: &mut Agent,
        response: AgentResponse,
    ) {
        let calls = response.into_calls();

        self.notify_and_stream_calls(agent, &calls);
        self.record_tool_calls(session, &calls);

        let (results, final_answer) = self.execute_tool_batch(session, agent, &calls).await;

        session.add_tool_results(results);
        if let Some(value) = final_answer {
            session.set_final_answer(value);
        }
    }

    fn notify_and_stream_calls(&self, agent: &mut Agent, calls: &[AgentToolCall]) {
        for call in calls {
            agent.hooks.on_tool_received(call);
            if call.name() != "update_scratchpad" && call.name() != "final_answer" {
                agent.stream_tool_call(call);
            }
        }
    }

    fn record_tool_calls(&self, session: &mut AgentSession, calls: &[AgentToolCall]) {
        let session_calls = calls
            .iter()
            .map(|c| {
                ToolCall::new(c.name(), c.arguments().clone(), c.id())
                    .with_thinking_state(c.thinking_state())
            })
            .collect();
        session.add_tool_calls(session_calls);
    }

    async fn execute_tool_batch(
        &self,
        session: &mut AgentSession,
        agent: &mut Agent,
        calls: &[AgentToolCall],
    ) -> (Vec<ToolResult>, Option<Value>) {
        let mut results = Vec::with_capacity(calls.len());
        let mut final_answer = None;

        for call in calls {
            let (payload, _) = self.execute_single_tool(session, agent, call, &mut final_answer).await;
            results.push(ToolResult::new(call.name(), payload, call.id()));
        }

        (results, final_answer)
    }

    async fn check_tool_permissions(&self, agent: &mut Agent, call: &AgentToolCall) -> PermissionDecision {
        let Some(tool) = agent.registry.get(call.name()) else {
            return PermissionDecision::Denied(format!("Unknown tool: {}", call.name()));
        };

        let metadata = tool.metadata();
        if self.policy.blacklisted_tools.iter().any(|name| name == call.name()) {
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

    async fn execute_single_tool(
        &self,
        session: &mut AgentSession,
        agent: &mut Agent,
        call: &AgentToolCall,
        final_answer: &mut Option<Value>,
    ) -> (String, bool) {
        match call.name() {
            "update_scratchpad" => (self.execute_scratchpad_update(session, call.clone()), false),
            _ => {
                match self.check_tool_permissions(agent, call).await {
                    PermissionDecision::Approved => match self.execute_tool(agent, call) {
                        Ok(result) => {
                            if call.name() == "final_answer" {
                                *final_answer = Some(call.arguments().clone());
                            }
                            (result, false)
                        }
                        Err(e) => {
                            let msg = format!("Tool '{}' failed: {}", call.name(), e);
                            agent.hooks.on_tool_failed(call, &msg);
                            (msg, false)
                        }
                    },
                    PermissionDecision::Denied(reason) => {
                        let msg = format!("Tool '{}' failed: {}", call.name(), reason);
                        agent.hooks.on_tool_failed(call, &reason);
                        (msg, true)
                    }
                }
            }
        }
    }

    fn execute_tool(&self, agent: &mut Agent, call: &AgentToolCall) -> Result<String, String> {
        agent
            .registry
            .get(call.name())
            .expect("correct_tool_name")
            .execute(call.arguments().clone())
            .map_err(|e| {
                let msg = e.to_string();
                agent.hooks.on_tool_failed(call, &msg);
                msg
            })
    }

    fn execute_scratchpad_update(&self, session: &mut AgentSession, call: AgentToolCall) -> String {
        match serde_json::from_value::<Scratchpad>(call.arguments()) {
            Ok(sp) => {
                session.update_scratchpad(sp);
                "scratchpad updated successfully".to_string()
            }
            Err(e) => format!(
                "Tool 'update_scratchpad' failed: invalid scratchpad — {}",
                e
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::Agent;
    use crate::core::capability::{Capability, ToolMetaData, ToolSafety};
    use crate::core::model::{Model, ModelName};
    use crate::core::session::ConversationEvent;
    use crate::tools::error::ToolError;
    use crate::tools::scratchpad::UpdateScratchpad;
    use serde_json::json;

    struct Echo;
    impl Capability for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "echoes args".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, args: Value) -> Result<String, ToolError> {
            Ok(format!("echo:{args}"))
        }
    }

    struct Boom;
    impl Capability for Boom {
        fn name(&self) -> &'static str {
            "boom"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "always fails".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Err(ToolError::ToolExecution {
                source: anyhow::anyhow!("boom failed"),
            })
        }
    }

    struct FinalAnswer;
    impl Capability for FinalAnswer {
        fn name(&self) -> &'static str {
            "final_answer"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "submits final answer".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, args: Value) -> Result<String, ToolError> {
            Ok(args.to_string())
        }
    }

    struct Dangerous;
    impl Capability for Dangerous {
        fn name(&self) -> &'static str {
            "bash"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "dangerous mutating command".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Mutating,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Ok("executed dangerous command".into())
        }
    }

    fn test_agent(tools: Vec<Box<dyn Capability>>) -> Agent {
        Agent::base(
            "test prompt",
            Model::with_default_temp(ModelName::GptOss120B),
        )
        .with_tools(tools)
    }

    fn call(name: &str, id: &str, args: Value) -> AgentToolCall {
        AgentToolCall::new(name.into(), id.into(), args, None)
    }

    #[tokio::test]
    async fn successful_tool_produces_matching_result() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("echo", "call_1", json!({"x": 1}))),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_1");
                assert!(rs[0].result.starts_with("echo:"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mutating_tool_requires_approval_by_default() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Dangerous)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bash", "call_1", json!({}))),
            )
            .await;

        let result_event = session
            .events()
            .iter()
            .find(|event| matches!(event, ConversationEvent::ToolResults(_)))
            .expect("tool result should exist");
        match result_event {
            ConversationEvent::ToolResults(rs) => {
                assert!(rs[0].result.contains("requires approval"));
                assert!(!rs[0].result.contains("executed dangerous command"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }

    }

    #[tokio::test]
    async fn mutating_tool_can_be_auto_executed_when_allowed_by_policy() {
        let engine = ToolExecutionEngine::new_with_policy(ToolExecutionPolicy {
            allow_safe_tools_without_prompt: true,
            auto_execute_mutating_tools: true,
            deny_mutating_tools: false,
            blacklisted_tools: vec![],
        });
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Dangerous)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bash", "call_1", json!({}))),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert!(rs[0].result.contains("executed dangerous command"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn failed_tool_wraps_error_message_in_result() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Boom)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("boom", "call_1", json!({}))),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert!(rs[0].result.contains("Tool 'boom' failed"));
                assert!(rs[0].result.contains("boom failed"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn failed_tool_call_is_still_streamed_for_activity_logging() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Boom)]).with_events_streaming(tx);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("boom", "call_1", json!({}))),
            )
            .await;

        assert!(matches!(
            rx.try_recv(),
            Ok(crate::agent::agents::AgentEvent::ToolCall(call)) if call.name() == "boom"
        ));
    }

    #[tokio::test]
    async fn final_answer_call_sets_session_final_answer() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(FinalAnswer)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("final_answer", "call_1", json!({"result": "42"}))),
            )
            .await;

        assert_eq!(session.take_final_answer(), Some(json!({"result": "42"})));
    }

    #[tokio::test]
    async fn non_final_tool_call_is_streamed_when_configured() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]).with_events_streaming(tx);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("echo", "call_1", json!({}))),
            )
            .await;

        assert!(matches!(
            rx.try_recv().expect("expected a streamed call"),
            crate::agent::agents::AgentEvent::ToolCall(call)
                if call.name() == "echo"
        ));
    }

    #[tokio::test]
    async fn final_answer_call_is_not_streamed() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(FinalAnswer)]).with_events_streaming(tx);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("final_answer", "call_1", json!({"result": "ok"}))),
            )
            .await;

        assert!(
            rx.try_recv().is_err(),
            "final_answer should not be streamed"
        );
    }

    #[tokio::test]
    async fn thinking_state_is_carried_into_tool_call_event() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        let signed = AgentToolCall::new(
            "echo".into(),
            "call_1".into(),
            json!({}),
            Some("sig-a".into()),
        );
        engine
            .dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(signed))
            .await;

        let calls = session
            .events()
            .iter()
            .find_map(|e| match e {
                ConversationEvent::ToolCalls(c) => Some(c),
                _ => None,
            })
            .expect("ToolCalls event present");
        assert_eq!(calls[0].thinking_state, Some("sig-a".into()));
    }

    #[tokio::test]
    async fn multiple_calls_produce_matching_ordered_results() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo), Box::new(Boom)]);

        let mut response = AgentResponse::new();
        response.push(call("echo", "call_1", json!({})));
        response.push(call("boom", "call_2", json!({})));
        engine.dispatch_tool_batch(&mut session, &mut agent, response).await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_1");
                assert!(rs[0].result.starts_with("echo:"));
                assert_eq!(rs[1].id, "call_2");
                assert!(rs[1].result.contains("failed"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }
    fn valid_scratchpad_args() -> Value {
        json!({
            "main_goal": "ship the scratchpad tool",
            "completed_milestones": ["designed schema"],
            "current_focus": "wiring the engine",
            "vital_findings": ["session owns Scratchpad directly"]
        })
    }

    #[tokio::test]
    async fn scratchpad_update_success_mutates_session_and_returns_confirmation() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::builder().user("investigate").build();
        let mut agent = test_agent(vec![]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("update_scratchpad", "call_1", valid_scratchpad_args())),
            )
            .await;

        let sp = session
            .scratchpad
            .as_ref()
            .expect("scratchpad should be Some after update");
        assert_eq!(sp.main_goal, "ship the scratchpad tool");
        assert_eq!(sp.current_focus, "wiring the engine");

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_1");
                assert_eq!(rs[0].result, "scratchpad updated successfully");
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn scratchpad_update_invalid_args_leaves_session_scratchpad_untouched() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::builder().user("investigate").build();
        let mut agent = test_agent(vec![]);

        // missing required fields -> serde_json::from_value fails
        let bad_args = json!({ "main_goal": "only this field" });

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("update_scratchpad", "call_1", bad_args)),
            )
            .await;

        assert_eq!(session.scratchpad, None);

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert!(rs[0].result.contains("Tool 'update_scratchpad' failed"));
                assert!(rs[0].result.contains("invalid scratchpad"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn scratchpad_update_is_never_streamed() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![]).with_events_streaming(tx);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("update_scratchpad", "call_1", valid_scratchpad_args())),
            )
            .await;

        assert!(
            rx.try_recv().is_err(),
            "scratchpad updates should not be streamed"
        );
    }

    #[tokio::test]
    async fn scratchpad_update_does_not_go_through_registry() {
        // empty registry — if execute_tool were hit, `.expect("correct_tool_name")` would panic
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("update_scratchpad", "call_1", valid_scratchpad_args())),
            )
            .await;
    }

    #[tokio::test]
    async fn final_answer_is_not_gated_by_scratchpad_updates() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::builder().user("request").build();
        let mut agent = test_agent(vec![Box::new(UpdateScratchpad), Box::new(FinalAnswer)]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("final_answer", "answer_1", json!({"ok": true}))),
            )
            .await;

        assert_eq!(session.final_answer, Some(json!({"ok": true})));
    }
}
