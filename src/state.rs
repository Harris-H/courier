use std::sync::Arc;

use tokio::sync::RwLock;

use courier::channels::Channel;
use courier::config::{AppConfig, EmailConfig};
use courier::db::Database;
use courier::llm::LlmClient;
use courier::scheduler::task::DigestTask;
use courier::scheduler::ExecutionRecord;
use courier::scheduler::Scheduler;
use courier::sources::Source;

/// Shared application state accessible from API routes and scheduler
pub struct AppState {
    pub config: AppConfig,
    pub config_path: String,
    pub sources: Vec<Arc<dyn Source>>,
    pub channels: Vec<Arc<dyn Channel>>,
    #[allow(dead_code)]
    pub llm: Arc<dyn LlmClient>,
    pub tasks: Vec<Arc<DigestTask>>,
    pub schedule_configs: RwLock<Vec<courier::config::ScheduleConfig>>,
    pub scheduler_history: Arc<RwLock<Vec<ExecutionRecord>>>,
    pub scheduler: Arc<Scheduler>,
    pub db: Arc<Database>,
    pub email_config: Arc<RwLock<EmailConfig>>,
    pub started_at: std::time::Instant,
}
