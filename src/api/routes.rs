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
    pub enabled: bool,
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
            enabled: c.enabled.unwrap_or(true),
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

        // Insert a "Running" record immediately
        let running_record = crate::scheduler::ExecutionRecord {
            task_name: name.clone(),
            status: crate::scheduler::task::TaskStatus::Running,
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
                tracing::error!("Failed to save running record to DB: {}", e);
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
                tracing::info!(
                    "✅ Manual task '{}' completed in {}ms ({} articles, {} chars)",
                    name, duration_ms, stats.articles_fetched, stats.digest_length
                );
                crate::scheduler::ExecutionRecord {
                    task_name: name.clone(),
                    status: crate::scheduler::task::TaskStatus::Success,
                    executed_at: started_at,
                    completed_at: Some(completed_at),
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
                tracing::error!("Failed to update execution record in DB: {}", e);
            }
        }

        // Replace the running record in memory
        let mut hist = history.write().await;
        if let Some(pos) = hist.iter().position(|r| {
            r.task_name == record.task_name
                && r.executed_at == record.executed_at
                && r.status == crate::scheduler::task::TaskStatus::Running
        }) {
            hist[pos] = record;
        } else {
            hist.push(record);
        }
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
    pub channels: Option<Vec<String>>,
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
                    if let Some(channels) = &req.channels {
                        table.insert("channels".to_string(), toml::Value::Array(
                            channels.iter().cloned().map(toml::Value::String).collect()
                        ));
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
            if let Some(channels) = &req.channels {
                cfg.channels = channels.clone();
            }
        }
    }

    // Hot-reload channels on the task
    if let Some(channel_names) = &req.channels {
        if let Some(task) = state.tasks.iter().find(|t| t.name == effective_name) {
            let new_channels: Vec<Arc<dyn crate::channels::Channel>> = channel_names
                .iter()
                .filter_map(|name| state.channels.iter().find(|c| c.name() == name.as_str()).cloned())
                .collect();
            let mut task_channels = task.channels.write().await;
            *task_channels = new_channels;
            tracing::info!("Hot-reloaded channels for task '{}'", effective_name);
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
    pub completed_at: Option<String>,
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
            completed_at: r.completed_at.map(|dt| dt.to_rfc3339()),
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

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub timestamps: Vec<String>,
}

pub async fn delete_history(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    if req.timestamps.is_empty() {
        return Ok(Json(UpdateConfigResponse {
            success: false,
            message: "没有选择要删除的记录".to_string(),
        }));
    }

    // Delete from database
    let deleted = state.db.delete_history_by_timestamps(&req.timestamps).map_err(|e| {
        tracing::error!("Failed to delete history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Remove from in-memory history
    {
        let mut hist = state.scheduler_history.write().await;
        hist.retain(|r| !req.timestamps.contains(&r.executed_at.to_rfc3339()));
    }

    tracing::info!("Deleted {} history record(s)", deleted);
    Ok(Json(UpdateConfigResponse {
        success: true,
        message: format!("已删除 {} 条记录", deleted),
    }))
}

pub async fn clear_history(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    let deleted = state.db.clear_all_history().map_err(|e| {
        tracing::error!("Failed to clear history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clear in-memory history
    {
        let mut hist = state.scheduler_history.write().await;
        hist.clear();
    }

    tracing::info!("Cleared all history ({} records)", deleted);
    Ok(Json(UpdateConfigResponse {
        success: true,
        message: format!("已清空 {} 条记录", deleted),
    }))
}

#[derive(Serialize)]
pub struct ConfigOverview {
    pub log_level: String,
    pub llm_model: String,
    pub llm_api_base: String,
    pub llm_max_tokens: u32,
    pub sources: SourcesStatus,
    pub channels: ChannelsStatus,
    pub feishu_webhook_url: String,
    pub email_config: EmailConfigOverview,
}

#[derive(Serialize)]
pub struct EmailConfigOverview {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub has_password: bool,
    pub from: String,
    pub to: Vec<String>,
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

    // Read email config from runtime state (hot-reloaded)
    let ec = state.email_config.read().await;
    let email_cfg = EmailConfigOverview {
        enabled: ec.enabled,
        smtp_host: ec.smtp_host.clone(),
        smtp_port: ec.smtp_port,
        smtp_username: ec.smtp_username.clone(),
        has_password: !ec.smtp_password.is_empty(),
        from: ec.from.clone(),
        to: ec.to.clone(),
    };
    drop(ec);

    Json(ConfigOverview {
        log_level: config.general.log_level.clone(),
        llm_model,
        llm_api_base: mask_sensitive_url(&config.llm.api_base),
        llm_max_tokens: config.llm.max_tokens,
        sources: SourcesStatus {
            hackernews: config.sources.hackernews.enabled,
            reddit: config.sources.reddit.enabled,
            rss: config.sources.rss.enabled,
        },
        channels: ChannelsStatus {
            telegram: config.channels.telegram.enabled,
            feishu: config.channels.feishu.enabled,
            email: email_cfg.enabled,
        },
        feishu_webhook_url: mask_sensitive_url(&config.channels.feishu.webhook_url),
        email_config: email_cfg,
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
pub struct UpdateEmailRequest {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from: String,
    pub to: Vec<String>,
}

pub async fn update_email_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateEmailRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    let content = std::fs::read_to_string(&state.config_path).map_err(|e| {
        tracing::error!("Failed to read config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e| {
        tracing::error!("Failed to parse config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Read existing password to preserve if not provided
    let existing_password = doc
        .get("channels")
        .and_then(|c| c.get("email"))
        .and_then(|e| e.get("smtp_password"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    let effective_password = if req.smtp_password.is_empty() {
        existing_password
    } else {
        req.smtp_password.clone()
    };

    if let Some(channels) = doc.get_mut("channels").and_then(|c| c.as_table_mut()) {
        let email_table = if let Some(email) = channels.get_mut("email").and_then(|e| e.as_table_mut()) {
            email
        } else {
            channels.insert("email".to_string(), toml::Value::Table(toml::map::Map::new()));
            channels.get_mut("email").unwrap().as_table_mut().unwrap()
        };

        email_table.insert("enabled".to_string(), toml::Value::Boolean(req.enabled));
        email_table.insert("smtp_host".to_string(), toml::Value::String(req.smtp_host.clone()));
        email_table.insert("smtp_port".to_string(), toml::Value::Integer(req.smtp_port as i64));
        email_table.insert("smtp_username".to_string(), toml::Value::String(req.smtp_username.clone()));
        email_table.insert("smtp_password".to_string(), toml::Value::String(effective_password.clone()));
        email_table.insert("from".to_string(), toml::Value::String(req.from.clone()));
        email_table.insert("to".to_string(), toml::Value::Array(
            req.to.iter().cloned().map(toml::Value::String).collect()
        ));
    }

    let new_content = toml::to_string_pretty(&doc).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    std::fs::write(&state.config_path, &new_content).map_err(|e| {
        tracing::error!("Failed to write config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Hot-reload: update runtime email config immediately
    {
        let mut email_cfg = state.email_config.write().await;
        email_cfg.enabled = req.enabled;
        email_cfg.smtp_host = req.smtp_host;
        email_cfg.smtp_port = req.smtp_port;
        email_cfg.smtp_username = req.smtp_username;
        email_cfg.smtp_password = effective_password;
        email_cfg.from = req.from;
        email_cfg.to = req.to;
    }

    tracing::info!("Email config updated and hot-reloaded (enabled: {})", req.enabled);

    Ok(Json(UpdateConfigResponse {
        success: true,
        message: "邮件配置已保存并立即生效".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct UpdateLlmRequest {
    pub model: String,
    pub max_tokens: Option<u32>,
}

#[derive(Deserialize)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}

pub async fn toggle_task(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<ToggleTaskRequest>,
) -> Result<Json<UpdateConfigResponse>, StatusCode> {
    // Validate the task exists
    {
        let configs = state.schedule_configs.read().await;
        if !configs.iter().any(|s| s.name == name) {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Update config file
    let content = std::fs::read_to_string(&state.config_path).map_err(|e| {
        tracing::error!("Failed to read config file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e| {
        tracing::error!("Failed to parse config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(schedules) = doc.get_mut("schedules").and_then(|s| s.as_array_mut()) {
        for schedule in schedules.iter_mut() {
            if let Some(table) = schedule.as_table_mut() {
                if table.get("name").and_then(|n| n.as_str()) == Some(&name) {
                    table.insert("enabled".to_string(), toml::Value::Boolean(req.enabled));
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

    // Update in-memory schedule configs
    {
        let mut configs = state.schedule_configs.write().await;
        if let Some(cfg) = configs.iter_mut().find(|s| s.name == name) {
            cfg.enabled = Some(req.enabled);
        }
    }

    // Enable/disable in scheduler
    if req.enabled {
        // Find the task and add to scheduler
        let schedule_config = {
            let configs = state.schedule_configs.read().await;
            configs.iter().find(|s| s.name == name).cloned()
        };
        if let Some(cfg) = schedule_config {
            let task = state.tasks.iter().find(|t| t.name == name).cloned();
            if let Some(task) = task {
                if let Err(e) = state.scheduler.add_task(task, &cfg).await {
                    tracing::warn!("Could not add task to scheduler: {}", e);
                }
            }
        }
        tracing::info!("✅ Task '{}' enabled", name);
    } else {
        if let Err(e) = state.scheduler.remove_task(&name).await {
            tracing::warn!("Could not remove task from scheduler: {}", e);
        }
        tracing::info!("⏸ Task '{}' disabled", name);
    }

    let message = if req.enabled {
        format!("任务 '{}' 已启用", name)
    } else {
        format!("任务 '{}' 已禁用", name)
    };

    Ok(Json(UpdateConfigResponse {
        success: true,
        message,
    }))
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
        if let Some(max_tokens) = req.max_tokens {
            llm.insert("max_tokens".to_string(), toml::Value::Integer(max_tokens as i64));
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

    let mut msg_parts = vec![format!("模型已切换为 {}", req.model)];
    if let Some(max_tokens) = req.max_tokens {
        tracing::info!("LLM config updated: model={}, max_tokens={}", req.model, max_tokens);
        msg_parts.push(format!("max_tokens={}", max_tokens));
    } else {
        tracing::info!("LLM model updated to: {}", req.model);
    }

    Ok(Json(UpdateConfigResponse {
        success: true,
        message: msg_parts.join(", "),
    }))
}
