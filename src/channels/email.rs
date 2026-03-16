use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use tracing::info;

use super::Channel;
use crate::config::EmailConfig;
use crate::error::{CourierError, Result};

pub struct EmailChannel {
    config: EmailConfig,
}

impl EmailChannel {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    fn build_transport(&self) -> std::result::Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
        let creds = Credentials::new(
            self.config.smtp_username.clone(),
            self.config.smtp_password.clone(),
        );

        Ok(AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)?
            .port(self.config.smtp_port)
            .credentials(creds)
            .build())
    }
}

#[async_trait]
impl Channel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        let transport = self.build_transport().map_err(|e| CourierError::ChannelSend {
            channel: "email".into(),
            message: e.to_string(),
        })?;

        // Convert markdown content to simple HTML
        let html_content = format!(
            "<html><body><h1>{}</h1><pre style=\"white-space: pre-wrap; font-family: sans-serif;\">{}</pre></body></html>",
            title, content
        );

        for recipient in &self.config.to {
            info!("Sending email to {}", recipient);

            let email = Message::builder()
                .from(self.config.from.parse().map_err(|e: lettre::address::AddressError| {
                    CourierError::ChannelSend {
                        channel: "email".into(),
                        message: e.to_string(),
                    }
                })?)
                .to(recipient.parse().map_err(|e: lettre::address::AddressError| {
                    CourierError::ChannelSend {
                        channel: "email".into(),
                        message: e.to_string(),
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
