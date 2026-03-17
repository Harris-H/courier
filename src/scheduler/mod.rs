pub mod task;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::config::ScheduleConfig;
use crate::db::Database;
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
    pub digest_content: Option<String>,
}

pub struct Scheduler {
    inner: JobScheduler,
    history: Arc<RwLock<Vec<ExecutionRecord>>>,
    db: Arc<Database>,
    job_ids: RwLock<HashMap<String, uuid::Uuid>>,
    tasks: RwLock<HashMap<String, Arc<DigestTask>>>,
}

impl Scheduler {
    pub async fn new(history: Arc<RwLock<Vec<ExecutionRecord>>>, db: Arc<Database>) -> Result<Self> {
        let inner = JobScheduler::new().await?;
        Ok(Self {
            inner,
            history,
            db,
            job_ids: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
        })
    }

    /// Register a digest task with cron scheduling
    pub async fn add_task(&self, task: Arc<DigestTask>, config: &ScheduleConfig) -> Result<()> {
        let name = config.name.clone();
        let cron = config.cron.clone();
        let history = self.history.clone();
        let db = self.db.clone();
        let run_on_start = config.run_on_start.unwrap_or(false);

        // Optionally run immediately on startup
        if run_on_start {
            let task_clone = task.clone();
            let history_clone = history.clone();
            let db_clone = db.clone();
            tokio::spawn(async move {
                info!("Running '{}' immediately (run_on_start=true)", task_clone.name);
                record_execution(&task_clone, &history_clone, &db_clone).await;
            });
        }

        // Store task reference before moving into closure
        let task_for_store = task.clone();

        let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
            let task = task.clone();
            let history = history.clone();
            let db = db.clone();
            Box::pin(async move {
                record_execution(&task, &history, &db).await;
            })
        })?;

        let uuid = self.inner.add(job).await?;
        self.job_ids.write().await.insert(name.clone(), uuid);
        self.tasks.write().await.insert(name.clone(), task_for_store);
        info!("📅 Scheduled '{}' → cron '{}'", name, cron);
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.inner.start().await?;
        info!("⏰ Scheduler started");
        Ok(())
    }

    /// Dynamically update a task's cron schedule
    pub async fn update_schedule(&self, task_name: &str, new_cron: &str) -> Result<()> {
        // Remove old job
        if let Some(old_uuid) = self.job_ids.write().await.remove(task_name) {
            self.inner.remove(&old_uuid).await?;
        }

        // Get the existing task
        let task = self.tasks.read().await.get(task_name).cloned()
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_name))?;

        let name = task_name.to_string();
        let history = self.history.clone();
        let db = self.db.clone();

        let job = Job::new_async(new_cron, move |_uuid, _lock| {
            let task = task.clone();
            let history = history.clone();
            let db = db.clone();
            Box::pin(async move {
                record_execution(&task, &history, &db).await;
            })
        })?;

        let uuid = self.inner.add(job).await?;
        self.job_ids.write().await.insert(name.clone(), uuid);
        info!("🔄 Rescheduled '{}' → cron '{}'", name, new_cron);
        Ok(())
    }

    /// Rename a task's key in job_ids and tasks maps
    pub async fn rename_task(&self, old_name: &str, new_name: &str) -> Result<()> {
        let mut job_ids = self.job_ids.write().await;
        if let Some(uuid) = job_ids.remove(old_name) {
            job_ids.insert(new_name.to_string(), uuid);
        }
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.remove(old_name) {
            tasks.insert(new_name.to_string(), task);
        }
        info!("📝 Renamed task '{}' → '{}'", old_name, new_name);
        Ok(())
    }
}

/// Execute a task and record the result
async fn record_execution(
    task: &DigestTask,
    history: &RwLock<Vec<ExecutionRecord>>,
    db: &Database,
) {
    let start = std::time::Instant::now();
    let started_at = chrono::Local::now();

    let result = task.execute().await;
    let duration_ms = start.elapsed().as_millis() as u64;

    let record = match &result {
        Ok(stats) => {
            info!(
                "✅ Task '{}' completed in {}ms ({} articles, {} channels ok, {} failed)",
                task.name, duration_ms, stats.articles_fetched, stats.channels_sent, stats.channels_failed
            );
            ExecutionRecord {
                task_name: task.name.clone(),
                status: TaskStatus::Success,
                executed_at: started_at,
                duration_ms,
                articles_count: stats.articles_fetched,
                error_message: None,
                digest_content: Some(stats.digest_content.clone()),
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
                digest_content: None,
            }
        }
    };

    // Save to database
    if let Err(e) = db.insert_record(&record) {
        error!("Failed to save execution record to DB: {}", e);
    }

    let mut hist = history.write().await;
    hist.push(record);
    // Keep only last 100 records in memory
    let len = hist.len();
    if len > 100 {
        hist.drain(..len - 100);
    }
}
