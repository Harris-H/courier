pub mod task;

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::config::ScheduleConfig;

pub struct Scheduler {
    inner: JobScheduler,
}

impl Scheduler {
    pub async fn new() -> Result<Self> {
        let inner = JobScheduler::new().await?;
        Ok(Self { inner })
    }

    pub async fn add_digest_job(
        &self,
        schedule: &ScheduleConfig,
        task_fn: impl Fn() -> tokio::task::JoinHandle<()> + Send + Sync + 'static,
    ) -> Result<()> {
        let name = schedule.name.clone();
        let job = Job::new_async(schedule.cron.as_str(), move |_uuid, _lock| {
            let name = name.clone();
            let handle = task_fn();
            Box::pin(async move {
                info!("Executing scheduled task: {}", name);
                if let Err(e) = handle.await {
                    tracing::error!("Task '{}' failed: {}", name, e);
                }
            })
        })?;

        self.inner.add(job).await?;
        info!("Scheduled task: '{}' with cron '{}'", schedule.name, schedule.cron);
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.inner.start().await?;
        info!("Scheduler started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown().await?;
        info!("Scheduler stopped");
        Ok(())
    }
}
