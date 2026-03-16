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
}

#[async_trait]
impl Channel for FeishuChannel {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        info!("Sending to Feishu webhook");

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
                        "content": content,
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
