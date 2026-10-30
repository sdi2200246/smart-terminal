mod policy;
mod signal_dispatcher;
mod tool_dispatcher;

pub use policy::{PermissionDecision, ToolExecutionPolicy, ToolPermissionMode};
pub use signal_dispatcher::SignalDispatcher;
pub use tool_dispatcher::ToolDispatcher;

use crate::agent::agents::Agent;
use crate::agent::signals::{AgentAction, ModelCall};
use crate::core::responce::{AgentResponse, AgentToolCall};
use crate::core::session::{AgentSession, ToolCall, ToolResult};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct ToolExecutionEngine {
    signal_dispatcher: SignalDispatcher,
    tool_dispatcher: ToolDispatcher,
}

impl ToolExecutionEngine {
    pub fn new() -> Self {
        Self::new_with_policy(ToolExecutionPolicy::default())
    }

    pub fn new_with_policy(policy: ToolExecutionPolicy) -> Self {
        Self {
            signal_dispatcher: SignalDispatcher,
            tool_dispatcher: ToolDispatcher::new(policy),
        }
    }

    pub fn policy(&self) -> &ToolExecutionPolicy {
        self.tool_dispatcher.policy()
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
            if !matches!(
                agent.action_space.get(call.name()),
                Some(AgentAction::Signal(_))
            ) {
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
        // Pre-sized so every call keeps its position.
        let mut results: Vec<Option<ToolResult>> = (0..calls.len()).map(|_| None).collect();

        // 1. Decode once, at the boundary.
        let mut signals = Vec::new();
        let mut tools = Vec::new();
        for call in agent.action_space.decode(calls) {
            match call {
                ModelCall::Signal(s) => signals.push(s),
                ModelCall::Tool(t) => tools.push(t),
                ModelCall::Unknown { pos, name } => {
                    let orig = &calls[pos];
                    let reason = format!("Unknown tool: {}", name);
                    let msg = format!("Tool '{}' failed: {}", name, reason);
                    agent.hooks.on_tool_failed(orig, &reason);
                    results[pos] = Some(ToolResult::new(name, msg, orig.id()));
                }
            }
        }

        // 2. Signals first. Sequential, main thread.
        let (signal_results, final_answer) = self
            .signal_dispatcher
            .run(signals, calls, session, agent);
        for (pos, r) in signal_results {
            results[pos] = Some(r);
        }

        // 3. Tools: authorize on the main thread, then execute.
        let (permitted, denied) = self
            .tool_dispatcher
            .authorize(tools, calls, session, agent)
            .await;
        for (pos, r) in denied {
            results[pos] = Some(r);
        }
        for (pos, r) in self.tool_dispatcher.execute(permitted, calls, agent).await {
            results[pos] = Some(r);
        }

        // 4. Every call has exactly one result, in the original order.
        let results = results
            .into_iter()
            .enumerate()
            .map(|(pos, r)| {
                r.unwrap_or_else(|| {
                    let orig = &calls[pos];
                    ToolResult::new(
                        orig.name(),
                        "internal error: no result for call",
                        orig.id(),
                    )
                })
            })
            .collect();
        (results, final_answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::{Agent, AgentBuilder};
    use crate::agent::signals::{
        AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome,
        UpdateScratchpadSignal,
    };
    use crate::core::capability::{Capability, ToolMetaData, ToolSafety};
    use crate::core::model::{Model, ModelName};
    use crate::core::session::ConversationEvent;
    use crate::tools::error::ToolError;
    use serde_json::json;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

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
    impl AgentSignal for FinalAnswer {
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
        fn handle(
            &self,
            args: Value,
            _ctx: &mut AgentSignalContext<'_>,
        ) -> Result<AgentSignalOutcome, ToolError> {
            Ok(AgentSignalOutcome {
                result: args.to_string(),
                control: AgentLoopControl::Finish(args),
            })
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

    fn test_agent_builder(tools: Vec<Box<dyn Capability>>) -> AgentBuilder {
        AgentBuilder::base(
            "test prompt",
            Model::with_default_temp(ModelName::GptOss120B),
        )
        .with_tools(tools)
    }

    fn test_agent(tools: Vec<Box<dyn Capability>>) -> Agent {
        test_agent_builder(tools).build()
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
        let mut agent = test_agent_builder(vec![Box::new(Boom)])
            .with_events_streaming(tx)
            .build();

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
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(FinalAnswer)])
            .build();

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
        let mut agent = test_agent_builder(vec![Box::new(Echo)])
            .with_events_streaming(tx)
            .build();

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
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(FinalAnswer)])
            .with_events_streaming(tx)
            .build();

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
        engine
            .dispatch_tool_batch(&mut session, &mut agent, response)
            .await;

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
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .build();

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
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .build();

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
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .with_events_streaming(tx)
            .build();

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
        // Registered as a signal, not a tool capability; even under deny policy it skips tool permission check
        let engine = ToolExecutionEngine::new_with_policy(ToolPermissionMode::Deny.to_policy());
        let mut session = AgentSession::new(5);
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .build();

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("update_scratchpad", "call_1", valid_scratchpad_args())),
            )
            .await;

