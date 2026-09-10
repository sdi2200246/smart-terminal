mod contexts;
pub mod hooks;
mod prompts;
use std::vec;

use crate::agent::agents::hooks::{DefaultAgentHook, ToolsRegulator};
use crate::agent::error::AgentError;
use crate::agent::patterns::hook::AgentLoopHook;
use crate::agent::patterns::oneshot::OneShot;
use crate::agent::patterns::react::ReactLoop;
use crate::core::capability::{Capability, ToolRegistry};
use crate::core::llm_client::LLMProvider;
use crate::core::model::Model;
use crate::core::session::AgentSession;
use crate::utils::FlatSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct Agent<P: LLMProvider> {
    runner: ReactLoop<P>,
    registry: ToolRegistry,
    system_prompt: &'static str,
    model: Model,
    hooks: Box<dyn AgentLoopHook>,
    context: Option<String>,
}

impl<P: LLMProvider> Agent<P> {
    // base constructor: empty registry, filled in by with_tools
    fn base(runner: ReactLoop<P>, system_prompt: &'static str, model: Model) -> Self {
        Self {
            runner,
            registry: ToolRegistry::new(vec![]),
            system_prompt,
            model,
            hooks: Box::new(DefaultAgentHook),
            context: None,
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

    pub fn planner(runner: ReactLoop<P>, model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(runner, prompts::PLANNER_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn executor(runner: ReactLoop<P>, model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(runner, prompts::EXECUTOR_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
            .with_hook(Box::new(ToolsRegulator::new()))
    }

    pub fn architect(runner: ReactLoop<P>, model: Model, tools: Vec<Box<dyn Capability>>) -> Self {
        Self::base(runner, prompts::ARCHITECT_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn cmd_predictor(
        runner: ReactLoop<P>,
        model: Model,
        tools: Vec<Box<dyn Capability>>,
    ) -> Self {
        Self::base(runner, prompts::CMD_PREDICTOR_SYS_PROMPT, model)
            .with_tools(tools)
            .with_context(&contexts::ShellEnv::gather())
    }

    pub async fn run<T>(&mut self, user_prompt: impl Into<String>) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        let mut builder = AgentSession::builder().system(self.system_prompt);
        if let Some(ctx) = &self.context {
            builder = builder.context(ctx);
        }
        let mut session = builder.user(user_prompt).build();

        self.runner
            .run::<T>(&mut session, &self.registry, &self.model, &mut self.hooks)
            .await
    }
}

pub struct OneShotAgent<'a, P: LLMProvider> {
    runner: &'a mut OneShot<P>,
    system_prompt: &'static str,
    context: Option<String>,
}

impl<'a, P: LLMProvider> OneShotAgent<'a, P> {
    pub fn new(runner: &'a mut OneShot<P>, sys_promt: &'static str) -> Self {
        Self {
            runner,
            system_prompt: sys_promt,
            context: None,
        }
    }

    pub fn with_context<C: Serialize>(mut self, ctx: &C) -> Self {
        self.context = Some(serde_json::to_string_pretty(ctx).expect("context serializes"));
        self
    }

    pub fn script_generator(runner: &'a mut OneShot<P>) -> Self {
        Self::new(runner, prompts::GENERATOR_SYS_PROMPT).with_context(&contexts::ShellEnv::gather())
    }

    pub async fn run<T>(&mut self, user_prompt: impl Into<String>) -> Result<T, AgentError>
    where
        T: FlatSchema + DeserializeOwned,
    {
        let mut builder = AgentSession::builder().system(self.system_prompt);
        if let Some(ctx) = &self.context {
            builder = builder.system(format!("Context:\n{}", ctx));
        }
        let mut session = builder.user(user_prompt).build();

        self.runner.run::<T>(&mut session).await
    }
}
