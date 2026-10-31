use super::job_store::{JobState, JobStore};
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use uuid::Uuid;

pub type JobTask = Box<dyn FnOnce() -> Result<String, String> + Send + 'static>;

pub struct JobEngine {
    tx: mpsc::Sender<(String, JobTask)>,
    store: JobStore,
}

impl JobEngine {
    pub fn new(capacity: usize) -> (Self, JobStore) {
        let max_concurrent = capacity.max(1);
        let store = JobStore::new();
        let (tx, mut rx) = mpsc::channel::<(String, JobTask)>(max_concurrent);
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let worker_store = store.clone();

        tokio::spawn(async move {
            while let Some((job_id, task)) = rx.recv().await {
                let s = worker_store.clone();
                let sem = semaphore.clone();

                tokio::spawn(Self::execute_job(job_id, task, s, sem));
            }
        });

        (
            Self {
                tx,
                store: store.clone(),
            },
            store,
        )
    }

    async fn execute_job(
        job_id: String,
        task: JobTask,
        store: JobStore,
        semaphore: Arc<Semaphore>,
    ) {
        let _permit = semaphore.acquire_owned().await.ok();
        let result = tokio::task::spawn_blocking(task).await;
        let state = match result {
            Ok(Ok(output)) => JobState::Completed(output),
            Ok(Err(err)) => JobState::Failed(err),
            Err(_) => JobState::Failed("Task panicked during execution".into()),
        };
        store.set(job_id, state).await;
    }

    pub async fn submit<F>(&self, task: F) -> String
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        let job_id = Uuid::new_v4().to_string();
        self.store.set(job_id.clone(), JobState::Running).await;

