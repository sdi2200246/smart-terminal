mod policy;
mod signal_dispatcher;
mod tool_dispatcher;

pub use policy::{PermissionDecision, ToolExecutionPolicy, ToolPermissionMode};
pub use signal_dispatcher::SignalDispatcher;
pub use tool_dispatcher::{DEFAULT_JOB_CAPACITY, TOOL_WAIT_THRESHOLD, ToolDispatcher};

use crate::agent::agents::Agent;
use crate::agent::prompts::must_run_alone_message;
use crate::agent::signals::{
    AgentAction, AgentSignalInvocation, ModelCall, ToolInvocation,
};
use crate::core::responce::{AgentResponse, AgentToolCall};
use crate::core::session::{AgentSession, ToolCall, ToolResult};
use crate::multithreading::JobStore;
use serde_json::Value;
use std::time::Duration;

pub(crate) struct PartitionedBatch {
    pub signals: Vec<AgentSignalInvocation>,
    pub tools: Vec<ToolInvocation>,
    pub rejected: Vec<(usize, ToolResult)>,
}

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

    pub fn new_with_threshold(policy: ToolExecutionPolicy, threshold: Duration) -> Self {
        Self {
            signal_dispatcher: SignalDispatcher,
            tool_dispatcher: ToolDispatcher::new_with_threshold(policy, threshold),
        }
    }

    pub fn policy(&self) -> &ToolExecutionPolicy {
        self.tool_dispatcher.policy()
    }

    pub fn job_store(&self) -> &JobStore {
        self.tool_dispatcher.job_store()
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
        let mut results: Vec<Option<ToolResult>> = (0..calls.len()).map(|_| None).collect();

        let partitioned = self.partition_batch(calls, agent);
        Self::place_results(&mut results, partitioned.rejected);

        let (signal_results, final_answer) = self.signal_dispatcher.run(
            partitioned.signals,
            calls,
            session,
            agent,
            self.tool_dispatcher.job_store(),
        );
        Self::place_results(&mut results, signal_results);

        let (permitted, denied) = self
            .tool_dispatcher
            .authorize(partitioned.tools, calls, session, agent)
            .await;
        Self::place_results(&mut results, denied);

        let executed = self
            .tool_dispatcher
            .execute(permitted, calls, agent)
            .await;
        Self::place_results(&mut results, executed);

        (Self::assemble_results(calls, results), final_answer)
    }

    pub(crate) fn partition_batch(
        &self,
        calls: &[AgentToolCall],
        agent: &mut Agent,
    ) -> PartitionedBatch {
        let decoded = agent.action_space.decode(calls);
        let batch_len = decoded.len();

        let mut signals = Vec::new();
        let mut tools = Vec::new();
        let mut rejected = Vec::new();

        for call in decoded {
            match call {
                ModelCall::Signal(s) if s.sig.must_run_alone() && batch_len > 1 => {
                    let result =
                        self.reject_non_exclusive_signal(&calls[s.pos], s.sig.name(), agent);
                    rejected.push((s.pos, result));
                }
                ModelCall::Signal(s) => signals.push(s),
                ModelCall::Tool(t) => tools.push(t),
                ModelCall::Unknown { pos, name } => {
                    let result = self.reject_unknown_call(&calls[pos], &name, agent);
                    rejected.push((pos, result));
                }
            }
        }

        PartitionedBatch {
            signals,
            tools,
            rejected,
        }
    }

    fn reject_non_exclusive_signal(
        &self,
        orig: &AgentToolCall,
        sig_name: &str,
        agent: &mut Agent,
    ) -> ToolResult {
        let reason = must_run_alone_message(sig_name);
        let msg = format!("Tool '{}' failed: {}", sig_name, reason);
        agent.hooks.on_tool_failed(orig, &reason);
        ToolResult::new(sig_name, msg, orig.id())
    }

    fn reject_unknown_call(
        &self,
        orig: &AgentToolCall,
        name: &str,
        agent: &mut Agent,
    ) -> ToolResult {
        let reason = format!("Unknown tool: {}", name);
        let msg = format!("Tool '{}' failed: {}", name, reason);
        agent.hooks.on_tool_failed(orig, &reason);
        ToolResult::new(name, msg, orig.id())
    }

    fn place_results(
        slots: &mut [Option<ToolResult>],
        entries: Vec<(usize, ToolResult)>,
    ) {
        for (pos, r) in entries {
            slots[pos] = Some(r);
        }
    }

    fn assemble_results(
        calls: &[AgentToolCall],
        results: Vec<Option<ToolResult>>,
    ) -> Vec<ToolResult> {
        results
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
            .collect()
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

        let recorded = log.lock().unwrap().clone();
        assert_eq!(recorded.len(), 3);
        assert_eq!(recorded[0], "sig_s", "signal must always execute before tools");
        assert!(recorded[1..].contains(&"tool_a"));
        assert!(recorded[1..].contains(&"tool_b"));

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

    struct SleepTool {
        name: &'static str,
        delay: Duration,
        output: &'static str,
    }

    impl Capability for SleepTool {
        fn name(&self) -> &'static str {
            self.name
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name.into(),
                description: "sleeps then returns output".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, args: Value) -> Result<String, ToolError> {
            std::thread::sleep(self.delay);
            if let Some(tag) = args.get("tag").and_then(|v| v.as_str()) {
                Ok(format!("{}:{tag}", self.output))
            } else {
                Ok(self.output.into())
            }
        }
    }

    struct PanicTool;
    impl Capability for PanicTool {
        fn name(&self) -> &'static str {
            "panic_tool"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "always panics".into(),
                parameters: json!({"type":"object","properties":{}}),
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            panic!("boom panic inside tool");
        }
    }

    fn fast_engine(threshold_ms: u64) -> ToolExecutionEngine {
        ToolExecutionEngine::new_with_threshold(
            ToolExecutionPolicy::default(),
            Duration::from_millis(threshold_ms),
        )
    }

    fn extract_job_id(pending_msg: &str) -> String {
        let marker = "Job ID: ";
        let start = pending_msg
            .find(marker)
            .expect("expected 'Job ID: ' in pending message")
            + marker.len();
        let rest = &pending_msg[start..];
        let end = rest.find('.').expect("expected '.' after Job ID");
        rest[..end].to_string()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn slow_tool_returns_job_id_and_later_wait_job_fetches_result_once() {
        let engine = fast_engine(40);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(SleepTool {
            name: "slow_op",
            delay: Duration::from_millis(120),
            output: "slow-finished-payload",
        })]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("slow_op", "call_slow", json!({}))),
            )
            .await;

        let job_id = match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_slow");
                assert_eq!(rs[0].name, "slow_op");
                assert!(
                    rs[0].result.contains("Still running in the background after"),
                    "got: {}",
                    rs[0].result
                );
                assert!(rs[0].result.contains("wait_job"));
                extract_job_id(&rs[0].result)
            }
            other => panic!("expected ToolResults, got {other:?}"),
        };

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call(
                    "wait_job",
                    "call_wait_1",
                    json!({ "job_ids": [job_id] }),
                )),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_wait_1");
                assert_eq!(rs[0].name, "wait_job");
                assert_eq!(
                    rs[0].result,
                    format!("Job {job_id}: completed\nslow-finished-payload")
                );
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call(
                    "wait_job",
                    "call_wait_2",
                    json!({ "job_ids": [job_id] }),
                )),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(
                    rs[0].result,
                    format!("Job {job_id}: not found (unknown ID, or result already read).")
                );
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn wait_job_rejected_when_called_with_other_calls_while_others_still_run() {
        let engine = fast_engine(40);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        let mut batch = AgentResponse::new();
        batch.push(call("echo", "call_echo", json!({"v": 1})));
        batch.push(call(
            "wait_job",
            "call_wait",
            json!({"job_ids": ["some-id"]}),
        ));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, batch)
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 2);
                assert_eq!(rs[0].id, "call_echo");
                assert!(rs[0].result.starts_with("echo:"));

                assert_eq!(rs[1].id, "call_wait");
                assert_eq!(rs[1].name, "wait_job");
                assert!(
                    rs[1].result.contains(
                        "`wait_job` must be called on its own, with no other calls in the same turn."
                    ),
                    "unexpected message: {}",
                    rs[1].result
                );
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn wait_job_alone_does_not_pay_tool_threshold() {
        let engine = fast_engine(2000);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        let start = std::time::Instant::now();
        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call(
                    "wait_job",
                    "call_wait",
                    json!({"job_ids": ["missing-job"]}),
                )),
            )
            .await;

        assert!(
            start.elapsed() < Duration::from_millis(300),
            "wait_job alone should not wait for tool threshold, took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn panicking_tool_returns_error_result_and_does_not_crash_engine() {
        let engine = fast_engine(50);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(PanicTool), Box::new(Echo)]);

        let mut batch = AgentResponse::new();
        batch.push(call("panic_tool", "call_panic", json!({})));
        batch.push(call("echo", "call_echo", json!({"ok": true})));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, batch)
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 2);
                assert_eq!(rs[0].id, "call_panic");
                assert!(
                    rs[0]
                        .result
                        .contains("Tool 'panic_tool' failed: Task panicked during execution"),
                    "got: {}",
                    rs[0].result
                );
                assert_eq!(rs[1].id, "call_echo");
                assert!(rs[1].result.starts_with("echo:"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn mixed_batch_fast_slow_signal_fast_preserves_exact_order() {
        let engine = fast_engine(50);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent_builder(vec![
            Box::new(Echo),
            Box::new(SleepTool {
                name: "slow_tool",
                delay: Duration::from_millis(180),
                output: "eventual",
            }),
        ])
        .with_signals(vec![Box::new(UpdateScratchpadSignal)])
        .build();

        let mut batch = AgentResponse::new();
        batch.push(call("echo", "c0", json!({"n": 1})));
        batch.push(call("slow_tool", "c1", json!({})));
        batch.push(call("update_scratchpad", "c2", valid_scratchpad_args()));
        batch.push(call("echo", "c3", json!({"n": 2})));

        engine
            .dispatch_tool_batch(&mut session, &mut agent, batch)
            .await;

        let job_id = match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 4);
                assert_eq!(rs[0].id, "c0");
                assert_eq!(rs[0].result, "echo:{\"n\":1}");

                assert_eq!(rs[1].id, "c1");
                assert!(rs[1].result.contains("Still running in the background after"));

                assert_eq!(rs[2].id, "c2");
                assert_eq!(rs[2].result, "scratchpad updated successfully");

                assert_eq!(rs[3].id, "c3");
                assert_eq!(rs[3].result, "echo:{\"n\":2}");

                extract_job_id(&rs[1].result)
            }
            other => panic!("expected ToolResults, got {other:?}"),
        };

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("wait_job", "c4", json!({"job_ids": [job_id]}))),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].result, format!("Job {job_id}: completed\neventual"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tools_in_one_batch_run_concurrently_and_same_named_tools_keep_order() {
        let engine = fast_engine(200);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(SleepTool {
            name: "work",
            delay: Duration::from_millis(120),
            output: "done",
        })]);

        let mut batch = AgentResponse::new();
        batch.push(call("work", "id_first", json!({"tag": "A"})));
        batch.push(call("work", "id_second", json!({"tag": "B"})));

        let start = std::time::Instant::now();
        engine
            .dispatch_tool_batch(&mut session, &mut agent, batch)
            .await;
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(320),
            "Expected concurrent execution near 200ms threshold, took {elapsed:?}"
        );

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs.len(), 2);
                assert_eq!(rs[0].id, "id_first");
                assert_eq!(rs[0].result, "done:A");
                assert_eq!(rs[1].id, "id_second");
                assert_eq!(rs[1].result, "done:B");
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn same_named_slow_tools_across_turns_get_distinct_job_ids_and_merge_in_requested_order()
    {
        let engine = fast_engine(30);
        let mut session = AgentSession::new(10);
        let mut agent = test_agent(vec![Box::new(SleepTool {
            name: "bash_sim",
            delay: Duration::from_millis(120),
            output: "out",
        })]);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bash_sim", "t1_call", json!({"tag": "turn1"}))),
            )
            .await;
        let id_turn1 = match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => extract_job_id(&rs[0].result),
            other => panic!("expected ToolResults, got {other:?}"),
        };

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bash_sim", "t2_call", json!({"tag": "turn2"}))),
            )
            .await;
        let id_turn2 = match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => extract_job_id(&rs[0].result),
            other => panic!("expected ToolResults, got {other:?}"),
        };

        assert_ne!(id_turn1, id_turn2);

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call(
                    "wait_job",
                    "t3_wait",
                    json!({"job_ids": [id_turn2.clone(), id_turn1.clone()]}),
                )),
            )
            .await;

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(
                    rs[0].result,
                    format!(
                        "Job {id_turn2}: completed\nout:turn2\n\nJob {id_turn1}: completed\nout:turn1"
                    )
                );
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn permission_prompt_does_not_stall_already_submitted_background_jobs() {
        let engine = fast_engine(30);
        let mut session = AgentSession::new(10);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let mut agent = test_agent_builder(vec![
            Box::new(SleepTool {
                name: "bg_job",
                delay: Duration::from_millis(100),
                output: "bg-done",
            }),
            Box::new(Dangerous),
        ])
        .with_events_streaming(tx)
        .build();

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bg_job", "c_bg", json!({}))),
            )
            .await;
        let bg_id = match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => extract_job_id(&rs[0].result),
            other => panic!("expected ToolResults, got {other:?}"),
        };

        let approval_handle = tokio::spawn(async move {
            while let Some(evt) = rx.recv().await {
                if let crate::agent::agents::AgentEvent::ToolApprovalRequest { responder, .. } = evt
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let _ = responder.send(PermissionDecision::Approved);
                    break;
                }
            }
        });

        engine
            .dispatch_tool_batch(
                &mut session,
                &mut agent,
                AgentResponse::single(call("bash", "c_mut", json!({}))),
            )
            .await;
        approval_handle.await.unwrap();

        assert_eq!(
            engine.job_store().get(&bg_id).await,
            Some(crate::multithreading::JobState::Completed("bg-done".into()))
        );
    }

    #[tokio::test]
    async fn segmented_partition_batch_separates_signals_tools_and_immediate_rejections() {
        let engine = fast_engine(30);
        let mut agent = test_agent_builder(vec![Box::new(Echo)])
            .with_signals(vec![Box::new(UpdateScratchpadSignal)])
            .build();

        let calls = vec![
            call("echo", "c0", json!({})),
            call("wait_job", "c1", json!({"job_ids": ["j1"]})),
            call("update_scratchpad", "c2", valid_scratchpad_args()),
            call("unknown_tool", "c3", json!({})),
        ];

        let partitioned = engine.partition_batch(&calls, &mut agent);
        assert_eq!(partitioned.tools.len(), 1);
        assert_eq!(partitioned.tools[0].pos, 0);

        assert_eq!(partitioned.signals.len(), 1);
        assert_eq!(partitioned.signals[0].pos, 2);

        assert_eq!(partitioned.rejected.len(), 2);
        assert_eq!(partitioned.rejected[0].0, 1);
        assert!(partitioned.rejected[0].1.result.contains("must be called on its own"));
        assert_eq!(partitioned.rejected[1].0, 3);
        assert!(partitioned.rejected[1].1.result.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn segmented_resolve_job_outcome_formats_completed_failed_and_pending() {
        let engine = fast_engine(1000);
        let mut agent = test_agent(vec![Box::new(Echo)]);
        let c = call("echo", "call_id_1", json!({}));

        let ok_res = engine.tool_dispatcher.resolve_job_outcome(
            "job-1",
            Some(crate::multithreading::JobState::Completed("yay".into())),
            &c,
            &mut agent,
        );
        assert_eq!(ok_res.id, "call_id_1");
        assert_eq!(ok_res.result, "yay");

        let err_res = engine.tool_dispatcher.resolve_job_outcome(
            "job-2",
            Some(crate::multithreading::JobState::Failed("bad".into())),
            &c,
            &mut agent,
        );
        assert_eq!(err_res.result, "Tool 'echo' failed: bad");

        let pending_res = engine.tool_dispatcher.resolve_job_outcome(
            "job-3",
            Some(crate::multithreading::JobState::Running),
            &c,
            &mut agent,
        );
        assert!(pending_res.result.contains("Still running in the background after 1s. Job ID: job-3."));
    }
}

