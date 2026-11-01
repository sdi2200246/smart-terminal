use super::{AgentLoopControl, AgentSignal, AgentSignalContext, AgentSignalOutcome};
use crate::agent::prompts::{WAIT_JOB_DESCRIPTION, WAIT_JOB_GUIDANCE};
use crate::core::capability::{ToolMetaData, ToolSafety};
use crate::multithreading::{JobStore, WaitOutcome};
use crate::tools::error::ToolError;
use crate::utils::FlatSchema;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

pub const WAIT_JOB_NAME: &str = "wait_job";

#[derive(JsonSchema, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
struct WaitJobArgs {
    /// List of pending background Job IDs to wait for until they finish.
    #[schemars(length(min = 1))]
    pub job_ids: Vec<String>,
}
impl FlatSchema for WaitJobArgs {}

pub struct WaitJobSignal;

impl WaitJobSignal {
    fn parse_args(args: Value) -> Result<WaitJobArgs, ToolError> {
        let parsed = serde_json::from_value::<WaitJobArgs>(args).map_err(|e| {
            ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("invalid wait_job arguments — {e}"),
            }
        })?;

        if parsed.job_ids.is_empty() {
            return Err(ToolError::ArgumentsParsing {
                source: anyhow::anyhow!("'job_ids' must not be empty"),
            });
        }

        Ok(parsed)
    }

    async fn wait_for_all(store: JobStore, job_ids: Vec<String>) -> Vec<(String, WaitOutcome)> {
        let mut out = Vec::with_capacity(job_ids.len());
        for id in job_ids {
            let outcome = store.wait_finished(&id, Duration::MAX).await;
            out.push((id, outcome));
        }
        out
    }

    fn format_outcomes(outcomes: &[(String, WaitOutcome)]) -> String {
        outcomes
            .iter()
            .map(|(id, outcome)| match outcome {
                WaitOutcome::Completed(s) => format!("Job {id}: completed\n{s}"),
                WaitOutcome::Failed(e) => format!("Job {id}: failed\n{e}"),
                WaitOutcome::Pending => format!(
                    "Job {id}: still running. Call {WAIT_JOB_NAME} again on its own to keep waiting."
                ),
                WaitOutcome::NotFound => {
                    format!("Job {id}: not found (unknown ID, or result already read).")
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl AgentSignal for WaitJobSignal {
    fn name(&self) -> &'static str {
        WAIT_JOB_NAME
    }

    fn must_run_alone(&self) -> bool {
        true
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: WAIT_JOB_DESCRIPTION.into(),
            parameters: WaitJobArgs::schema(),
            safety: ToolSafety::Safe,
        }
    }

    fn prompt_guidance(&self) -> Option<&'static str> {
        Some(WAIT_JOB_GUIDANCE)
    }

    fn handle(
        &self,
        args: Value,
        ctx: &mut AgentSignalContext<'_>,
    ) -> Result<AgentSignalOutcome, ToolError> {
        let parsed = Self::parse_args(args)?;
        let store = ctx.job_store.clone();

        let outcomes = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(Self::wait_for_all(store, parsed.job_ids))
        });

        let text = Self::format_outcomes(&outcomes);
        Ok(AgentSignalOutcome {
            result: text,
            control: AgentLoopControl::Continue,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::AgentBuilder;
    use crate::core::model::{Model, ModelName};
    use crate::core::session::AgentSession;
    use crate::multithreading::{JobEngine, JobState};
    use serde_json::json;
    use std::thread;

    fn run_wait_signal(
        store: &JobStore,
        args: Value,
    ) -> Result<AgentSignalOutcome, ToolError> {
        let mut session = AgentSession::new(5);
        let mut agent =
            AgentBuilder::base("test", Model::with_default_temp(ModelName::GptOss120B)).build();
        let mut ctx = AgentSignalContext {
            session: &mut session,
            agent: &mut agent,
            job_store: store,
        };
        WaitJobSignal.handle(args, &mut ctx)
    }

    #[test]
    fn metadata_and_exclusivity_are_configured() {
        let sig = WaitJobSignal;
        assert_eq!(sig.name(), "wait_job");
        assert!(sig.must_run_alone());
        let meta = sig.metadata();
        assert_eq!(meta.name, "wait_job");
        assert!(meta.description.contains("alone"));
        assert!(meta.parameters["properties"].get("timeout_seconds").is_none());
        assert!(sig.prompt_guidance().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_or_empty_job_ids_returns_error() {
        let store = JobStore::new();
        assert!(run_wait_signal(&store, json!({})).is_err());
        assert!(run_wait_signal(&store, json!({"job_ids": []})).is_err());
        assert!(run_wait_signal(&store, json!({"job_ids": [123]})).is_err());
        assert!(run_wait_signal(&store, json!({"job_ids": ["j1"], "timeout_seconds": 5})).is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn waits_until_running_jobs_complete_and_evicts_results() {
        let (engine, store) = JobEngine::new(4);
        let id1 = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(60));
                Ok("first output".into())
            })
            .await;
        let id2 = engine
            .submit(|| Err("second failed".into()))
            .await;

        let outcome = run_wait_signal(
            &store,
            json!({ "job_ids": [id1.clone(), id2.clone(), "unknown-id"] }),
        )
        .unwrap();

        assert_eq!(outcome.control, AgentLoopControl::Continue);
        let expected = format!(
            "Job {id1}: completed\nfirst output\n\nJob {id2}: failed\nsecond failed\n\nJob unknown-id: not found (unknown ID, or result already read)."
        );
        assert_eq!(outcome.result, expected);

        let second = run_wait_signal(&store, json!({ "job_ids": [id1.clone()] })).unwrap();
        assert_eq!(
            second.result,
            format!("Job {id1}: not found (unknown ID, or result already read).")
        );
        assert_eq!(store.get(&id1).await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn preserves_requested_id_order_regardless_of_completion_order() {
        let (engine, store) = JobEngine::new(4);
        let slow_id = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(80));
                Ok("slow-out".into())
            })
            .await;
        let fast_id = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(5));
                Ok("fast-out".into())
            })
            .await;

        let outcome = run_wait_signal(&store, json!({ "job_ids": [slow_id, fast_id] })).unwrap();
        let slow_pos = outcome.result.find("slow-out").unwrap();
        let fast_pos = outcome.result.find("fast-out").unwrap();
        assert!(slow_pos < fast_pos, "Expected slow-out before fast-out in:\n{}", outcome.result);
    }

    #[test]
    fn format_outcomes_renders_all_variants() {
        let store = JobStore::new();
        let _ = store;
        let text = WaitJobSignal::format_outcomes(&[
            ("j1".into(), WaitOutcome::Completed("ok".into())),
            ("j2".into(), WaitOutcome::Failed("err".into())),
            ("j3".into(), WaitOutcome::NotFound),
        ]);
        assert!(text.contains("Job j1: completed\nok"));
        assert!(text.contains("Job j2: failed\nerr"));
        assert!(text.contains("Job j3: not found"));
        let _ = JobState::Running;
    }
}

