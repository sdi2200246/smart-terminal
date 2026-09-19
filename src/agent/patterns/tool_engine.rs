use crate::agent::agents::Agent;
use crate::core::responce::{AgentResponse, AgentToolCall};
use crate::core::session::{AgentSession, ToolCall, ToolResult};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct ToolExecutionEngine {
    events_stream: Option<UnboundedSender<AgentToolCall>>,
}

impl ToolExecutionEngine {
    pub fn new() -> Self {
        Self { events_stream: None }
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<AgentToolCall>) -> Self {
        self.events_stream = Some(tx);
        self
    }

    pub fn dispatch_tool_batch(
        &self,
        session: &mut AgentSession,
        agent: &mut Agent,
        response: AgentResponse,
    ) {
        let calls = response.into_calls();

        for call in &calls {
            agent.hooks.on_tool_received(call);
        }

        session.add_tool_calls(
            calls
                .iter()
                .map(|c| {
                    ToolCall::new(c.name(), c.arguments().clone(), c.id())
                        .with_thinking_state(c.thinking_state())
                })
                .collect(),
        );

        let mut results = Vec::with_capacity(calls.len());
        let mut final_answer: Option<Value> = None;

        for call in &calls {
            let payload = match self.execute_tool(agent, call) {
                Ok(result) => {
                    if call.name() == "final_answer" {
                        final_answer = Some(call.arguments().clone());
                    } else if let Some(stream) = &self.events_stream {
                        let _ = stream.send(call.clone());
                    }
                    result
                }
                Err(e) => format!("Tool '{}' failed: {}", call.name(), e),
            };
            results.push(ToolResult::new(call.name(), payload, call.id()));
        }

        session.add_tool_results(results);

        if let Some(value) = final_answer {
            session.set_final_answer(value);
        }
    }

    fn execute_tool(&self, agent: &mut Agent, call: &AgentToolCall) -> Result<String, String> {
        match agent
            .registry
            .get(call.name())
            .expect("correct_tool_name")
            .execute(call.arguments().clone())
        {
            Ok(result) => Ok(result),
            Err(e) => {
                let msg = e.to_string();
                agent.hooks.on_tool_failed(call, &msg);
                Err(msg)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::Agent;
    use crate::core::capability::{Capability, ToolMetaData};
    use crate::core::model::{Model, ModelName};
    use crate::core::session::ConversationEvent;
    use crate::tools::error::ToolError;
    use serde_json::json;

    struct Echo;
    impl Capability for Echo {
        fn name(&self) -> &'static str { "echo" }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "echoes args".into(),
                parameters: json!({"type":"object","properties":{}}),
            }
        }
        fn execute(&self, args: Value) -> Result<String, ToolError> {
            Ok(format!("echo:{args}"))
        }
    }

    struct Boom;
    impl Capability for Boom {
        fn name(&self) -> &'static str { "boom" }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "always fails".into(),
                parameters: json!({"type":"object","properties":{}}),
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Err(ToolError::ToolExecution { source: anyhow::anyhow!("boom failed") })
        }
    }

    struct FinalAnswer;
    impl Capability for FinalAnswer {
        fn name(&self) -> &'static str { "final_answer" }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "submits final answer".into(),
                parameters: json!({"type":"object","properties":{}}),
            }
        }
        fn execute(&self, args: Value) -> Result<String, ToolError> {
            Ok(args.to_string())
        }
    }

    fn test_agent(tools: Vec<Box<dyn Capability>>) -> Agent {
        Agent::base("test prompt", Model::with_default_temp(ModelName::GptOss120B)).with_tools(tools)
    }

    fn call(name: &str, id: &str, args: Value) -> AgentToolCall {
        AgentToolCall::new(name.into(), id.into(), args, None)
    }

    #[test]
    fn successful_tool_produces_matching_result() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(call("echo", "call_1", json!({"x": 1}))));

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert_eq!(rs[0].id, "call_1");
                assert!(rs[0].result.starts_with("echo:"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[test]
    fn failed_tool_wraps_error_message_in_result() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Boom)]);

        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(call("boom", "call_1", json!({}))));

        match session.events().last().unwrap() {
            ConversationEvent::ToolResults(rs) => {
                assert!(rs[0].result.contains("Tool 'boom' failed"));
                assert!(rs[0].result.contains("boom failed"));
            }
            other => panic!("expected ToolResults, got {other:?}"),
        }
    }

    #[test]
    fn final_answer_call_sets_session_final_answer() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(FinalAnswer)]);

        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(call("final_answer", "call_1", json!({"result": "42"}))));

        assert_eq!(session.take_final_answer(), Some(json!({"result": "42"})));
    }

    #[test]
    fn non_final_tool_call_is_streamed_when_configured() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new().with_events_streaming(tx);
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(call("echo", "call_1", json!({}))));

        assert_eq!(rx.try_recv().expect("expected a streamed call").name(), "echo");
    }

    #[test]
    fn final_answer_call_is_not_streamed() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = ToolExecutionEngine::new().with_events_streaming(tx);
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(FinalAnswer)]);

        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(call("final_answer", "call_1", json!({"result": "ok"}))));

        assert!(rx.try_recv().is_err(), "final_answer should not be streamed");
    }

    #[test]
    fn thinking_state_is_carried_into_tool_call_event() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo)]);

        let signed = AgentToolCall::new("echo".into(), "call_1".into(), json!({}), Some("sig-a".into()));
        engine.dispatch_tool_batch(&mut session, &mut agent, AgentResponse::single(signed));

        let calls = session.events().iter().find_map(|e| match e {
            ConversationEvent::ToolCalls(c) => Some(c),
            _ => None,
        }).expect("ToolCalls event present");
        assert_eq!(calls[0].thinking_state, Some("sig-a".into()));
    }

    #[test]
    fn multiple_calls_produce_matching_ordered_results() {
        let engine = ToolExecutionEngine::new();
        let mut session = AgentSession::new(5);
        let mut agent = test_agent(vec![Box::new(Echo), Box::new(Boom)]);

        let mut response = AgentResponse::new();
        response.push(call("echo", "call_1", json!({})));
        response.push(call("boom", "call_2", json!({})));
        engine.dispatch_tool_batch(&mut session, &mut agent, response);

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
}