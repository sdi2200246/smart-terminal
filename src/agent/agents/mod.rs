mod contexts;
pub mod hooks;
mod prompts;
use std::vec;

use crate::agent::agents::hooks::{DefaultAgentHook, ToolsRegulator};
use crate::agent::patterns::hook::AgentLoopHook;
use crate::core::capability::{Capability, ToolRegistry};
use crate::core::model::Model;
use crate::core::responce::AgentToolCall;
use crate::core::session::AgentSession;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

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

pub struct Agent {
    pub registry: ToolRegistry,
    pub system_prompt: &'static str,
    pub model: Model,
    pub hooks: Box<dyn AgentLoopHook>,
    pub context: Option<String>,
    event_stream: Option<UnboundedSender<AgentEvent>>,
}

impl Agent {

    pub fn base(system_prompt: &'static str, model: Model) -> Self {
        Self {
            registry: ToolRegistry::new(vec![]),
            system_prompt,
            model,
            hooks: Box::new(DefaultAgentHook),
            context: None,
            event_stream: None,
        }
    }
    
    pub fn with_tools(mut self, tools: Vec<Box<dyn Capability>>) -> Self {
        self.registry = ToolRegistry::new(tools);
        self
    }

    pub fn with_context<C: Serialize>(mut self, ctx: &C) -> Self {
        self.context = Some(serde_json::to_string_pretty(ctx).expect("context serializes"));
        self
    }

    pub fn with_hook(mut self, hook: Box<dyn AgentLoopHook>) -> Self {
        self.hooks = hook;
        self
    }

    pub fn with_events_streaming(mut self, tx: UnboundedSender<AgentEvent>) -> Self {
        self.event_stream = Some(tx);
        self
    }

    pub(crate) fn stream_tool_call(&self, call: &AgentToolCall) {
        if let Some(stream) = &self.event_stream {
            if let Err(error) = stream.send(AgentEvent::ToolCall(call.clone())) {
                tracing::warn!(tool = %call.name(), error = %error, "failed to stream agent tool event");
            }
        }
    }

    pub(crate) fn update_state(&self, state: AgentState) {
        if let Some(stream) = &self.event_stream {
            if let Err(error) = stream.send(AgentEvent::StateUpdate(state)) {
                tracing::warn!(state = state.label(), error = %error, "failed to stream agent state");
            }
        }
    }

    pub fn planner(model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(prompts::PLANNER_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn executor(model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(prompts::EXECUTOR_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn architect(model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(prompts::ARCHITECT_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn cmd_predictor(
        model: Model,
        tools: Vec<Box<dyn Capability>>,
    ) -> Self {
        Self::base(prompts::CMD_PREDICTOR_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn build_session(&self, user_prompt: impl Into<String>) -> AgentSession {
        let mut builder = AgentSession::builder().system(self.system_prompt);
        if let Some(ctx) = &self.context {
            builder = builder.context(ctx);
        }
        builder.user(user_prompt).build()
    }
}

pub struct OneShotAgent {
    pub system_prompt: &'static str,
    pub context: Option<String>,
}

impl OneShotAgent {
    pub fn new(sys_promt: &'static str) -> Self {
        Self {
            system_prompt: sys_promt,
            context: None,
        }
    }

    pub fn with_context<C: Serialize>(mut self, ctx: &C) -> Self {
        self.context = Some(serde_json::to_string_pretty(ctx).expect("context serializes"));
        self
    }

    pub fn script_generator() -> Self {
        Self::new(prompts::GENERATOR_SYS_PROMPT).with_context(&contexts::ShellEnv::gather())
    }

    pub fn build_session(&self, user_prompt: impl Into<String>) -> AgentSession {
        let mut builder = AgentSession::builder().system(self.system_prompt);
        if let Some(ctx) = &self.context {
            builder = builder.system(format!("Context:\n{}", ctx));
        }
        builder.user(user_prompt).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{Model, ModelName};

    #[test]
    fn state_updates_are_sent_to_the_presenter() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let agent = Agent::base("test prompt", Model::with_default_temp(ModelName::GptOss120B))
            .with_events_streaming(tx);

        agent.update_state(AgentState::ExecutingTool);

        assert!(matches!(
            rx.try_recv(),
            Ok(AgentEvent::StateUpdate(AgentState::ExecutingTool))
        ));
    }
}