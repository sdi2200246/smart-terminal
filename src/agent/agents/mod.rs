mod prompts;
mod contexts;
use crate::agent::loops::ReactLoop;
use crate::core::capability::{Capability, ToolRegistry, ToolMetaData};
use crate::core::session::{Model , AgentSession};
use crate::core::llm_client::LLMProvider;
use crate::tools::read_dir::ReadDir;
use crate::tools::bash::Bash;
use crate::tools::read_file::ReadFile;
use crate:: utils::FlatSchema;
use crate::agent::error::AgentError;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Agent<'a, P: LLMProvider> {
    runner: &'a mut  ReactLoop<P>,
    registry: ToolRegistry,
    tools_metadata: Vec<ToolMetaData>,
    system_prompt: &'static str,
    model: Model,
    context:Option<String>
}

impl<'a, P: LLMProvider> Agent<'a, P> {
    
    pub fn new(
        runner: &'a mut  ReactLoop<P>,
        registry: ToolRegistry,
        tools_metadata: Vec<ToolMetaData>,
        system_prompt: &'static str,
        model: Model,
    ) -> Self {
        Self { runner, registry, tools_metadata, system_prompt, model , context:None}
    }

    pub fn with_context<C: Serialize>(mut self, ctx: &C) -> Self {
        self.context = Some(serde_json::to_string_pretty(ctx).expect("context serializes"));
        self
    }


    pub fn planner(runner: &'a mut ReactLoop<P>, model: Model) -> Self {
        let read_dir = Box::new(ReadDir) as Box<dyn Capability>;

        let tools_metadata = vec![read_dir.metadata()];

        let mut registry = ToolRegistry::new();
        registry.insert(read_dir.name(), read_dir);

        Self::new(runner, registry, tools_metadata, prompts::PLANNER_SYS_PROMPT, model)
        .with_context(&contexts::ShellEnv::gather())
    }

    pub fn executor(runner: &'a mut ReactLoop<P>, model: Model) -> Self {
        let read_dir  = Box::new(ReadDir)  as Box<dyn Capability>;
        let bash      = Box::new(Bash)     as Box<dyn Capability>;
        let read_file = Box::new(ReadFile) as Box<dyn Capability>;

        let tools_metadata = vec![
            read_dir.metadata(),
            bash.metadata(),
            read_file.metadata(),
        ];

        let mut registry = ToolRegistry::new();
        registry.insert(read_dir.name(),  read_dir);
        registry.insert(bash.name(),      bash);
        registry.insert(read_file.name(), read_file);

        Self::new(runner, registry, tools_metadata, prompts::EXECUTOR_SYS_PROMPT, model)
        .with_context(&contexts::ShellEnv::gather())
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

        self.runner
            .run::<T>(&mut session, &self.registry, &self.tools_metadata, &self.model)
            .await
    }
}