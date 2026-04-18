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
        if current.len() + line.len() + 1 > max_len && !current.is_empty() {
            chunks.push(current);
            current = String::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_message_short_text_returns_single_chunk() {
        let text = "Hello, world!";
        let chunks = split_message(text, 4096);
        assert_eq!(chunks, vec!["Hello, world!"]);
    }

    #[test]
    fn split_message_exact_limit_returns_single_chunk() {
        let text = "a".repeat(4096);
        let chunks = split_message(&text, 4096);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 4096);
    }

    #[test]
    fn split_message_exceeds_limit_splits_at_line_boundary() {
        let lines: Vec<String> = (0..100)
            .map(|i| format!("Line {}: some content here", i))
            .collect();
        let text = lines.join("\n");
        let chunks = split_message(&text, 200);

        // Every chunk should be within the limit
        for chunk in &chunks {
            assert!(chunk.len() <= 200, "Chunk too long: {} chars", chunk.len());
        }

        // Rejoined content should equal original
        let rejoined = chunks.join("\n");
        assert_eq!(rejoined, text);
    }

    #[test]
    fn split_message_empty_text_returns_single_empty_chunk() {
        let chunks = split_message("", 4096);
        // Empty string has one empty line, which produces one empty chunk
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn split_message_preserves_all_lines() {
        let text = "Line 1\nLine 2\nLine 3";
        let chunks = split_message(text, 4096);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn split_message_single_long_line_becomes_own_chunk() {
        // A single line longer than max_len — it must still appear in output
        let long_line = "x".repeat(5000);
        let text = format!("short\n{}\nshort2", long_line);
        let chunks = split_message(&text, 4096);
        assert!(chunks.len() >= 2);
        // The long line should exist in exactly one chunk
        assert!(chunks.iter().any(|c| c.contains(&long_line)));
    }
}
