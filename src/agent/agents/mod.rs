mod contexts;
pub mod hooks;
mod prompts;
use std::vec;

use crate::agent::archtectures::oneshot::OneShot;
use crate::agent::archtectures::react::ReactLoop;
use crate::agent::error::AgentError;
use crate::core::capability::{Capability, ToolRegistry};
use crate::core::llm_client::LLMProvider;
use crate::core::model::Model;
use crate::core::session::AgentSession;
use crate::tools::bash::Bash;
use crate::tools::docker::Docker;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::json::Json;
use crate::tools::last_error::ReadLastError;
use crate::tools::read_dir::ReadDir;
use crate::tools::read_file::ReadFile;
use crate::utils::FlatSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub struct Agent<'a, P: LLMProvider> {
    runner: &'a mut ReactLoop<P>,
    registry: ToolRegistry,
    system_prompt: &'static str,
    model: Model,
    context: Option<String>,
}

impl<'a, P: LLMProvider> Agent<'a, P> {
    // base constructor: empty registry, filled in by with_tools
    fn base(runner: &'a mut ReactLoop<P>, system_prompt: &'static str, model: Model) -> Self {
        Self {
            runner,
            registry: ToolRegistry::new(vec![]),
            system_prompt,
            model,
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

    pub fn planner(runner: &'a mut ReactLoop<P>, model: Model) -> Self {
        Self::base(runner, prompts::PLANNER_SYS_PROMPT, model)
            .with_tools(vec![Box::new(ReadDir)])
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn executor(runner: &'a mut ReactLoop<P>, model: Model) -> Self {
        Self::base(runner, prompts::EXECUTOR_SYS_PROMPT, model)
            .with_tools(vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)])
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn architect(runner: &'a mut ReactLoop<P>, model: Model) -> Self {
        Self::base(runner, prompts::ARCHITECT_SYS_PROMPT, model)
            .with_tools(vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)])
            .with_context(&contexts::ShellEnv::gather())
    }

    pub fn cmd_predictor(runner: &'a mut ReactLoop<P>, model: Model, scheema: Value) -> Self {
        Self::base(runner, prompts::CMD_PREDICTOR_SYS_PROMPT, model)
            .with_tools(vec![
                Box::new(GitDiffStaged),
                Box::new(Docker),
                Box::new(Json {
                    properties: scheema,
                }),
                Box::new(ReadLastError),
            ])
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
            .run::<T>(&mut session, &self.registry, &self.model)
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
