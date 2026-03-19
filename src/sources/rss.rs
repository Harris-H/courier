use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, warn};

use super::{Article, Source};
use crate::config::RssConfig;
use crate::error::Result;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: u32 = 2;

pub struct RssSource {
    client: Client,
    feeds: Vec<RssFeedEntry>,
    source_name: String,
}

use crate::config::RssFeedEntry;

impl RssSource {
    fn build_client() -> Client {
        Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    #[allow(dead_code)]
    pub fn new(config: RssConfig) -> Self {
        Self {
            client: Self::build_client(),
            feeds: config.feeds,
            source_name: "rss".to_string(),
        }
    }

    /// Create a source for a single RSS feed (e.g. "rss:V2EX 热门")
    pub fn new_single(feed: RssFeedEntry) -> Self {
        let name = format!("rss:{}", feed.name);
        Self {
            client: Self::build_client(),
            feeds: vec![feed],
            source_name: name,
        }
    }

    /// Fetch a single feed with retry on transient failures
    async fn fetch_feed_with_retry(&self, feed_entry: &RssFeedEntry) -> Vec<Article> {
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Duration::from_secs(2u64.pow(attempt));
                warn!(
                    "Retrying RSS feed '{}' (attempt {}/{}) after {}s",
                    feed_entry.name,
                    attempt + 1,
                    MAX_RETRIES + 1,
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
            }

            match self.fetch_single_feed(feed_entry).await {
                Ok(articles) => return articles,
                Err(e) => {
                    warn!("RSS feed '{}' fetch failed: {}", feed_entry.name, e);
                }
            }
        }

        warn!(
            "RSS feed '{}' failed after {} retries, skipping",
            feed_entry.name,
            MAX_RETRIES
        );
        Vec::new()
    }

    async fn fetch_single_feed(&self, feed_entry: &RssFeedEntry) -> Result<Vec<Article>> {
        let response = self
            .client
            .get(&feed_entry.url)
            .send()
            .await
            .map_err(|e| crate::error::CourierError::SourceFetch {
                origin: feed_entry.name.clone(),
                message: e.to_string(),
            })?;

        let body = response
            .bytes()
            .await
            .map_err(|e| crate::error::CourierError::SourceFetch {
                origin: feed_entry.name.clone(),
                message: e.to_string(),
            })?;

        let channel =
            rss::Channel::read_from(&body[..]).map_err(|e| crate::error::CourierError::SourceFetch {
                origin: feed_entry.name.clone(),
                message: e.to_string(),
            })?;

        let articles = channel
            .items()
            .iter()
            .map(|item| Article {
                title: item.title().unwrap_or("Untitled").to_string(),
                url: item.link().map(|s| s.to_string()),
                source: feed_entry.name.clone(),
                summary: item.description().map(|s| s.to_string()),
                score: None,
                comments_count: None,
                published_at: item.pub_date().map(|s| s.to_string()),
            })
            .collect();

        Ok(articles)
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
            let feed_articles = self.fetch_feed_with_retry(feed_entry).await;
            articles.extend(feed_articles);
        }

        Ok(articles)
    }
}
