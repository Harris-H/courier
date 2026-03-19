use std::sync::Arc;

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use pulldown_cmark::{html, Parser};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::Channel;
use crate::config::EmailConfig;
use crate::error::{CourierError, Result};

pub struct EmailChannel {
    config: Arc<RwLock<EmailConfig>>,
}

impl EmailChannel {
    pub fn new(config: Arc<RwLock<EmailConfig>>) -> Self {
        Self { config }
    }

    fn build_transport(config: &EmailConfig) -> std::result::Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
        let creds = Credentials::new(
            config.smtp_username.clone(),
            config.smtp_password.clone(),
        );

        Ok(AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
            .port(config.smtp_port)
            .credentials(creds)
            .build())
    }

    /// Build the from address: if no '@' in `from`, auto-construct "Name <smtp_username>"
    fn resolve_from(config: &EmailConfig) -> String {
        let from = config.from.trim();
        if from.is_empty() {
            config.smtp_username.clone()
        } else if from.contains('@') {
            from.to_string()
        } else {
            format!("{} <{}>", from, config.smtp_username)
        }
    }

    fn markdown_to_html(md: &str) -> String {
        let parser = Parser::new(md);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }
}

#[async_trait]
impl Channel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            debug!("Email channel is disabled, skipping");
            return Ok(());
        }

        let transport = Self::build_transport(&config).map_err(|e| CourierError::ChannelSend {
            channel: "email".into(),
            message: e.to_string(),
        })?;

        let body_html = Self::markdown_to_html(content);
        let html_content = format!(
            r#"<html><head><style>
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 800px; margin: 0 auto; padding: 20px; }}
h1 {{ color: #1a1a1a; border-bottom: 2px solid #eee; padding-bottom: 10px; }}
h2, h3 {{ color: #2c3e50; }}
a {{ color: #3498db; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
ul, ol {{ padding-left: 20px; }}
</style></head><body><h1>{}</h1>{}</body></html>"#,
            title, body_html
        );

        let from_addr = Self::resolve_from(&config);

        for recipient in &config.to {
            info!("Sending email to {}", recipient);

            let email = Message::builder()
                .from(from_addr.parse().map_err(|e: lettre::address::AddressError| {
                    CourierError::ChannelSend {
                        channel: "email".into(),
                        message: format!("Invalid from address '{}': {}", from_addr, e),
                    }
                })?)
                .to(recipient.parse().map_err(|e: lettre::address::AddressError| {
                    CourierError::ChannelSend {
                        channel: "email".into(),
                        message: format!("Invalid recipient '{}': {}", recipient, e),
                    }
                })?)
                .subject(title)
                .header(ContentType::TEXT_HTML)
                .body(html_content.clone())
                .map_err(|e| CourierError::ChannelSend {
                    channel: "email".into(),
                    message: e.to_string(),
                })?;

            transport.send(email).await.map_err(|e| CourierError::ChannelSend {
                channel: "email".into(),
                message: e.to_string(),
            })?;
        }

        Ok(())
    }
}
