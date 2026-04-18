use async_trait::async_trait;
use reqwest::Client;
use tracing::info;

use super::Channel;
use crate::config::FeishuConfig;
use crate::error::{CourierError, Result};

pub struct FeishuChannel {
    client: Client,
    config: FeishuConfig,
}

impl FeishuChannel {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Convert markdown to Feishu-compatible format
    /// Feishu cards only support # and ## headings; convert ### and deeper to bold
    fn adapt_markdown(content: &str) -> String {
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("### ") {
                    format!("**{}**", trimmed.trim_start_matches('#').trim())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        info!("Sending to Feishu webhook");

        let adapted_content = Self::adapt_markdown(content);

        // Feishu webhook supports rich text cards
        let body = serde_json::json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": title,
                    },
                    "template": "blue",
                },
                "elements": [
                    {
                        "tag": "markdown",
                        "content": adapted_content,
                    }
                ]
            }
        });

        let resp = self
            .client
            .post(&self.config.webhook_url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(CourierError::ChannelSend {
                channel: "feishu".into(),
                message: error_text,
            });
        }

        // Check Feishu response for errors
        let resp_body: serde_json::Value = resp.json().await.unwrap_or_default();
        if resp_body.get("code").and_then(|c| c.as_i64()) != Some(0) {
            return Err(CourierError::ChannelSend {
                channel: "feishu".into(),
                message: resp_body.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapt_markdown_converts_h3_to_bold() {
        let input = "### 重要标题";
        assert_eq!(FeishuChannel::adapt_markdown(input), "**重要标题**");
    }

    #[test]
    fn adapt_markdown_preserves_h1_and_h2() {
        let input = "# Title\n## Subtitle\nText";
        assert_eq!(
            FeishuChannel::adapt_markdown(input),
            "# Title\n## Subtitle\nText"
        );
    }

    #[test]
    fn adapt_markdown_handles_mixed_headings() {
        let input = "## Section\n### Detail\nParagraph\n### Another";
        let result = FeishuChannel::adapt_markdown(input);
        assert_eq!(result, "## Section\n**Detail**\nParagraph\n**Another**");
    }

    #[test]
    fn adapt_markdown_empty_string() {
        assert_eq!(FeishuChannel::adapt_markdown(""), "");
    }

    #[test]
    fn adapt_markdown_h3_with_extra_spaces() {
        let input = "   ### Indented heading";
        let result = FeishuChannel::adapt_markdown(input);
        assert_eq!(result, "**Indented heading**");
    }
}
