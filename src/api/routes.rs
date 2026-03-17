use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

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
    let configs = state.schedule_configs.read().await;
    let tasks: Vec<TaskInfo> = configs
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
    let history = state.scheduler_history.clone();
    let db = state.db.clone();

    // Run in background and record execution history
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let started_at = chrono::Local::now();

        let result = task.execute().await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let record = match &result {
            Ok(stats) => {
                tracing::info!(
                    "✅ Manual task '{}' completed in {}ms ({} articles)",
                    name, duration_ms, stats.articles_fetched
                );
                crate::scheduler::ExecutionRecord {
                    task_name: name.clone(),
                    status: crate::scheduler::task::TaskStatus::Success,
                    executed_at: started_at,
                    duration_ms,
                    articles_count: stats.articles_fetched,
                    error_message: None,
                    digest_content: Some(stats.digest_content.clone()),
                }
            }
            Err(e) => {
                tracing::error!("❌ Manual task '{}' failed after {}ms: {}", name, duration_ms, e);
                crate::scheduler::ExecutionRecord {
                    task_name: name.clone(),
                    status: crate::scheduler::task::TaskStatus::Failed,
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
            tracing::error!("Failed to save execution record to DB: {}", e);
        }

        let mut hist = history.write().await;
        hist.push(record);
        if hist.len() > 100 {
            let len = hist.len();
            hist.drain(..len - 100);
        }
    });

    Ok(Json(RunTaskResponse {
        message: format!("Task '{}' triggered", task_name),
    }))
}

#[derive(Deserialize)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub cron: Option<String>,
    pub max_retries: Option<u32>,
}

