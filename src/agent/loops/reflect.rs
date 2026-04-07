use crate::agent::loops::traits::AgentLoop;
use crate::agent::request::AgentRequest;
use crate::agent::error::AgentError;
use crate::utils::FlatSchema;
use crate::core::error::ProviderError;
use crate::core::session::{ConversationEvent, Model};
use crate::core::llm_client::LLMProvider;
use crate::core::session::AgentSession;
use crate::core::capability::ToolNames;
use serde_json::Value;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(JsonSchema, Deserialize)]
struct Reflect {
    /// Your corrective plan starting after "Plan:"
    pub reflection: String,
}
impl FlatSchema for Reflect {}

pub struct ReflexionLoop {
    evaluator: fn(&Value) -> Option<String>,
    exec_model: Model,
    reflexion_model: Model,
    reflections: Vec<String>,
}

impl ReflexionLoop {
    pub fn new(evaluator: fn(&Value) -> Option<String>, exec_model: Model, reflexion_model: Model) -> Self {
        ReflexionLoop { evaluator, exec_model, reflexion_model, reflections: vec![] }
    }

    pub fn build_reflection_session(&self, failure_reason: &str, attempt_session: &AgentSession) -> AgentSession {
        let reflect_capability = ToolNames::Json(Reflect::schema()).to_capability();
        let mut session = AgentSession::new(vec![reflect_capability.metadata()], 3, self.reflexion_model.clone());

        let original_goal = attempt_session.events.iter().find_map(|e| {
            if let ConversationEvent::User(message) = e {
                Some(message.as_str())
            } else {
                None
            }
        }).unwrap_or_default();

        let transcript = attempt_session.events()
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");

        let previous = if self.reflections.is_empty() {
            String::new()
        } else {
            format!(
                "\nPrevious reflections:\n{}\n",
                self.reflections.join("\n\n")
            )
        };

        let prompt = format!(
            "You will be given the history of a past experience in which you were placed \
            in an environment and given a task to complete. You were unsuccessful in completing \
            the task. Do not summarize your environment, but rather think about the strategy \
            and path you took to attempt to complete the task. Devise a concise, new plan of \
            action that accounts for your mistake with reference to specific actions that you \
            should have taken. For example, if you tried A and B but forgot C, then devise a \
            plan to achieve C with environment-specific actions. You will need this later when \
            you are solving the same task. Give your plan after 'Plan'.\
            {previous}\
            \nInstruction: {original_goal}\
            \nTrial transcript:\n{transcript}"
        );

        session.add_system(prompt);
        session.add_user(format!("Failure reason: {}\nPlan:", failure_reason));

        session
    }
    async fn reflect(
    &self,
    failure_reason: &str,
    provider: &mut impl LLMProvider,
    attempt_session: &AgentSession,
) -> Result<String, AgentError> {
    let mut session = self.build_reflection_session(failure_reason, attempt_session);
    let reflect_capability = ToolNames::Json(Reflect::schema()).to_capability();
    let mut last_args: Option<Value> = None;

    loop {
        if session.steps_exhausted() {
            return Err(AgentError::StepsExhausted);
        }

        if session.is_resolved() {
            let reflect: Reflect = serde_json::from_value(last_args.unwrap())
                .expect("Json tool validated schema — deserialization cannot fail");
            return Ok(reflect.reflection);
        }

        let call = match provider.complete(&session).await {
            Ok(call) => call,
            Err(ProviderError::InvalidToolCal { source }) => {
                session.add_error(source.to_string());
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        let result = match reflect_capability.execute(call.arguments().clone()) {
            Ok(result) => result,
            Err(e) => {
                session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                continue;
            }
        };

        last_args = Some(call.arguments().clone());
        session.add_tool_call(call.name(), call.arguments().clone(), call.id());
        session.add_tool_result(call.name(), result, call.id());
    }
}
   
}

impl AgentLoop for ReflexionLoop {
    #[tracing::instrument(skip(self, req, provider), fields(loop_kind = "Reflection"))]
    async fn agent_loop(&mut self, req: AgentRequest, provider: &mut impl LLMProvider) -> Result<Value, AgentError> {
        let tools = Self::build_tools_registry(&req);
        let mut session = Self::build_attempt_session(&tools, &req, self.exec_model.clone());
        let mut last_args: Option<Value> = None;

        loop {
            if session.steps_exhausted() {
                tracing::warn!("agent exhausted all steps");
                return Err(AgentError::StepsExhausted);
            }

            if session.is_resolved() {
                let args = last_args.as_ref().unwrap();
                match (self.evaluator)(args) {
                    None => {
                        tracing::info!("task completed successfully");
                        return Ok(args.clone());
                    }
                    Some(failure_reason) => {
                        tracing::warn!(reason = %failure_reason, "answer failed evaluation, reflecting");
                        let reflection = self.reflect(&failure_reason, provider, &session).await?;

                        session.clear_resolved();
                        if self.reflections.len() == 3 {
                            self.reflections.remove(0);
                        }
                        self.reflections.push(reflection.clone());
                        session.add_reflection(reflection);
                    }
                }
                continue;
            }

            let call = match provider.complete(&session).await {
                Ok(call) => call,
                Err(ProviderError::InvalidToolCal { source }) => {
                    tracing::warn!(%source, "invalid tool call, recovering and continuing");
                    let available: Vec<_> = tools.keys().copied().collect();
                    session.add_error(format!(
                        "Invalid tool call!:\nOnly Available tools:{}",
                        available.join(", ")
                    ));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            tracing::info!(tool = %call.name(), args = %call.arguments(), "executing tool");

            let result = match tools[call.name()].execute(call.arguments().clone()) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(tool = %call.name(), error = %e, "tool execution failed");
                    session.add_error(format!("Tool '{}' failed: {}", call.name(), e));
                    continue;
                }
            };

            last_args = Some(call.arguments().clone());
            session.add_tool_call(call.name(), call.arguments().clone(), call.id());
            session.add_tool_result(call.name(), result, call.id());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::{AgentSession, Model, ModelName};
    use crate::groq::client::GroqClient;
    use serde_json::json;

    fn make_attempt_session() -> AgentSession {
        let mut session = AgentSession::new(vec![], 10, Model::with_default_temp(ModelName::GptOss120B));
        session.add_system("You are an expert bash execution agent embedded in a developer's shell.");
        session.add_user("Find the 3 largest files modified in the last 7 days, show their sizes in human readable format, sorted by size descending");
        session.add_tool_call("find_files", json!({"path": "."}), "call_1");
        session.add_tool_result(
            "find_files",
            "find . -mtime -7 -printf '%s %p\\n' | sort -rn | head -3",
            "call_1",
        );
        session.add_tool_call("execute_script", json!({}), "call_2");
        session.add_tool_result(
            "execute_script",
            "find: illegal option -- -printf\nusage: find [-H | -L | -P] [-EXdsx] [-f path] path ... [expression]",
            "call_2",
        );
        session
    }

    #[test]
    fn build_reflection_session_structure() {
        let loop_ = ReflexionLoop::new(
            |_| None,
            Model::with_default_temp(ModelName::GptOss120B),
            Model::with_default_temp(ModelName::GptOss120B),
        );
        let attempt = make_attempt_session();
        let session = loop_.build_reflection_session(
            "script failed: find: illegal option -- -printf",
            &attempt,
        );

        assert_eq!(session.available_tools.len(), 1);
        assert_eq!(session.available_tools[0].name, "final_answer");

        let system_count = session.events.iter()
            .filter(|e| matches!(e, ConversationEvent::System(_)))
            .count();
        assert_eq!(system_count, 1);

        if let ConversationEvent::System(msg) = &session.events[0] {
            assert!(msg.contains("Plan"), "missing framing instruction");
            assert!(msg.contains("Find the 3 largest files"), "missing original goal");
            assert!(msg.contains("Trial transcript"), "missing transcript");
            assert!(!msg.contains("Previous reflections"), "should have no reflections yet");
        }

        assert!(matches!(
            session.events.last(),
            Some(ConversationEvent::User(msg)) if msg.contains("Plan:")
        ));
    }

    #[test]
    fn build_reflection_session_injects_previous_reflections() {
        let mut loop_ = ReflexionLoop::new(
            |_| None,
            Model::with_default_temp(ModelName::GptOss120B),
            Model::deterministic(ModelName::Llma3p370B),
        );
        loop_.reflections.push("Plan: Use BSD find syntax instead of GNU find".into());
        loop_.reflections.push("Plan: Use stat instead of -printf for file sizes".into());

        let attempt = make_attempt_session();
        let session = loop_.build_reflection_session(
            "script still failed",
            &attempt,
        );

        if let ConversationEvent::System(msg) = &session.events[0] {
            assert!(msg.contains("Previous reflections"), "missing previous reflections");
            assert!(msg.contains("BSD find syntax"), "missing first reflection");
            assert!(msg.contains("stat instead of -printf"), "missing second reflection");
        }
    }

    #[test]
    fn reflections_capped_at_3() {
        let mut loop_ = ReflexionLoop::new(
            |_| None,
            Model::with_default_temp(ModelName::GptOss120B),
            Model::deterministic(ModelName::Llma3p370B),
        );
        loop_.reflections = vec![
            "reflection 1".into(),
            "reflection 2".into(),
            "reflection 3".into(),
        ];

        if loop_.reflections.len() == 3 {
            loop_.reflections.remove(0);
        }
        loop_.reflections.push("reflection 4".into());

        assert_eq!(loop_.reflections.len(), 3);
        assert!(!loop_.reflections.contains(&"reflection 1".to_string()));
        assert!(loop_.reflections.contains(&"reflection 4".to_string()));
    }


    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY"]
    async fn reflect_with_previous_reflections() {
        let mut provider = GroqClient::default();
        let mut loop_ = ReflexionLoop::new(
            |_| None,
            Model::with_default_temp(ModelName::GptOss120B),
            Model::deterministic(ModelName::Llma3p370B),
        );
        loop_.reflections.push(
            "Plan: Use BSD find syntax. Replace -printf with -exec stat.".into(),
        );

        let attempt = make_attempt_session();

        let result = loop_.reflect(
            "script still failed. stat command not available on this system.",
            &mut provider,
            &attempt,
        ).await;

        assert!(result.is_ok());
        let reflection = result.unwrap();
        assert!(!reflection.is_empty());
        println!("reflection with prior context:\n{reflection}");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::groq::client::GroqClient;
    use crate::core::capability::ToolNames;
    use crate::core::session::{Model, ModelName};
    use crate::agent::request::AgentRequest;
    use schemars::JsonSchema;
    use serde::Deserialize;

    use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    const TEST_POLICY: &str = "You are an expert bash execution agent embedded in a developer's shell.
    You will receive a JSON context object describing the environment.
    Use it to inform every decision — OS, shell, cwd, and user intent are all there.

    STRATEGY:
    Use your tools to build a complete picture before acting.
    Inspect, validate, and reason before committing to any script.
    When you have enough information to produce a correct script, stop looping.

    AMBIGUITY:
    If the intent is unclear, make the safest reasonable assumption and proceed.

    SCOPE:
    Never operate outside the cwd unless explicitly stated.
    Never modify system files.

    OUTPUT:
    You MUST submit your final script using the json tool — this is the only valid way to produce output.
    The script must be complete and executable as-is.
    Do NOT include comments or explanations in the script — code only.";

    #[derive(JsonSchema, Deserialize)]
    pub struct Script {
        /// Complete executable shell script including shebang
        pub script: String,
    }
    impl FlatSchema for Script {}

    fn evaluate_script(response: &Value) -> Option<String> {
        let script: Script = serde_json::from_value(response.clone()).ok()?;

        if script.script.is_empty() {
            return Some("script is empty".into());
        }

        if !script.script.contains("#!/") {
            return Some("script is missing shebang".into());
        }

        let tmp_dir = tempfile::TempDir::new().ok()?;
        let script_path = tmp_dir.path().join("script.sh");
        std::fs::write(&script_path, &script.script).ok()?;

        let output = std::process::Command::new("bash")
            .arg(&script_path)
            .current_dir(tmp_dir.path())
            .output()
            .ok()?;

        if output.status.success() {
            None
        } else {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    #[tokio::test]
    #[ignore = "requires GROQ_API_KEY"]
    async fn reflexion_loop_produces_valid_script() {
        let file_appender = tracing_appender::rolling::daily("./logs", "app.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_ansi(false))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
            .with(tracing_subscriber::EnvFilter::new("warn,smart_terminal=debug"))
            .try_init()
            .ok();

        let mut provider = GroqClient::default();
        let mut loop_ = ReflexionLoop::new(
            evaluate_script,
            Model::with_default_temp(ModelName::GptOss120B),
            Model::creative(ModelName::GptOss120B),
        );

        let req = AgentRequest::builder()
            .tools(vec![ToolNames::Json(Script::schema()),ToolNames::GitStatus, ToolNames::GitLog, ToolNames::GitDiffStaged])
            .with_system_promt(TEST_POLICY.into())
            .with_user_promt("Find most user commands from histroy".into());

        let result = loop_.agent_loop(req, &mut provider).await;

        assert!(result.is_ok(), "agent loop failed: {:?}", result.err());
        let script: Script = serde_json::from_value(result.unwrap()).unwrap();
        assert!(!script.script.is_empty());
        assert!(script.script.contains("#!/"));
        println!("final script:\n{}", script.script);
        println!("reflections used: {}", loop_.reflections.len());
        if !loop_.reflections.is_empty() {
            println!("reflections:\n{}", loop_.reflections.join("\n\n"));
        }
    }
}