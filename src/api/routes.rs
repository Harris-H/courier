use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_secs: u64,
    pub tasks_count: usize,
    pub sources_count: usize,
    pub channels_count: usize,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let uptime = state.started_at.elapsed().as_secs();
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
        tasks_count: state.tasks.len(),
        sources_count: state.sources.len(),
        channels_count: state.channels.len(),
    })
}

#[derive(Serialize)]
pub struct TaskInfo {
    pub name: String,
    pub cron: String,
    pub sources: Vec<String>,
    pub channels: Vec<String>,
    pub run_on_start: bool,
    pub max_retries: u32,
}

pub async fn list_tasks(State(state): State<Arc<AppState>>) -> Json<Vec<TaskInfo>> {
    let tasks: Vec<TaskInfo> = state
        .schedule_configs
        .iter()
        .map(|c| TaskInfo {
            name: c.name.clone(),
            cron: c.cron.clone(),
            sources: c.sources.clone(),
            channels: c.channels.clone(),
            run_on_start: c.run_on_start.unwrap_or(false),
            max_retries: c.max_retries.unwrap_or(2),
        })
        .collect();
    Json(tasks)
}

#[derive(Serialize)]
pub struct RunTaskResponse {
    pub message: String,
}

pub async fn run_task(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<RunTaskResponse>, StatusCode> {
    let task = state
        .tasks
        .iter()
        .find(|t| t.name == name)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    let task_name = task.name.clone();

    // Run in background
    tokio::spawn(async move {
        if let Err(e) = task.execute().await {
            tracing::error!("Manual task '{}' failed: {}", name, e);
        }
    });

    Ok(Json(RunTaskResponse {
        message: format!("Task '{}' triggered", task_name),
    }))
}

#[derive(Serialize)]
pub struct HistoryEntry {
    pub task_name: String,
    pub status: String,
    pub executed_at: String,
    pub duration_ms: u64,
    pub articles_count: usize,
    pub error_message: Option<String>,
}

pub async fn get_history(State(state): State<Arc<AppState>>) -> Json<Vec<HistoryEntry>> {
    let records = state.scheduler_history.read().await;
    let entries: Vec<HistoryEntry> = records
        .iter()
        .rev()
        .map(|r| HistoryEntry {
            task_name: r.task_name.clone(),
            status: format!("{:?}", r.status),
            executed_at: r.executed_at.to_rfc3339(),
            duration_ms: r.duration_ms,
            articles_count: r.articles_count,
            error_message: r.error_message.clone(),
        })
        .collect();
    Json(entries)
}

#[derive(Serialize)]
pub struct ConfigOverview {
    pub log_level: String,
    pub llm_model: String,
    pub llm_api_base: String,
    pub sources: SourcesStatus,
    pub channels: ChannelsStatus,
}

#[derive(Serialize)]
pub struct SourcesStatus {
    pub hackernews: bool,
    pub reddit: bool,
    pub rss: bool,
}

#[derive(Serialize)]
pub struct ChannelsStatus {
    pub telegram: bool,
    pub feishu: bool,
    pub email: bool,
}

pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<ConfigOverview> {
    let config = &state.config;
    Json(ConfigOverview {
        log_level: config.general.log_level.clone(),
        llm_model: config.llm.model.clone(),
        llm_api_base: config.llm.api_base.clone(),
        sources: SourcesStatus {
            hackernews: config.sources.hackernews.enabled,
            reddit: config.sources.reddit.enabled,
            rss: config.sources.rss.enabled,
        },
        channels: ChannelsStatus {
            telegram: config.channels.telegram.enabled,
            feishu: config.channels.feishu.enabled,
            email: config.channels.email.enabled,
        },
    })
}

#[derive(Serialize)]
pub struct SourceInfo {
    pub name: String,
    pub enabled: bool,
}

pub async fn list_sources(State(state): State<Arc<AppState>>) -> Json<Vec<SourceInfo>> {
    let sources: Vec<SourceInfo> = state
        .sources
        .iter()
        .map(|s| SourceInfo {
            name: s.name().to_string(),
            enabled: true,
        })
        .collect();
    Json(sources)
}
