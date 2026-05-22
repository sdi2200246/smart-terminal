use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::agents::Agent;
use crate::agent::archtectures::react::ReactLoop;
use crate::agent::error::AgentError;
use crate::core::llm_client::LLMProvider;
use crate::core::memory::{Conversation, Interaction, Memory};
use crate::core::session::{Model, ModelName};
use crate::utils::FlatSchema;

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum Reversibility {
    Full,
    Mostly,
    Partial,
    Hard,
    Irreversible,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct NextCommand {
    /// The exact shell command to run. No backticks, no `$ ` prefix.
    pub cmd: String,
    /// One short sentence describing what the command does and any assumption made. Under 15 words.
    pub man: String,
    /// How reversible the command is, given the current environment.
    pub scale: Reversibility,
}
impl FlatSchema for NextCommand {}

pub struct NextCmd<'a, P: LLMProvider, M: Memory> {
    runner: &'a mut ReactLoop<P>,
    memory: &'a mut M,
}

impl<'a, P: LLMProvider, M: Memory> NextCmd<'a, P, M> {
    pub fn new(runner: &'a mut ReactLoop<P>, memory: &'a mut M) -> Self {
        Self { runner, memory }
    }

    pub async fn run(&mut self, input: impl Into<String>) -> Result<NextCommand, AgentError> {
        let input = input.into();

        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let loaded = self.memory.load(&cwd).unwrap_or(false);
        let history = if loaded { self.memory.current() } else { None };

        let user_prompt = build_user_prompt(&input, history);

        let prediction: NextCommand = {
            let mut predictor = Agent::cmd_predictor(
                &mut *self.runner,
                Model::creative(ModelName::GptOss120B),
            );
            predictor.run(user_prompt).await?
        };

        if loaded {
            let entry = Interaction {
                user_input: input.clone(),
                predicted_cmd: prediction.cmd.clone(),
                timestamp: now_secs(),
            };
            if let Err(e) = self.memory.append(entry) {
                tracing::warn!(error = %e, "failed to persist interaction");
            }
        }

        Ok(prediction)
    }
}

fn build_user_prompt(input: &str, history: Option<&Conversation>) -> String {
    let mut out = String::new();
    if let Some(conv) = history {
        if !conv.interactions.is_empty() {
            out.push_str("Recent interactions in this project (most recent last):\n");
            for entry in &conv.interactions {
                out.push_str(&format!(
                    "  user: {}\n  cmd:  {}\n",
                    entry.user_input.trim(),
                    entry.predicted_cmd
                ));
            }
            out.push('\n');
        }
    }
    out.push_str("User input:\n");
    out.push_str(input);
    out
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::memory::FolderMemory;
    use crate::core::error::ProviderError;
    use crate::core::llm_client::AgentRequest;
    use crate::core::session::{AgentSession, AgentToolCall, ConversationEvent};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    struct MockProvider {
        captured: Arc<Mutex<Vec<String>>>,
        canned_cmd: String,
    }

    impl MockProvider {
        fn new(cmd: impl Into<String>) -> (Self, Arc<Mutex<Vec<String>>>) {
            let captured = Arc::new(Mutex::new(Vec::new()));
            (
                Self { captured: captured.clone(), canned_cmd: cmd.into() },
                captured,
            )
        }
    }

    impl LLMProvider for MockProvider {
        async fn complete(&mut self, request: AgentRequest<'_>) -> Result<AgentToolCall, ProviderError> {
            let last_user = request
                .session
                .events
                .iter()
                .rev()
                .find_map(|e| match e {
                    ConversationEvent::User(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            self.captured.lock().unwrap().push(last_user);
            Ok(AgentToolCall::new("stop".into(), "".into(), Value::String("done".into())))
        }

        async fn complete_structured(
            &mut self,
            _session: &AgentSession,
            _schema: Value,
        ) -> Result<Value, ProviderError> {
            Ok(json!({
                "cmd": self.canned_cmd,
                "man": "test prediction",
                "scale": "Full",
            }))
        }
    }

    fn seed_interaction(input: &str, cmd: &str) -> Interaction {
        Interaction {
            user_input: input.into(),
            predicted_cmd: cmd.into(),
            timestamp: 0,
        }
    }

    #[tokio::test]
    async fn runs_without_registered_memory() {
        let tmp = TempDir::new().unwrap();
        let mut memory = FolderMemory::new(tmp.path());

        let (provider, _captured) = MockProvider::new("ls -la");
        let mut runner = ReactLoop::new(provider);

        let mut workflow = NextCmd::new(&mut runner, &mut memory);
        let result = workflow.run("list files").await.unwrap();

        assert_eq!(result.cmd, "ls -la");
        assert!(memory.current().is_none(), "unregistered cwd should not load a conversation");
    }

    #[tokio::test]
    async fn appends_interaction_when_registered() {
        let tmp = TempDir::new().unwrap();
        let mut memory = FolderMemory::new(tmp.path());
        let cwd = env::current_dir().unwrap();
        memory.register(&cwd).unwrap();

        let (provider, _) = MockProvider::new("git status");
        let mut runner = ReactLoop::new(provider);

        {
            let mut workflow = NextCmd::new(&mut runner, &mut memory);
            workflow.run("git st").await.unwrap();
        }

        let conv = memory.current().expect("conversation loaded");
        assert_eq!(conv.interactions.len(), 1);
        assert_eq!(conv.interactions[0].user_input, "git st");
        assert_eq!(conv.interactions[0].predicted_cmd, "git status");
    }

    #[tokio::test]
    async fn prior_interactions_appear_in_prompt() {
        let tmp = TempDir::new().unwrap();
        let mut memory = FolderMemory::new(tmp.path());
        let cwd = env::current_dir().unwrap();
        memory.register(&cwd).unwrap();
        memory.append(seed_interaction("git st", "git status")).unwrap();

        let (provider, captured) = MockProvider::new("git diff");
        let mut runner = ReactLoop::new(provider);

        {
            let mut workflow = NextCmd::new(&mut runner, &mut memory);
            workflow.run("git df").await.unwrap();
        }

        let prompt = captured.lock().unwrap()[0].clone();
        assert!(prompt.contains("git status"), "prior cmd missing from prompt:\n{prompt}");
        assert!(prompt.contains("git df"),     "current input missing from prompt:\n{prompt}");
    }

    #[tokio::test]
    async fn append_persists_across_workflow_invocations() {
        let tmp = TempDir::new().unwrap();
        let memory_root = tmp.path().to_path_buf();
        let cwd = env::current_dir().unwrap();

        {
            let mut memory = FolderMemory::new(&memory_root);
            memory.register(&cwd).unwrap();
            let (provider, _) = MockProvider::new("ls");
            let mut runner = ReactLoop::new(provider);
            let mut workflow = NextCmd::new(&mut runner, &mut memory);
            workflow.run("show files").await.unwrap();
        }

        let mut memory = FolderMemory::new(&memory_root);
        assert!(memory.load(&cwd).unwrap());
        let conv = memory.current().unwrap();
        assert_eq!(conv.interactions.len(), 1);
        assert_eq!(conv.interactions[0].predicted_cmd, "ls");
    }
}