use std::sync::Arc;

use tokio::sync::RwLock;

use crate::channels::Channel;
use crate::config::AppConfig;
use crate::db::Database;
use crate::llm::LlmClient;
use crate::scheduler::Scheduler;
use crate::scheduler::task::DigestTask;
use crate::scheduler::ExecutionRecord;
use crate::sources::Source;

/// Shared application state accessible from API routes and scheduler
pub struct AppState {
    pub config: AppConfig,
    pub config_path: String,
    pub sources: Vec<Arc<dyn Source>>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub llm: Arc<dyn LlmClient>,
    pub tasks: Vec<Arc<DigestTask>>,
    pub schedule_configs: RwLock<Vec<crate::config::ScheduleConfig>>,
    pub scheduler_history: Arc<RwLock<Vec<ExecutionRecord>>>,
    pub scheduler: Arc<Scheduler>,
    pub db: Arc<Database>,
    pub started_at: std::time::Instant,
}
