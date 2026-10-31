use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobState {
    Running,
    Completed(String),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitOutcome {
    Completed(String),
    Failed(String),
    Pending,
    NotFound,
}

#[derive(Clone)]
pub struct JobStore {
    inner: Arc<RwLock<HashMap<String, JobState>>>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set(&self, id: String, state: JobState) {
        let mut map = self.inner.write().await;
        map.insert(id, state);
    }

    pub async fn get(&self, id: &str) -> Option<JobState> {
        let map = self.inner.read().await;
        map.get(id).cloned()
    }

    pub async fn take_if_finished(&self, id: &str) -> Option<JobState> {
        let mut map = self.inner.write().await;
        let finished = matches!(map.get(id), Some(s) if *s != JobState::Running);
        if finished { map.remove(id) } else { None }
    }

    pub async fn wait_finished(&self, id: &str, timeout: Duration) -> WaitOutcome {
        let start = Instant::now();
        loop {
            match self.get(id).await {
                None => return WaitOutcome::NotFound,
                Some(JobState::Running) => {
                    if start.elapsed() >= timeout {
                        return WaitOutcome::Pending;
                    }
                }
                Some(_) => {
                    return match self.take_if_finished(id).await {
                        Some(JobState::Completed(out)) => WaitOutcome::Completed(out),
                        Some(JobState::Failed(err)) => WaitOutcome::Failed(err),
                        _ => WaitOutcome::NotFound,
                    };
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_then_get_returns_stored_state() {
        let store = JobStore::new();
        store.set("job-1".into(), JobState::Running).await;
        assert_eq!(store.get("job-1").await, Some(JobState::Running));

        store
            .set("job-1".into(), JobState::Completed("done".into()))
            .await;
        assert_eq!(
            store.get("job-1").await,
            Some(JobState::Completed("done".into()))
        );
    }

    #[tokio::test]
    async fn take_if_finished_removes_completed_and_failed() {
        let store = JobStore::new();
        store
            .set("job-ok".into(), JobState::Completed("output".into()))
            .await;
        store
            .set("job-err".into(), JobState::Failed("boom".into()))
            .await;

        assert_eq!(
            store.take_if_finished("job-ok").await,
            Some(JobState::Completed("output".into()))
        );
        assert_eq!(store.take_if_finished("job-ok").await, None);

        assert_eq!(
            store.take_if_finished("job-err").await,
            Some(JobState::Failed("boom".into()))
        );
        assert_eq!(store.take_if_finished("job-err").await, None);
    }

    #[tokio::test]
    async fn take_if_finished_leaves_running_job_in_place() {
        let store = JobStore::new();
        store.set("job-run".into(), JobState::Running).await;

        assert_eq!(store.take_if_finished("job-run").await, None);
        assert_eq!(store.get("job-run").await, Some(JobState::Running));
    }

    #[tokio::test]
    async fn wait_finished_returns_pending_on_timeout() {
        let store = JobStore::new();
        store.set("job-slow".into(), JobState::Running).await;

        let outcome = store
            .wait_finished("job-slow", Duration::from_millis(60))
            .await;
        assert_eq!(outcome, WaitOutcome::Pending);
        assert_eq!(store.get("job-slow").await, Some(JobState::Running));
    }

    #[tokio::test]
    async fn wait_finished_returns_finished_and_evicts() {
        let store = JobStore::new();
        store
            .set("job-1".into(), JobState::Completed("res".into()))
            .await;
        store
            .set("job-2".into(), JobState::Failed("err".into()))
            .await;

        assert_eq!(
            store.wait_finished("job-1", Duration::from_millis(50)).await,
            WaitOutcome::Completed("res".into())
        );
        assert_eq!(
            store.wait_finished("job-1", Duration::from_millis(50)).await,
            WaitOutcome::NotFound
        );

        assert_eq!(
            store.wait_finished("job-2", Duration::from_millis(50)).await,
            WaitOutcome::Failed("err".into())
        );
        assert_eq!(
            store.wait_finished("job-2", Duration::from_millis(50)).await,
            WaitOutcome::NotFound
        );
    }

    #[tokio::test]
    async fn wait_finished_unknown_id_returns_not_found() {
        let store = JobStore::new();
        assert_eq!(
            store
                .wait_finished("missing", Duration::from_millis(50))
                .await,
            WaitOutcome::NotFound
        );
    }
}