        assert!(session.scratchpad.is_some());
    }

    #[tokio::test]
    async fn final_answer_is_not_gated_by_scratchpad_updates() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::builder().user("request").build();
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(UpdateScratchpadSignal), Box::new(FinalAnswer)])
            .build();

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("final_answer", "answer_1", json!({"ok": true}))),
            )
            .await;

        assert_eq!(session.final_answer, Some(json!({"ok": true})));
    }

    struct FailingSignal;
    impl AgentSignal for FailingSignal {
        fn name(&self) -> &'static str {
            "failing_sig"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "always fails".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn handle(
            &self,
            _args: Value,
            _ctx: &mut AgentSignalContext<'_>,
        ) -> Result<AgentSignalOutcome, ToolError> {
            Err(ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("signal bad input"),
            })
        }
    }

    #[tokio::test]
    async fn signal_returning_error_becomes_error_result_not_panic() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent_builder(vec![])
            .with_signals(vec![Box::new(FailingSignal)])
            .build();

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("failing_sig", "call_1", json!({}))),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_1");
                assert!(rs[0].result.contains("Tool 'failing_sig' failed"));
                assert!(rs[0].result.contains("signal bad input"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    struct LoggedTool {
        name: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }
    impl Capability for LoggedTool {
        fn name(&self) -> &'static str {
            self.name
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name.into(),
                description: "logged tool".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            self.log.lock().unwrap().push(self.name);
            Ok(format!("tool:{}", self.name))
        }
    }

    struct LoggedSignal {
        name: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }
    impl AgentSignal for LoggedSignal {
        fn name(&self) -> &'static str {
            self.name
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name.into(),
                description: "logged signal".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn handle(
            &self,
            _args: Value,
            _ctx: &mut AgentSignalContext<'_>,
        ) -> Result<AgentSignalOutcome, ToolError> {
            self.log.lock().unwrap().push(self.name);
            Ok(AgentSignalOutcome {
                result: format!("sig:{}", self.name),
                control: AgentLoopControl::Continue,
            })
        }
    }

    #[tokio::test]
    async fn mixed_batch_runs_signals_before_tools_and_preserves_call_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent_builder(vec![
            Box::new(LoggedTool {
                name: "tool_a",
                log: log.clone(),
            }),
            Box::new(LoggedTool {
                name: "tool_b",
                log: log.clone(),
            }),
        ])
        .with_signals(vec![Box::new(LoggedSignal {
            name: "sig_s",
            log: log.clone(),
        })])
        .build();

        let mut response = AgentResponse::new();
        response.push(call("tool_a", "id_a", json!({})));
        response.push(call("sig_s", "id_s", json!({})));
        response.push(call("tool_b", "id_b", json!({})));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, response)
            .await;

        // S ran before A and B
        assert_eq!(*log.lock().unwrap(), vec!["sig_s", "tool_a", "tool_b"]);

        // Results returned in original call order [A, S, B]
        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 3);
                assert_eq!(rs[0].id, "id_a");
                assert_eq!(rs[0].name, "tool_a");
                assert_eq!(rs[0].result, "tool:tool_a");

                assert_eq!(rs[1].id, "id_s");
                assert_eq!(rs[1].name, "sig_s");
                assert_eq!(rs[1].result, "sig:sig_s");

                assert_eq!(rs[2].id, "id_b");
                assert_eq!(rs[2].name, "tool_b");
                assert_eq!(rs[2].result, "tool:tool_b");
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    struct TrackedMutatingTool {
        executed: Arc<AtomicBool>,
    }
    impl Capability for TrackedMutatingTool {
        fn name(&self) -> &'static str {
            "mutating_op"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "mutating op".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Mutating,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            self.executed.store(true, Ordering::SeqCst);
            Ok("ran mutating_op".into())
        }
    }

    #[tokio::test]
    async fn denied_tool_never_reaches_execute_and_result_is_in_right_position() {
        let executed = Arc::new(AtomicBool::new(false));
        let engine = ToolExecutionEngine::new_with_policy(ToolPermissionMode::Deny.to_policy());
        let mut session = AgentSession::new(5);
        let mut agent = test_agent_builder(vec![
            Box::new(Echo),
            Box::new(TrackedMutatingTool {
                executed: executed.clone(),
            }),
        ])
        .with_signals(vec![Box::new(UpdateScratchpadSignal)])
        .build();

        let mut response = AgentResponse::new();
        response.push(call("echo", "id_0", json!({"n": 1})));
        response.push(call("mutating_op", "id_1", json!({})));
        response.push(call("update_scratchpad", "id_2", valid_scratchpad_args()));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, response)
            .await;

        assert!(
            !executed.load(Ordering::SeqCst),
            "denied tool must never reach execute"
        );

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 3);
                assert_eq!(rs[0].id, "id_0");
                assert!(rs[0].result.starts_with("echo:"));

                assert_eq!(rs[1].id, "id_1");
                assert_eq!(rs[1].name, "mutating_op");
                assert!(rs[1].result.contains("Tool 'mutating_op' failed"));
                assert!(rs[1].result.contains("blocked by the current run permission policy"));

                assert_eq!(rs[2].id, "id_2");
                assert_eq!(rs[2].result, "scratchpad updated successfully");
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_tool_name_behaves_as_before() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        let mut response = AgentResponse::new();
        response.push(call("echo", "id_0", json!({})));
        response.push(call("nonexistent_tool", "id_1", json!({})));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, response)
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 2);
                assert_eq!(rs[0].id, "id_0");
                assert!(rs[0].result.starts_with("echo:"));

                assert_eq!(rs[1].id, "id_1");
                assert_eq!(rs[1].name, "nonexistent_tool");
                assert_eq!(
                    rs[1].result,
                    "Tool 'nonexistent_tool' failed: Unknown tool: nonexistent_tool"
                );
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }
}