pub async fn update_task_schedule(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    // Validate the task exists
    {
        let configs = state.schedule_configs.read().await;
        if !configs.iter().any(|s| s.name == name) {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Validate new name doesn't conflict with existing tasks
    if let Some(new_name) = &req.name {
        if !new_name.is_empty() && *new_name != name {
            let configs = state.schedule_configs.read().await;
            if configs.iter().any(|s| s.name == *new_name) {
                return Ok(Json(UpdateConfigResponse {
                    success: false,
                    message: format!("任务名 '{}' 已存在", new_name),
                }));
            }
        }
    }

    // Read and update config file
    let content = std::fs::read_to_string(&state.config_path).map_err(|e| {
        tracing::error!("Failed to read config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e| {
        tracing::error!("Failed to parse config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Find and update the schedule
    if let Some(schedules) = doc.get_mut("schedules").and_then(|s| s.as_array_mut()) {
        for schedule in schedules.iter_mut() {
            if let Some(table) = schedule.as_table_mut() {
                if table.get("name").and_then(|n| n.as_str()) == Some(&name) {
                    if let Some(new_name) = &req.name {
                        if !new_name.is_empty() {
                            table.insert("name".to_string(), toml::Value::String(new_name.clone()));
                        }
                    }
                    if let Some(cron) = &req.cron {
                        table.insert("cron".to_string(), toml::Value::String(cron.clone()));
                    }
                    if let Some(retries) = req.max_retries {
                        table.insert("max_retries".to_string(), toml::Value::Integer(retries as i64));
                    }
                }
            }
        }
    }

    let new_content = toml::to_string_pretty(&doc).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    std::fs::write(&state.config_path, &new_content).map_err(|e| {
        tracing::error!("Failed to write config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Determine the effective name after possible rename
    let effective_name = req.name.as_deref()
        .filter(|n| !n.is_empty())
        .unwrap_or(&name);

    // Update in-memory schedule configs
    {
        let mut configs = state.schedule_configs.write().await;
        if let Some(cfg) = configs.iter_mut().find(|s| s.name == name) {
            if let Some(new_name) = &req.name {
                if !new_name.is_empty() {
                    cfg.name = new_name.clone();
                }
            }
            if let Some(cron) = &req.cron {
                cfg.cron = cron.clone();
            }
            if let Some(retries) = req.max_retries {
                cfg.max_retries = Some(retries);
            }
        }
    }

    // Handle rename in scheduler
    if let Some(new_name) = &req.name {
        if !new_name.is_empty() && *new_name != name {
            if let Err(e) = state.scheduler.rename_task(&name, new_name).await {
                tracing::error!("Failed to rename task in scheduler: {}", e);
            }
        }
    }

    // Hot-reload: update scheduler cron immediately
    if let Some(cron) = &req.cron {
        if let Err(e) = state.scheduler.update_schedule(effective_name, cron).await {
            tracing::error!("Failed to hot-reload schedule: {}", e);
            return Ok(Json(UpdateConfigResponse {
                success: true,
                message: "配置已保存，但计划热更新失败，需重启服务".to_string(),
            }));
        }
    }

    tracing::info!("Schedule '{}' updated (effective name: '{}')", name, effective_name);

    Ok(Json(UpdateConfigResponse {
        success: true,
        message: "任务配置已保存并立即生效".to_string(),
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
    pub has_content: bool,
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
            has_content: r.digest_content.is_some(),
        })
        .collect();
    Json(entries)
}

#[derive(Serialize)]
pub struct HistoryContentResponse {
    pub content: Option<String>,
}

pub async fn get_history_content(
    State(state): State<Arc<AppState>>,
    Path(index): Path<usize>,
) -> Result<Json<HistoryContentResponse>, StatusCode> {
    let records = state.scheduler_history.read().await;
    // Index is the reverse order position (0 = most recent)
    let len = records.len();
    if index >= len {
        return Err(StatusCode::NOT_FOUND);
    }
    let record = &records[len - 1 - index];
    Ok(Json(HistoryContentResponse {
        content: record.digest_content.clone(),
    }))
}

#[derive(Serialize)]
pub struct ConfigOverview {
    pub log_level: String,
    pub llm_model: String,
    pub llm_api_base: String,
    pub sources: SourcesStatus,
    pub channels: ChannelsStatus,
    pub feishu_webhook_url: String,
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

    // Read LLM model from config file (may have been updated via API)
    let llm_model = std::fs::read_to_string(&state.config_path)
        .ok()
        .and_then(|content| content.parse::<toml::Value>().ok())
        .and_then(|doc| doc.get("llm")?.get("model")?.as_str().map(String::from))
        .unwrap_or_else(|| config.llm.model.clone());

    Json(ConfigOverview {
        log_level: config.general.log_level.clone(),
        llm_model,
        llm_api_base: mask_sensitive_url(&config.llm.api_base),
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
        feishu_webhook_url: mask_sensitive_url(&config.channels.feishu.webhook_url),
    })
}

/// Mask sensitive URLs, keeping only the domain visible
fn mask_sensitive_url(url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    match url::Url::parse(url) {
        Ok(parsed) => {
            if let Some(host) = parsed.host_str() {
                format!("{}://{}/*****", parsed.scheme(), host)
            } else {
                "***configured***".to_string()
            }
        }
        Err(_) => "***configured***".to_string(),
    }
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

#[derive(Deserialize)]
pub struct UpdateFeishuRequest {
    pub enabled: bool,
    pub webhook_url: String,
}

#[derive(Serialize)]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub message: String,
}

pub async fn update_feishu_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateFeishuRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    // Read the current config file as raw TOML
    let content = std::fs::read_to_string(&state.config_path).map_err(|e| {
        tracing::error!("Failed to read config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e| {
        tracing::error!("Failed to parse config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Update feishu section
    if let Some(channels) = doc.get_mut("channels").and_then(|c| c.as_table_mut()) {
        if let Some(feishu) = channels.get_mut("feishu").and_then(|f| f.as_table_mut()) {
            feishu.insert("enabled".to_string(), toml::Value::Boolean(req.enabled));
            feishu.insert("webhook_url".to_string(), toml::Value::String(req.webhook_url));
        } else {
            let mut feishu_table = toml::map::Map::new();
            feishu_table.insert("enabled".to_string(), toml::Value::Boolean(req.enabled));
            feishu_table.insert("webhook_url".to_string(), toml::Value::String(req.webhook_url));
            channels.insert("feishu".to_string(), toml::Value::Table(feishu_table));
        }
    }

    // Write back
    let new_content = toml::to_string_pretty(&doc).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    std::fs::write(&state.config_path, &new_content).map_err(|e| {
        tracing::error!("Failed to write config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Feishu config updated (enabled: {})", req.enabled);

    Ok(Json(UpdateConfigResponse {
        success: true,
        message: "飞书配置已保存，重启服务后生效".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct UpdateLlmRequest {
    pub model: String,
}

pub async fn update_llm_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateLlmRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    if req.model.is_empty() {
        return Ok(Json(UpdateConfigResponse {
            success: false,
            message: "模型名称不能为空".to_string(),
        }));
    }

    let content = std::fs::read_to_string(&state.config_path).map_err(|e| {
        tracing::error!("Failed to read config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e| {
        tracing::error!("Failed to parse config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(llm) = doc.get_mut("llm").and_then(|l| l.as_table_mut()) {
        llm.insert("model".to_string(), toml::Value::String(req.model.clone()));
    }

    let new_content = toml::to_string_pretty(&doc).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    std::fs::write(&state.config_path, &new_content).map_err(|e| {
        tracing::error!("Failed to write config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("LLM model updated to: {}", req.model);

    Ok(Json(UpdateConfigResponse {
        success: true,
        message: format!("模型已切换为 {}", req.model),
    }))
}
