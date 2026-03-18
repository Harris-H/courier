use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;

use super::{Article, Source};
use crate::config::RssConfig;
use crate::error::{CourierError, Result};

pub struct RssSource {
    client: Client,
    feeds: Vec<RssFeedEntry>,
    source_name: String,
}

use crate::config::RssFeedEntry;

impl RssSource {
    #[allow(dead_code)]
    pub fn new(config: RssConfig) -> Self {
        Self {
            client: Client::new(),
            feeds: config.feeds,
            source_name: "rss".to_string(),
        }
    }

    /// Create a source for a single RSS feed (e.g. "rss:V2EX 热门")
    pub fn new_single(feed: RssFeedEntry) -> Self {
        let name = format!("rss:{}", feed.name);
        Self {
            client: Client::new(),
            feeds: vec![feed],
            source_name: name,
        }
    }
}

#[async_trait]
impl Source for RssSource {
    fn name(&self) -> &str {
        &self.source_name
    }

    async fn fetch(&self) -> Result<Vec<Article>> {
        let mut articles = Vec::new();

        for feed_entry in &self.feeds {
            debug!("Fetching RSS: {}", feed_entry.name);

            let body = self
                .client
                .get(&feed_entry.url)
                .send()
                .await?
                .bytes()
                .await
                .map_err(|e| CourierError::SourceFetch {
                    origin: feed_entry.name.clone(),
                    message: e.to_string(),
                })?;

            let channel = rss::Channel::read_from(&body[..]).map_err(|e| {
                CourierError::SourceFetch {
                    origin: feed_entry.name.clone(),
                    message: e.to_string(),
                }
            })?;

            for item in channel.items() {
                let title = item.title().unwrap_or("Untitled").to_string();
                let url = item.link().map(|s| s.to_string());
                let summary = item.description().map(|s| s.to_string());
                let published_at = item.pub_date().map(|s| s.to_string());

                articles.push(Article {
                    title,
                    url,
                    source: feed_entry.name.clone(),
                    summary,
                    score: None,
                    comments_count: None,
                    published_at,
                });
            }
        }

        Ok(articles)
    }
}
