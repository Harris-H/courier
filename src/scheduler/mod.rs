pub mod task;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono_tz::Tz;
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
    pub completed_at: Option<chrono::DateTime<chrono::Local>>,
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
    timezone: Tz,
}

impl Scheduler {
    pub async fn new(history: Arc<RwLock<Vec<ExecutionRecord>>>, db: Arc<Database>, timezone: Tz) -> Result<Self> {
        let inner = JobScheduler::new().await?;
        Ok(Self {
            inner,
            history,
            db,
            job_ids: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            timezone,
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

        let job = Job::new_async_tz(cron.as_str(), self.timezone, move |_uuid, _lock| {
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
        info!("📅 Scheduled '{}' → cron '{}' (timezone: {})", name, cron, self.timezone);
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

        let job = Job::new_async_tz(new_cron, self.timezone, move |_uuid, _lock| {
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

    /// Remove a task from the scheduler (disable it)
    pub async fn remove_task(&self, task_name: &str) -> Result<()> {
        if let Some(uuid) = self.job_ids.write().await.remove(task_name) {
            self.inner.remove(&uuid).await?;
            info!("⏸ Removed task '{}' from scheduler", task_name);
        }
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

    // Insert a "Running" record immediately
    let running_record = ExecutionRecord {
        task_name: task.name.clone(),
        status: TaskStatus::Running,
        executed_at: started_at,
        completed_at: None,
        duration_ms: 0,
        articles_count: 0,
        error_message: None,
        digest_content: None,
    };
    let row_id = match db.insert_record(&running_record) {
        Ok(id) => Some(id),
        Err(e) => {
            error!("Failed to save running record to DB: {}", e);
            None
        }
    };
    {
        let mut hist = history.write().await;
        hist.push(running_record);
    }

    let result = task.execute().await;
    let duration_ms = start.elapsed().as_millis() as u64;
    let completed_at = chrono::Local::now();

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
                completed_at: Some(completed_at),
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
                completed_at: Some(completed_at),
                duration_ms,
                articles_count: 0,
                error_message: Some(e.to_string()),
                digest_content: None,
            }
        }
    };

    // Update the running record in database
    if let Some(id) = row_id {
        if let Err(e) = db.update_record(id, &record) {
            error!("Failed to update execution record in DB: {}", e);
        }
    }

    // Replace the running record in memory
    let mut hist = history.write().await;
    if let Some(pos) = hist.iter().position(|r| {
        r.task_name == record.task_name
            && r.executed_at == record.executed_at
            && r.status == TaskStatus::Running
    }) {
        hist[pos] = record;
    } else {
        hist.push(record);
    }
    // Keep only last 100 records in memory
    let len = hist.len();
    if len > 100 {
        hist.drain(..len - 100);
    }
}
