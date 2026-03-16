use std::sync::Arc;

use tracing::{error, info};

use crate::channels::Channel;
use crate::config::ScheduleConfig;
use crate::llm::LlmClient;
use crate::sources::Source;

/// A digest task: fetch from sources → summarize via LLM → push to channels
pub struct DigestTask {
    pub name: String,
    pub sources: Vec<Arc<dyn Source>>,
    pub llm: Arc<dyn LlmClient>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub prompt_template: Option<String>,
}

impl DigestTask {
    pub fn new(
        config: &ScheduleConfig,
        sources: Vec<Arc<dyn Source>>,
        llm: Arc<dyn LlmClient>,
        channels: Vec<Arc<dyn Channel>>,
    ) -> Self {
        Self {
            name: config.name.clone(),
            sources,
            llm,
            channels,
            prompt_template: config.prompt_template.clone(),
        }
    }

    pub async fn execute(&self) -> anyhow::Result<()> {
        info!("Running digest task: {}", self.name);

        // 1. Fetch articles from all sources
        let mut all_articles = Vec::new();
        for source in &self.sources {
            match source.fetch().await {
                Ok(articles) => {
                    info!("Fetched {} articles from {}", articles.len(), source.name());
                    all_articles.extend(articles);
                }
                Err(e) => {
                    error!("Failed to fetch from {}: {}", source.name(), e);
                }
            }
        }

        if all_articles.is_empty() {
            info!("No articles fetched, skipping digest");
            return Ok(());
        }

        // 2. Build content for LLM
        let content = all_articles
            .iter()
            .enumerate()
            .map(|(i, a)| {
                format!(
                    "{}. [{}] {}\n   URL: {}\n   {}",
                    i + 1,
                    a.source,
                    a.title,
                    a.url.as_deref().unwrap_or("N/A"),
                    a.summary.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // 3. Summarize via LLM
        let digest = self.llm.summarize(&content, self.prompt_template.as_deref()).await?;
        info!("Digest generated ({} chars)", digest.len());

        // 4. Push to all channels
        let title = format!("📬 {} - {}", self.name, chrono::Local::now().format("%Y-%m-%d"));
        for channel in &self.channels {
            match channel.send(&title, &digest).await {
                Ok(_) => info!("Sent digest via {}", channel.name()),
                Err(e) => error!("Failed to send via {}: {}", channel.name(), e),
            }
        }

        Ok(())
    }
}
