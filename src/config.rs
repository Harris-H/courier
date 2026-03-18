use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{CourierError, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub sources: SourcesConfig,
    pub llm: LlmConfig,
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub schedules: Vec<ScheduleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    /// API server port (default: 3001)
    pub api_port: Option<u16>,
    /// Optional API key for dashboard authentication
    pub api_key: Option<String>,
    /// Timezone for cron schedules (e.g. "Asia/Shanghai"), defaults to "UTC"
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_data_dir() -> String {
    "./data".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct SourcesConfig {
    #[serde(default)]
    pub hackernews: HackerNewsConfig,
    #[serde(default)]
    pub reddit: RedditConfig,
    #[serde(default)]
    pub rss: RssConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HackerNewsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_top_n")]
    pub top_n: usize,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RedditConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub subreddits: Vec<String>,
    #[serde(default = "default_top_n")]
    pub top_n: usize,
}

fn default_top_n() -> usize {
    20
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RssConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub feeds: Vec<RssFeedEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RssFeedEntry {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub api_base: String,
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
}

fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_max_tokens() -> u32 {
    2000
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub email: EmailConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub chat_ids: Vec<i64>,
    #[serde(default)]
    pub chat_mode: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FeishuConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub webhook_url: String,
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_username: String,
    #[serde(default)]
    pub smtp_password: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: Vec<String>,
}

fn default_smtp_port() -> u16 {
    465
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleConfig {
    pub name: String,
    pub cron: String,
    pub sources: Vec<String>,
    pub channels: Vec<String>,
    pub prompt_template: Option<String>,
    /// Whether this task is enabled (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: Option<bool>,
    /// Run the task immediately on startup
    pub run_on_start: Option<bool>,
    /// Max retry count for LLM calls (default: 2)
    pub max_retries: Option<u32>,
}

fn default_enabled() -> Option<bool> {
    Some(true)
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            CourierError::Config(format!("Failed to read config file: {}", e))
        })?;

        let config: AppConfig = toml::from_str(&content).map_err(|e| {
            CourierError::Config(format!("Failed to parse config: {}", e))
        })?;

        Ok(config)
    }
}
