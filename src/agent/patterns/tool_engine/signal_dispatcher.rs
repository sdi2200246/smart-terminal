use crate::agent::agents::Agent;
use crate::agent::signals::{AgentLoopControl, AgentSignalContext, AgentSignalInvocation};
use crate::core::responce::AgentToolCall;
use crate::core::session::{AgentSession, ToolResult};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct SignalDispatcher;

impl SignalDispatcher {
    pub fn run(
        &self,
        signals: Vec<AgentSignalInvocation>,
        calls: &[AgentToolCall],
        session: &mut AgentSession,
        agent: &mut Agent,
    ) -> (Vec<(usize, ToolResult)>, Option<Value>) {
        let mut out = Vec::with_capacity(signals.len());
        let mut final_answer = None;

        for call in signals {
            let orig = &calls[call.pos];
            // Reborrow, do not move: session and agent are used again on the next iteration.
            let mut ctx = AgentSignalContext {
                session: &mut *session,
                agent: &mut *agent,
            };
            match call.sig.handle(call.args, &mut ctx) {
                Ok(o) => {
                    out.push((call.pos, ToolResult::new(orig.name(), o.result, orig.id())));
                    if let AgentLoopControl::Finish(v) = o.control {
                        final_answer = Some(v);
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    agent.hooks.on_tool_failed(orig, &err_str);
                    let msg = format!("Tool '{}' failed: {}", orig.name(), err_str);
                    agent.hooks.on_tool_failed(orig, &msg);
                    out.push((call.pos, ToolResult::new(orig.name(), msg, orig.id())));
                }
            }
        }
        (out, final_answer)
    }
}

