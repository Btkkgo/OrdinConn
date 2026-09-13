use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, Semaphore, broadcast, watch},
    task::JoinHandle,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerLifecycle {
    Idle,
    Running,
    Paused,
    Shutdown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRunResult {
    pub records: u64,
    pub observations: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceScheduleState {
    pub source_id: String,
    pub last_scheduled_at: Option<DateTime<Utc>>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub current_backoff_seconds: u64,
    pub last_result: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerEvent {
    pub source_id: String,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerDiagnostics {
    pub completed_runs: u64,
    pub failed_runs: u64,
    pub records: u64,
    pub observations: u64,
    pub reconnects: u64,
    pub stale_sources: u64,
}

#[async_trait]
pub trait ScheduledJob: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn cadence(&self) -> Duration;
    async fn run_once(&self) -> Result<JobRunResult, String>;
}

pub struct CollectorScheduler {
    lifecycle: watch::Sender<SchedulerLifecycle>,
    jobs: Mutex<Vec<Arc<dyn ScheduledJob>>>,
    handles: Mutex<Vec<JoinHandle<()>>>,
    states: Arc<Mutex<HashMap<String, SourceScheduleState>>>,
    diagnostics: Arc<Mutex<SchedulerDiagnostics>>,
    concurrency: Arc<Semaphore>,
    events: broadcast::Sender<SchedulerEvent>,
}

impl CollectorScheduler {
    pub fn new(global_concurrency: usize) -> Self {
        let (lifecycle, _) = watch::channel(SchedulerLifecycle::Idle);
        let (events, _) = broadcast::channel(128);
        Self {
            lifecycle,
            jobs: Mutex::new(vec![]),
            handles: Mutex::new(vec![]),
            states: Arc::new(Mutex::new(HashMap::new())),
            diagnostics: Arc::new(Mutex::new(SchedulerDiagnostics::default())),
            concurrency: Arc::new(Semaphore::new(global_concurrency.max(1))),
            events,
        }
    }

    pub async fn register(&self, job: Arc<dyn ScheduledJob>) -> Result<(), String> {
        if self.state().await != SchedulerLifecycle::Idle {
            return Err("scheduler already started".into());
        }
        if self
            .jobs
            .lock()
            .await
            .iter()
            .any(|item| item.id() == job.id())
        {
            return Err("duplicate scheduled source".into());
        }
        self.states
            .lock()
            .await
            .entry(job.id().into())
            .or_insert_with(|| SourceScheduleState {
                source_id: job.id().into(),
                ..Default::default()
            });
        self.jobs.lock().await.push(job);
        Ok(())
    }

    pub async fn restore_state(&self, state: SourceScheduleState) -> Result<(), String> {
        if self.state().await != SchedulerLifecycle::Idle {
            return Err("cannot restore after start".into());
        }
        self.states
            .lock()
            .await
            .insert(state.source_id.clone(), state);
        Ok(())
    }

    pub async fn start(&self) -> Result<(), String> {
        if self.state().await != SchedulerLifecycle::Idle {
            return Err("scheduler is not idle".into());
        }
        self.lifecycle.send_replace(SchedulerLifecycle::Running);
        let jobs = self.jobs.lock().await.clone();
        let mut handles = self.handles.lock().await;
        for job in jobs {
            let mut control = self.lifecycle.subscribe();
            let states = Arc::clone(&self.states);
            let diagnostics = Arc::clone(&self.diagnostics);
            let concurrency = Arc::clone(&self.concurrency);
            let events = self.events.clone();
            handles.push(tokio::spawn(async move {
                let restored_delay = states
                    .lock()
                    .await
                    .get(job.id())
                    .and_then(|state| state.next_run_at)
                    .and_then(|next| (next - Utc::now()).to_std().ok());
                if let Some(delay) = restored_delay {
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {
                            let _ = events.send(SchedulerEvent { source_id: job.id().into(), event_type: "collector.recovered".into(), occurred_at: Utc::now(), detail: None });
                        }
                        changed = control.changed() => {
                            if changed.is_err() || *control.borrow() == SchedulerLifecycle::Shutdown { return; }
                        }
                    }
                }
                loop {
                    let lifecycle = *control.borrow();
                    match lifecycle {
                        SchedulerLifecycle::Shutdown => break,
                        SchedulerLifecycle::Paused | SchedulerLifecycle::Idle => {
                            if control.changed().await.is_err() { break; }
                            continue;
                        }
                        SchedulerLifecycle::Running => {}
                    }
                    let source_id = job.id().to_string();
                    let scheduled_at = Utc::now();
                    {
                        let mut all = states.lock().await;
                        let state = all.entry(source_id.clone()).or_default();
                        state.source_id = source_id.clone();
                        state.last_scheduled_at = Some(scheduled_at);
                        state.last_started_at = Some(Utc::now());
                        state.last_result = Some("running".into());
                    }
                    let _ = events.send(SchedulerEvent { source_id: source_id.clone(), event_type: "collector.started".into(), occurred_at: Utc::now(), detail: None });
                    let permit = match concurrency.acquire().await { Ok(value) => value, Err(_) => break };
                    let result = job.run_once().await;
                    drop(permit);
                    let delay = {
                        let mut all = states.lock().await;
                        let state = all.entry(source_id.clone()).or_default();
                        state.last_completed_at = Some(Utc::now());
                        let mut diagnostic = diagnostics.lock().await;
                        match result {
                            Ok(run) => {
                                state.consecutive_failures = 0;
                                state.current_backoff_seconds = 0;
                                state.last_result = Some("completed".into());
                                diagnostic.completed_runs += 1;
                                diagnostic.records += run.records;
                                diagnostic.observations += run.observations;
                                let _ = events.send(SchedulerEvent { source_id: source_id.clone(), event_type: "collector.completed".into(), occurred_at: Utc::now(), detail: None });
                                job.cadence()
                            }
                            Err(error) => {
                                state.consecutive_failures = state.consecutive_failures.saturating_add(1);
                                let exponent = state.consecutive_failures.min(8);
                                let base = job.cadence().as_secs().max(1).saturating_mul(1_u64 << exponent).min(300);
                                state.current_backoff_seconds = base;
                                state.last_result = Some("failed".into());
                                diagnostic.failed_runs += 1;
                                let _ = events.send(SchedulerEvent { source_id: source_id.clone(), event_type: "collector.failed".into(), occurred_at: Utc::now(), detail: Some(error) });
                                Duration::from_secs(base) + deterministic_jitter(&source_id, state.consecutive_failures)
                            }
                        }
                    };
                    {
                        let mut all = states.lock().await;
                        if let Some(state) = all.get_mut(&source_id) { state.next_run_at = Some(Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default()); }
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        changed = control.changed() => { if changed.is_err() || *control.borrow() == SchedulerLifecycle::Shutdown { break; } }
                    }
                }
            }));
        }
        Ok(())
    }

    pub async fn pause(&self) {
        if self.state().await == SchedulerLifecycle::Running {
            self.lifecycle.send_replace(SchedulerLifecycle::Paused);
        }
    }
    pub async fn resume(&self) {
        if self.state().await == SchedulerLifecycle::Paused {
            self.lifecycle.send_replace(SchedulerLifecycle::Running);
        }
    }
    pub async fn shutdown(&self) {
        self.lifecycle.send_replace(SchedulerLifecycle::Shutdown);
        for handle in self.handles.lock().await.drain(..) {
            let _ = handle.await;
        }
        let _ = self.events.send(SchedulerEvent {
            source_id: "scheduler".into(),
            event_type: "scheduler.stopped".into(),
            occurred_at: Utc::now(),
            detail: None,
        });
    }
    pub async fn state(&self) -> SchedulerLifecycle {
        *self.lifecycle.borrow()
    }
    pub async fn snapshot(&self) -> Vec<SourceScheduleState> {
        let mut values: Vec<_> = self.states.lock().await.values().cloned().collect();
        values.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        values
    }
    pub async fn diagnostics(&self) -> SchedulerDiagnostics {
        self.diagnostics.lock().await.clone()
    }
    pub fn subscribe_events(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.events.subscribe()
    }
}

fn deterministic_jitter(source_id: &str, failures: u32) -> Duration {
    let hash = source_id.bytes().fold(failures as u64, |value, byte| {
        value.wrapping_mul(31).wrapping_add(byte as u64)
    });
    Duration::from_millis(hash % 251)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::Notify;

    struct FixtureJob {
        id: String,
        cadence: std::time::Duration,
        runs: AtomicUsize,
        active: AtomicUsize,
        peak_active: AtomicUsize,
        release: Option<Arc<Notify>>,
        fail_first: bool,
    }

    #[async_trait]
    impl ScheduledJob for FixtureJob {
        fn id(&self) -> &str {
            &self.id
        }
        fn cadence(&self) -> std::time::Duration {
            self.cadence
        }
        async fn run_once(&self) -> Result<JobRunResult, String> {
            let run = self.runs.fetch_add(1, Ordering::SeqCst) + 1;
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak_active.fetch_max(active, Ordering::SeqCst);
            if let Some(release) = &self.release {
                release.notified().await;
            }
            self.active.fetch_sub(1, Ordering::SeqCst);
            if self.fail_first && run == 1 {
                Err("fixture failure".into())
            } else {
                Ok(JobRunResult {
                    records: 1,
                    observations: 1,
                })
            }
        }
    }

    fn job(id: &str) -> Arc<FixtureJob> {
        Arc::new(FixtureJob {
            id: id.into(),
            cadence: std::time::Duration::from_secs(10),
            runs: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            peak_active: AtomicUsize::new(0),
            release: None,
            fail_first: false,
        })
    }

    #[tokio::test(start_paused = true)]
    async fn cadence_pause_resume_and_shutdown_are_owned() {
        let item = job("source-a");
        let scheduler = CollectorScheduler::new(2);
        scheduler.register(item.clone()).await.unwrap();
        scheduler.start().await.unwrap();
        tokio::task::yield_now().await;
        assert_eq!(item.runs.load(Ordering::SeqCst), 1);
        scheduler.pause().await;
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        tokio::task::yield_now().await;
        assert_eq!(item.runs.load(Ordering::SeqCst), 1);
        scheduler.resume().await;
        tokio::task::yield_now().await;
        tokio::time::advance(std::time::Duration::from_secs(10)).await;
        tokio::task::yield_now().await;
        assert!(item.runs.load(Ordering::SeqCst) >= 2);
        scheduler.shutdown().await;
        let after = item.runs.load(Ordering::SeqCst);
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        assert_eq!(item.runs.load(Ordering::SeqCst), after);
        assert_eq!(scheduler.state().await, SchedulerLifecycle::Shutdown);
    }

    #[tokio::test(start_paused = true)]
    async fn source_is_single_flight_and_global_concurrency_is_bounded() {
        let release = Arc::new(Notify::new());
        let item = Arc::new(FixtureJob {
            id: "source-a".into(),
            cadence: std::time::Duration::from_secs(1),
            runs: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            peak_active: AtomicUsize::new(0),
            release: Some(release.clone()),
            fail_first: false,
        });
        let scheduler = CollectorScheduler::new(1);
        scheduler.register(item.clone()).await.unwrap();
        scheduler.start().await.unwrap();
        tokio::task::yield_now().await;
        tokio::time::advance(std::time::Duration::from_secs(20)).await;
        tokio::task::yield_now().await;
        assert_eq!(item.runs.load(Ordering::SeqCst), 1);
        assert_eq!(item.peak_active.load(Ordering::SeqCst), 1);
        release.notify_waiters();
        tokio::task::yield_now().await;
        scheduler.shutdown().await;
    }

    #[tokio::test(start_paused = true)]
    async fn failure_uses_bounded_backoff_and_exposes_state() {
        let item = Arc::new(FixtureJob {
            id: "source-a".into(),
            cadence: std::time::Duration::from_secs(2),
            runs: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            peak_active: AtomicUsize::new(0),
            release: None,
            fail_first: true,
        });
        let scheduler = CollectorScheduler::new(1);
        scheduler.register(item.clone()).await.unwrap();
        scheduler.start().await.unwrap();
        tokio::task::yield_now().await;
        let state = scheduler
            .snapshot()
            .await
            .into_iter()
            .find(|state| state.source_id == "source-a")
            .unwrap();
        assert_eq!(state.consecutive_failures, 1);
        assert!(state.current_backoff_seconds >= 2 && state.current_backoff_seconds <= 300);
        assert_eq!(state.last_result.as_deref(), Some("failed"));
        scheduler.shutdown().await;
    }
}
