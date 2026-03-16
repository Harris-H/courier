pub mod task;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::config::ScheduleConfig;
use task::{DigestTask, TaskStatus};

/// Execution record for tracking task history
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub task_name: String,
    pub status: TaskStatus,
    pub executed_at: chrono::DateTime<chrono::Local>,
    pub duration_ms: u64,
    pub articles_count: usize,
    pub error_message: Option<String>,
}

pub struct Scheduler {
    inner: JobScheduler,
    history: Arc<RwLock<Vec<ExecutionRecord>>>,
}

impl Scheduler {
    pub async fn new(history: Arc<RwLock<Vec<ExecutionRecord>>>) -> Result<Self> {
        let inner = JobScheduler::new().await?;
        Ok(Self { inner, history })
    }

    /// Register a digest task with cron scheduling
    pub async fn add_task(&self, task: Arc<DigestTask>, config: &ScheduleConfig) -> Result<()> {
        let name = config.name.clone();
        let cron = config.cron.clone();
        let history = self.history.clone();
        let run_on_start = config.run_on_start.unwrap_or(false);

        // Optionally run immediately on startup
        if run_on_start {
            let task_clone = task.clone();
            let history_clone = history.clone();
            tokio::spawn(async move {
                info!("Running '{}' immediately (run_on_start=true)", task_clone.name);
                record_execution(&task_clone, &history_clone).await;
            });
        }

        let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
            let task = task.clone();
            let history = history.clone();
            Box::pin(async move {
                record_execution(&task, &history).await;
            })
        })?;

        self.inner.add(job).await?;
        info!("📅 Scheduled '{}' → cron '{}'", name, cron);
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.inner.start().await?;
        info!("⏰ Scheduler started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown().await?;
        info!("Scheduler stopped");
        Ok(())
    }

    /// Get recent execution history
    pub async fn history(&self) -> Vec<ExecutionRecord> {
        self.history.read().await.clone()
    }
}

/// Execute a task and record the result
async fn record_execution(
    task: &DigestTask,
    history: &RwLock<Vec<ExecutionRecord>>,
) {
    let start = std::time::Instant::now();
    let started_at = chrono::Local::now();

    let result = task.execute().await;
    let duration_ms = start.elapsed().as_millis() as u64;

    let record = match &result {
        Ok(stats) => {
            info!(
                "✅ Task '{}' completed in {}ms ({} articles)",
                task.name, duration_ms, stats.articles_fetched
            );
            ExecutionRecord {
                task_name: task.name.clone(),
                status: TaskStatus::Success,
                executed_at: started_at,
                duration_ms,
                articles_count: stats.articles_fetched,
                error_message: None,
            }
        }
        Err(e) => {
            error!("❌ Task '{}' failed after {}ms: {}", task.name, duration_ms, e);
            ExecutionRecord {
                task_name: task.name.clone(),
                status: TaskStatus::Failed,
                executed_at: started_at,
                duration_ms,
                articles_count: 0,
                error_message: Some(e.to_string()),
            }
        }
    };

    let mut hist = history.write().await;
    hist.push(record);
    // Keep only last 100 records
    let len = hist.len();
    if len > 100 {
        hist.drain(..len - 100);
    }
}
