use async_trait::async_trait;
use reqwest::Client;
use tracing::info;

use super::Channel;
use crate::config::TelegramConfig;
use crate::error::{CourierError, Result};

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
/// Max message length for Telegram (UTF-8)
const MAX_MESSAGE_LEN: usize = 4096;

pub struct TelegramChannel {
    client: Client,
    config: TelegramConfig,
}

impl TelegramChannel {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    async fn send_message(&self, chat_id: i64, text: &str) -> Result<()> {
        let url = format!(
            "{}/bot{}/sendMessage",
            TELEGRAM_API_BASE, self.config.bot_token
        );

        // Split long messages
        let chunks = split_message(text, MAX_MESSAGE_LEN);

        for chunk in chunks {
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": chunk,
                "parse_mode": "Markdown",
                "disable_web_page_preview": true,
            });

            let resp = self.client.post(&url).json(&body).send().await?;

            if !resp.status().is_success() {
                let error_text = resp.text().await.unwrap_or_default();
                return Err(CourierError::ChannelSend {
                    channel: "telegram".into(),
                    message: error_text,
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        let full_message = format!("*{}*\n\n{}", title, content);

        for chat_id in &self.config.chat_ids {
            info!("Sending to Telegram chat {}", chat_id);
            self.send_message(*chat_id, &full_message).await?;
        }

        Ok(())
    }
}

fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > max_len {
            if !current.is_empty() {
                chunks.push(current);
                current = String::new();
            }
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}
