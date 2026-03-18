mod api;
mod channels;
mod chat;
mod config;
mod db;
mod error;
mod llm;
mod scheduler;
mod sources;
mod state;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// Custom timer that outputs timestamps in local timezone (respects TZ env var)
struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"))
    }
}

use channels::{email::EmailChannel, feishu::FeishuChannel, telegram::TelegramChannel, Channel};
use chat::handler::ChatHandler;
use config::AppConfig;
use llm::{openai::OpenAIClient, LlmClient};
use scheduler::{task::DigestTask, ExecutionRecord, Scheduler};
use sources::{hackernews::HackerNewsSource, reddit::RedditSource, rss::RssSource, Source};
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());

    let config = AppConfig::load(&config_path)?;

    // Init logging - write to daily rotating log files
    let log_dir = std::path::Path::new(&config.general.data_dir).join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "courier.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Write logs to both stdout and file
    let combined_writer = std::io::stdout.and(non_blocking_file);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("courier=info,warn")),
        )
        .with_writer(combined_writer)
        .with_ansi(false)
        .with_timer(LocalTimer)
        .init();

    info!("🚀 Courier starting...");
    info!("📂 Logs are also written to: {}", log_dir.display());

    // Create data directory
    std::fs::create_dir_all(&config.general.data_dir)?;

    // Build sources
    let sources = build_sources(&config);
    info!("Loaded {} source(s)", sources.len());

    // Build LLM client
    let llm: Arc<dyn LlmClient> = Arc::new(OpenAIClient::new(&config.llm));
    info!("LLM client initialized (model: {})", config.llm.model);

    // Build channels
    let channels = build_channels(&config);
    info!("Loaded {} channel(s)", channels.len());

    // Shared execution history
    let history: Arc<RwLock<Vec<ExecutionRecord>>> = Arc::new(RwLock::new(Vec::new()));

    // Initialize database
    let db = Arc::new(db::Database::open(&config.general.data_dir)?);

    // Load history from database into memory
    if let Ok(records) = db.get_history(100) {
        let mut hist = history.write().await;
        // Records come in reverse order from DB, reverse to maintain chronological order
        let mut records = records;
        records.reverse();
        *hist = records;
        info!("Loaded {} history record(s) from database", hist.len());
    }

    // Build tasks
    let mut tasks: Vec<Arc<DigestTask>> = Vec::new();
    let mut enabled_configs: Vec<&config::ScheduleConfig> = Vec::new();

    for schedule_config in &config.schedules {
        if schedule_config.enabled == Some(false) {
            info!("⏸ Schedule '{}' is disabled, skipping", schedule_config.name);
            continue;
        }

        let task_sources: Vec<Arc<dyn Source>> = schedule_config
            .sources
            .iter()
            .filter_map(|name| sources.iter().find(|s| s.name() == name.as_str()).cloned())
            .collect();

        let task_channels: Vec<Arc<dyn Channel>> = schedule_config
            .channels
            .iter()
            .filter_map(|name| channels.iter().find(|c| c.name() == name.as_str()).cloned())
            .collect();

        if task_sources.is_empty() {
            tracing::warn!(
                "Schedule '{}' has no matching sources, skipping",
                schedule_config.name
            );
            continue;
        }

        tasks.push(Arc::new(DigestTask::new(
            schedule_config,
            task_sources,
            llm.clone(),
            task_channels,
        )));
        enabled_configs.push(schedule_config);
    }

    // Setup scheduler
    let timezone: chrono_tz::Tz = config.general.timezone.parse()
        .map_err(|e| anyhow::anyhow!("Invalid timezone '{}': {}", config.general.timezone, e))?;
    info!("🕐 Scheduler timezone: {}", timezone);
    let sched = Arc::new(Scheduler::new(history.clone(), db.clone(), timezone).await?);
    for (task, schedule_config) in tasks.iter().zip(enabled_configs.iter()) {
        sched.add_task(task.clone(), schedule_config).await?;
    }
    sched.start().await?;
    info!("Scheduler started with {} task(s)", tasks.len());

    // Build shared app state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        config_path: config_path.clone(),
        sources: sources.clone(),
        channels: channels.clone(),
        llm: llm.clone(),
        tasks: tasks.clone(),
        schedule_configs: RwLock::new(config.schedules.clone()),
        scheduler_history: history,
        scheduler: sched.clone(),
        db: db.clone(),
        started_at: std::time::Instant::now(),
    });

    // Start API server
    let api_port = config.general.api_port.unwrap_or(3001);
    let api_state = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = api::start_server(api_state, api_port).await {
            tracing::error!("API server error: {}", e);
        }
    });

    // Start chat mode if Telegram chat is enabled
    if config.channels.telegram.enabled && config.channels.telegram.chat_mode {
        info!("Starting Telegram chat mode...");
        let chat_handler = Arc::new(ChatHandler::new(llm.clone(), sources.clone()));
        start_telegram_chat(config.channels.telegram.bot_token.clone(), chat_handler).await;
    } else {
        info!("📬 Courier is running. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
    }

    info!("Shutting down...");
    Ok(())
}

fn build_sources(config: &AppConfig) -> Vec<Arc<dyn Source>> {
    let mut sources: Vec<Arc<dyn Source>> = Vec::new();

    if config.sources.hackernews.enabled {
        sources.push(Arc::new(HackerNewsSource::new(
            config.sources.hackernews.clone(),
        )));
    }

    if config.sources.reddit.enabled {
        match RedditSource::new(config.sources.reddit.clone()) {
            Ok(source) => sources.push(Arc::new(source)),
            Err(e) => tracing::error!("Failed to create Reddit source: {}", e),
        }
    }

    if config.sources.rss.enabled {
        sources.push(Arc::new(RssSource::new(config.sources.rss.clone())));
    }

    sources
}

fn build_channels(config: &AppConfig) -> Vec<Arc<dyn Channel>> {
    let mut channels: Vec<Arc<dyn Channel>> = Vec::new();

    if config.channels.telegram.enabled {
        channels.push(Arc::new(TelegramChannel::new(
            config.channels.telegram.clone(),
        )));
    }

    if config.channels.feishu.enabled {
        channels.push(Arc::new(FeishuChannel::new(
            config.channels.feishu.clone(),
        )));
    }

    if config.channels.email.enabled {
        channels.push(Arc::new(EmailChannel::new(config.channels.email.clone())));
    }

    channels
}

async fn start_telegram_chat(bot_token: String, handler: Arc<ChatHandler>) {
    use teloxide::prelude::*;

    let bot = Bot::new(bot_token);

    info!("Telegram bot is listening for messages...");

    let handler_clone = handler.clone();
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let handler = handler_clone.clone();
        async move {
            if let Some(text) = msg.text() {
                let response = handler.handle_message(text).await;
                bot.send_message(msg.chat.id, response).await?;
            }
            Ok(())
        }
    })
    .await;
}

