mod api;
mod channels;
mod chat;
mod config;
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

    // Init logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.general.log_level)),
        )
        .init();

    info!("🚀 Courier starting...");

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

    // Build tasks
    let mut tasks: Vec<Arc<DigestTask>> = Vec::new();
    for schedule_config in &config.schedules {
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
    }

    // Setup scheduler
    let mut sched = Scheduler::new(history.clone()).await?;
    for (task, schedule_config) in tasks.iter().zip(config.schedules.iter()) {
        sched.add_task(task.clone(), schedule_config).await?;
    }
    sched.start().await?;
    info!("Scheduler started with {} task(s)", tasks.len());

    // Build shared app state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        sources: sources.clone(),
        channels: channels.clone(),
        llm: llm.clone(),
        tasks: tasks.clone(),
        schedule_configs: config.schedules.clone(),
        scheduler_history: history,
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

    sched.shutdown().await?;
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
        sources.push(Arc::new(RedditSource::new(config.sources.reddit.clone())));
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

