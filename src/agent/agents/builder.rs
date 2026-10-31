use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use super::contexts;
use super::events::AgentEvent;
use super::hooks::{DefaultAgentHook, ToolsRegulator};
use super::Agent;
use crate::agent::patterns::hook::AgentLoopHook;
use crate::agent::prompts::{
    AgentPrompt, CMD_PREDICTOR_PROMPT, EXECUTOR_PROMPT, PLANNER_PROMPT, SystemPromptBuilder,
};
use crate::agent::signals::{
    AgentAction, AgentActionSpace, AgentSignal, FinalAnswerSignal, UpdateScratchpadSignal,
};
use crate::core::capability::Capability;
use crate::core::model::Model;

enum PromptSource {
    Raw(String),
    Template(AgentPrompt),
}

pub struct AgentBuilder {
    action_space: AgentActionSpace,
    prompt_source: PromptSource,
    model: Model,
    hooks: Box<dyn AgentLoopHook>,
    context: Option<String>,
    event_stream: Option<UnboundedSender<AgentEvent>>,
}

impl AgentBuilder {
    pub fn base(system_prompt: impl Into<String>, model: Model) -> Self {
        Self {
            action_space: AgentActionSpace::new(),
            prompt_source: PromptSource::Raw(system_prompt.into()),
            model,
            hooks: Box::new(DefaultAgentHook),
            context: None,
            event_stream: None,
        }
    }

    pub fn for_prompt(prompt: AgentPrompt, model: Model) -> Self {
        Self {
            action_space: AgentActionSpace::new(),
            prompt_source: PromptSource::Template(prompt),
            model,
            hooks: Box::new(DefaultAgentHook),
            context: None,
            event_stream: None,
        }
    }

    pub fn planner(model: Model, tools: Vec<Box<dyn Capability>>, output_schema: Value) -> Self {
        Self::for_prompt(PLANNER_PROMPT, model)
            .with_tools(tools)
            .with_signals(vec![Box::new(FinalAnswerSignal {
                properties: output_schema,
            })])
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn executor(model: Model, tools: Vec<Box<dyn Capability>>, output_schema: Value) -> Self {
        Self::for_prompt(EXECUTOR_PROMPT, model)
            .with_tools(tools)
            .with_signals(vec![
                Box::new(UpdateScratchpadSignal),
                Box::new(FinalAnswerSignal {
                    properties: output_schema,
                }),
            ])
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn cmd_predictor(
        model: Model,
        tools: Vec<Box<dyn Capability>>,
        output_schema: Value,
    ) -> Self {
        Self::for_prompt(CMD_PREDICTOR_PROMPT, model)
            .with_tools(tools)
            .with_signals(vec![Box::new(FinalAnswerSignal {
                properties: output_schema,
            })])
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn with_action_space(mut self, action_space: AgentActionSpace) -> Self {
        self.action_space = action_space;
        self
    }

    pub fn with_tools(mut self, tools: Vec<Box<dyn Capability>>) -> Self {
        for tool in tools {
            self.action_space
                .register_tool(Arc::from(tool))
                .expect("duplicate tool registration");
        }
        self
    }

    pub fn with_signals(mut self, signals: Vec<Box<dyn AgentSignal>>) -> Self {
        for sig in signals {
            self.action_space
                .register_signal(Arc::from(sig))
                .expect("duplicate signal registration");
        }
        self
    }

    pub fn with_actions(mut self, actions: Vec<AgentAction>) -> Self {
        for item in actions {
            match item {
                AgentAction::Tool(cap) => self
                    .action_space
                    .register_tool(cap)
                    .expect("duplicate tool registration"),
                AgentAction::Signal(sig) => self
                    .action_space
                    .register_signal(sig)
                    .expect("duplicate signal registration"),
            }
        }
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

    pub fn with_events_streaming_opt(mut self, tx: Option<UnboundedSender<AgentEvent>>) -> Self {
        self.event_stream = tx;
        self
    }

    pub fn build(self) -> Agent {
        let system_prompt = match self.prompt_source {
            PromptSource::Raw(raw) => raw,
            PromptSource::Template(prompt) => {
                let schemas = self.action_space.model_schemas();
                SystemPromptBuilder::new(prompt)
                    .available_tools(&schemas)
                    .tool_guidance(self.action_space.prompt_guidance())
                    .build()
            }
        };

        Agent {
            action_space: self.action_space,
            system_prompt,
            model: self.model,
            hooks: self.hooks,
            context: self.context,
            event_stream: self.event_stream,
        }
    }
}

