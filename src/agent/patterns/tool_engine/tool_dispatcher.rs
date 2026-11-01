use super::policy::{PermissionDecision, ToolExecutionPolicy};
use crate::agent::agents::Agent;
use crate::agent::prompts::pending_job_message;
use crate::agent::signals::{ToolInvocation, WAIT_JOB_NAME};
use crate::core::capability::ToolSafety;
use crate::core::responce::AgentToolCall;
use crate::core::session::{AgentSession, ToolResult};
use crate::multithreading::{JobEngine, JobState, JobStore};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

pub const TOOL_WAIT_THRESHOLD: Duration = Duration::from_secs(1);
pub const DEFAULT_JOB_CAPACITY: usize = 100;

#[derive(Clone)]
pub struct ToolDispatcher {
    policy: ToolExecutionPolicy,
    job_engine: Arc<JobEngine>,
    job_store: JobStore,
    threshold: Duration,
}

impl fmt::Debug for ToolDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolDispatcher")
            .field("policy", &self.policy)
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new(ToolExecutionPolicy::default())
    }
}

impl ToolDispatcher {
    pub fn new(policy: ToolExecutionPolicy) -> Self {
        Self::new_with_threshold(policy, TOOL_WAIT_THRESHOLD)
    }

    pub fn new_with_threshold(policy: ToolExecutionPolicy, threshold: Duration) -> Self {
        let (job_engine, job_store) = JobEngine::new(DEFAULT_JOB_CAPACITY);
        Self::with_job_pool(policy, job_engine, job_store, threshold)
    }

    pub fn with_job_pool(
        policy: ToolExecutionPolicy,
        job_engine: JobEngine,
        job_store: JobStore,
        threshold: Duration,
    ) -> Self {
        Self {
            policy,
            job_engine: Arc::new(job_engine),
            job_store,
            threshold,
        }
    }

    pub fn policy(&self) -> &ToolExecutionPolicy {
        &self.policy
    }

    pub fn job_store(&self) -> &JobStore {
        &self.job_store
    }

    pub fn threshold(&self) -> Duration {
        self.threshold
    }

    pub async fn authorize(
        &self,
        tools: Vec<ToolInvocation>,
        calls: &[AgentToolCall],
        _session: &mut AgentSession,
        agent: &mut Agent,
    ) -> (Vec<ToolInvocation>, Vec<(usize, ToolResult)>) {
        let mut permitted = Vec::new();
        let mut denied = Vec::new();

        for inv in tools {
            let call = &calls[inv.pos];
            match self.check_tool_permissions(&inv, agent, call).await {
                PermissionDecision::Approved => permitted.push(inv),
                PermissionDecision::Denied(reason) => {
                    let msg = format!("Tool '{}' failed: {}", call.name(), reason);
                    agent.hooks.on_tool_failed(call, &reason);
                    denied.push((inv.pos, ToolResult::new(call.name(), msg, call.id())));
                }
            }
        }

        (permitted, denied)
    }

    pub async fn execute(
        &self,
        permitted: Vec<ToolInvocation>,
        calls: &[AgentToolCall],
        agent: &mut Agent,
    ) -> Vec<(usize, ToolResult)> {
        let jobs = self.submit_jobs(permitted).await;
        self.wait_for_threshold(!jobs.is_empty()).await;
        self.collect_job_results(jobs, calls, agent).await
    }

    async fn submit_jobs(&self, permitted: Vec<ToolInvocation>) -> Vec<(usize, String)> {
        let mut jobs = Vec::with_capacity(permitted.len());
        for inv in permitted {
            let (cap, args) = (inv.cap, inv.args);
            let id = self
                .job_engine
                .submit(move || cap.execute(args).map_err(|e| e.to_string()))
                .await;
            jobs.push((inv.pos, id));
        }
        jobs
    }

    async fn wait_for_threshold(&self, has_jobs: bool) {
        if has_jobs {
            tokio::time::sleep(self.threshold).await;
        }
    }

    async fn collect_job_results(
        &self,
        jobs: Vec<(usize, String)>,
        calls: &[AgentToolCall],
        agent: &mut Agent,
    ) -> Vec<(usize, ToolResult)> {
        let mut out = Vec::with_capacity(jobs.len());
        for (pos, id) in jobs {
            let state = self.job_store.take_if_finished(&id).await;
            let result = self.resolve_job_outcome(&id, state, &calls[pos], agent);
            out.push((pos, result));
        }
        out
    }

    pub(crate) fn resolve_job_outcome(
        &self,
        job_id: &str,
        state: Option<JobState>,
        call: &AgentToolCall,
        agent: &mut Agent,
    ) -> ToolResult {
        let payload = match state {
            Some(JobState::Completed(output)) => output,
            Some(JobState::Failed(err_str)) => {
                agent.hooks.on_tool_failed(call, &err_str);
                let msg = format!("Tool '{}' failed: {}", call.name(), err_str);
                agent.hooks.on_tool_failed(call, &msg);
                msg
            }
            None | Some(JobState::Running) => {
                pending_job_message(self.threshold.as_secs(), job_id, WAIT_JOB_NAME)
            }
        };
        ToolResult::new(call.name(), payload, call.id())
    }

    async fn check_tool_permissions(
        &self,
        inv: &ToolInvocation,
        agent: &mut Agent,
        call: &AgentToolCall,
    ) -> PermissionDecision {
        let metadata = inv.cap.metadata();
        if self
            .policy
            .blacklisted_tools
            .iter()
            .any(|name| name == call.name())
        {
            return PermissionDecision::Denied(format!(
                "Tool '{}' is disabled by policy.",
                call.name()
            ));
        }

        if metadata.safety == ToolSafety::Safe && self.policy.allow_safe_tools_without_prompt {
            return PermissionDecision::Approved;
        }

        if matches!(metadata.safety, ToolSafety::Mutating) {
            if self.policy.deny_mutating_tools {
                return PermissionDecision::Denied(format!(
                    "Tool '{}' is blocked by the current run permission policy.",
                    call.name()
                ));
            }

            if self.policy.auto_execute_mutating_tools {
                return PermissionDecision::Approved;
            }

            let (tx, rx) = oneshot::channel();
            agent.request_tool_approval(call, tx);
            match rx.await {
                Ok(decision) => decision,
                Err(_) => PermissionDecision::Denied(format!(
                    "Tool '{}' requires approval but the approval request was cancelled.",
                    call.name()
                )),
            }
        } else {
            PermissionDecision::Approved
        }
    }
}