        if self.tx.send((job_id.clone(), Box::new(task))).await.is_err() {
            self.store
                .set(
                    job_id.clone(),
                    JobState::Failed("Job engine is shut down".into()),
                )
                .await;
        }
        job_id
    }

    pub fn store(&self) -> JobStore {
        self.store.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multithreading::job_pool::job_store::WaitOutcome;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn submit_registers_running_immediately() {
        let (engine, store) = JobEngine::new(4);
        let id = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(100));
                Ok("done".into())
            })
            .await;

        assert_eq!(store.get(&id).await, Some(JobState::Running));
    }

    #[tokio::test]
    async fn closure_returning_ok_ends_as_completed() {
        let (engine, store) = JobEngine::new(4);
        let id = engine.submit(|| Ok("hello world".into())).await;

        let outcome = store.wait_finished(&id, Duration::from_secs(1)).await;
        assert_eq!(outcome, WaitOutcome::Completed("hello world".into()));
    }

    #[tokio::test]
    async fn closure_returning_err_ends_as_failed() {
        let (engine, store) = JobEngine::new(4);
        let id = engine.submit(|| Err("something broke".into())).await;

        let outcome = store.wait_finished(&id, Duration::from_secs(1)).await;
        assert_eq!(outcome, WaitOutcome::Failed("something broke".into()));
    }

    #[tokio::test]
    async fn panicking_closure_ends_as_failed() {
        let (engine, store) = JobEngine::new(4);
        let id = engine
            .submit(|| -> Result<String, String> {
                panic!("unexpected panic");
            })
            .await;

        let outcome = store.wait_finished(&id, Duration::from_secs(1)).await;
        assert_eq!(
            outcome,
            WaitOutcome::Failed("Task panicked during execution".into())
        );
    }

    #[tokio::test]
    async fn several_jobs_run_concurrently() {
        let (engine, store) = JobEngine::new(4);
        let start = Instant::now();

        let id1 = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(200));
                Ok("one".into())
            })
            .await;
        let id2 = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(200));
                Ok("two".into())
            })
            .await;
        let id3 = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(200));
                Ok("three".into())
            })
            .await;

        assert_eq!(
            store.wait_finished(&id1, Duration::from_secs(1)).await,
            WaitOutcome::Completed("one".into())
        );
        assert_eq!(
            store.wait_finished(&id2, Duration::from_secs(1)).await,
            WaitOutcome::Completed("two".into())
        );
        assert_eq!(
            store.wait_finished(&id3, Duration::from_secs(1)).await,
            WaitOutcome::Completed("three".into())
        );

        assert!(
            start.elapsed() < Duration::from_millis(500),
            "Expected concurrent execution under 500ms, took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn fast_job_finishes_while_slow_job_is_still_running() {
        let (engine, store) = JobEngine::new(4);

        let slow_id = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(300));
                Ok("slow".into())
            })
            .await;
        let fast_id = engine
            .submit(|| {
                thread::sleep(Duration::from_millis(20));
                Ok("fast".into())
            })
            .await;

        assert_eq!(
            store.wait_finished(&fast_id, Duration::from_millis(150)).await,
            WaitOutcome::Completed("fast".into())
        );
        assert_eq!(store.get(&slow_id).await, Some(JobState::Running));

        assert_eq!(
            store.wait_finished(&slow_id, Duration::from_secs(1)).await,
            WaitOutcome::Completed("slow".into())
        );
    }

    #[tokio::test]
    async fn semaphore_caps_concurrent_jobs() {
        let (engine, store) = JobEngine::new(2);
        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        let mut ids = Vec::new();
        for i in 0..5 {
            let active_cl = active.clone();
            let max_cl = max_seen.clone();
            let id = engine
                .submit(move || {
                    let cur = active_cl.fetch_add(1, Ordering::SeqCst) + 1;
                    max_cl.fetch_max(cur, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(80));
                    active_cl.fetch_sub(1, Ordering::SeqCst);
                    Ok(format!("job-{i}"))
                })
                .await;
            ids.push(id);
        }

        for (i, id) in ids.into_iter().enumerate() {
            assert_eq!(
                store.wait_finished(&id, Duration::from_secs(2)).await,
                WaitOutcome::Completed(format!("job-{i}"))
            );
        }

        assert!(
            max_seen.load(Ordering::SeqCst) <= 2,
            "Expected at most 2 concurrent jobs, saw {}",
            max_seen.load(Ordering::SeqCst)
        );
    }

    fn simulate_fake_job(
        i: usize,
        active: &AtomicUsize,
        max_seen: &AtomicUsize,
    ) -> Result<String, String> {
        let cur = active.fetch_add(1, Ordering::SeqCst) + 1;
        max_seen.fetch_max(cur, Ordering::SeqCst);

        // Simulate mixed fake workloads (fast CPU work, slow I/O sleep, errors, panics)
        let delay_ms = match i % 5 {
            0 => 2,  // very fast
            1 => 8,  // medium
            2 => 15, // slower
            3 => 5,  // will fail
            _ => 3,  // every 10th panics, others succeed
        };
        thread::sleep(Duration::from_millis(delay_ms));

        active.fetch_sub(1, Ordering::SeqCst);

        if i % 10 == 9 {
            panic!("simulated panic in job {i}");
        } else if i % 5 == 3 {
            Err(format!("simulated error in job {i}"))
        } else {
            Ok(format!("result-{i}"))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn load_test_mixed_fake_jobs() {
        let capacity = 8;
        let total_jobs = 200;
        let (engine, store) = JobEngine::new(capacity);
        let engine = Arc::new(engine);

        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();

        // Spawn all 200 fake jobs concurrently across producer tasks
        let mut submit_handles = Vec::with_capacity(total_jobs);
        for i in 0..total_jobs {
            let eng = engine.clone();
            let active_cl = active.clone();
            let max_cl = max_seen.clone();

            submit_handles.push(tokio::spawn(async move {
                let job_id = eng
                    .submit(move || simulate_fake_job(i, &active_cl, &max_cl))
                    .await;
                (i, job_id)
            }));
        }

        let mut submitted = Vec::with_capacity(total_jobs);
        for h in submit_handles {
            submitted.push(h.await.expect("submit task panicked"));
        }

        // Concurrently poll and collect all 200 outcomes
        let mut wait_handles = Vec::with_capacity(total_jobs);
        for (i, job_id) in submitted {
            let st = store.clone();
            wait_handles.push(tokio::spawn(async move {
                let outcome = st.wait_finished(&job_id, Duration::from_secs(10)).await;
                (i, job_id, outcome)
            }));
        }

        let mut completed_count = 0;
        let mut failed_err_count = 0;
        let mut panicked_count = 0;

        for h in wait_handles {
            let (i, job_id, outcome) = h.await.expect("wait task panicked");
            if i % 10 == 9 {
                assert_eq!(
                    outcome,
                    WaitOutcome::Failed("Task panicked during execution".into())
                );
                panicked_count += 1;
            } else if i % 5 == 3 {
                assert_eq!(
                    outcome,
                    WaitOutcome::Failed(format!("simulated error in job {i}"))
                );
                failed_err_count += 1;
            } else {
                assert_eq!(outcome, WaitOutcome::Completed(format!("result-{i}")));
                completed_count += 1;
            }

            // Verify read-once eviction under load
            assert_eq!(store.get(&job_id).await, None);
        }

        let elapsed = start.elapsed();
        let peak = max_seen.load(Ordering::SeqCst);

        println!(
            "Load test finished {total_jobs} jobs in {elapsed:?} | peak concurrency: {peak}/{capacity} | ok: {completed_count}, err: {failed_err_count}, panic: {panicked_count}"
        );

        assert_eq!(completed_count + failed_err_count + panicked_count, total_jobs);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(
            peak <= capacity,
            "Expected peak concurrency <= {capacity}, saw {peak}"
        );
        assert!(
            peak > 1,
            "Expected multiple jobs to run in parallel under load, saw peak {peak}"
        );
    }
}

