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

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        if !status.is_success() {
            return Err(crate::error::CourierError::SourceFetch {
                origin: feed_entry.name.clone(),
                message: format!("HTTP {} (content-type: {})", status, content_type),
            });
        }

        if !content_type.contains("xml")
            && !content_type.contains("rss")
            && !content_type.contains("atom")
        {
            warn!(
                "RSS feed '{}' returned unexpected content-type: {}",
                feed_entry.name, content_type
            );
        }

        let body = response
            .bytes()
            .await
            .map_err(|e| crate::error::CourierError::SourceFetch {
                origin: feed_entry.name.clone(),
                message: e.to_string(),
            })?;

        // Try RSS 2.0 first, then fall back to Atom format
        if let Ok(channel) = rss::Channel::read_from(&body[..]) {
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
            return Ok(articles);
        }

        // Fall back to Atom parsing (RSSHub often returns Atom format)
        let body_str = String::from_utf8_lossy(&body);
        match body_str.parse::<atom_syndication::Feed>() {
            Ok(feed) => {
                let articles = feed
                    .entries()
                    .iter()
                    .map(|entry| {
                        let summary = entry
                            .content()
                            .and_then(|c| c.value())
                            .or_else(|| entry.summary().map(|s| s.as_str()))
                            .map(|html| {
                                let text = scraper::Html::parse_fragment(html)
                                    .root_element()
                                    .text()
                                    .collect::<String>();
                                let trimmed = text.trim().to_string();
                                if trimmed.len() > 500 {
                                    format!("{}...", &trimmed[..500])
                                } else {
                                    trimmed
                                }
                            })
                            .filter(|s| !s.is_empty());

                        Article {
                            title: entry.title().as_str().to_string(),
                            url: entry.links().first().map(|l| l.href().to_string()),
                            source: feed_entry.name.clone(),
                            summary,
                            score: None,
                            comments_count: None,
                            published_at: entry.published().map(|d| d.to_string()),
                        }
                    })
                    .collect();
                Ok(articles)
            }
            Err(atom_err) => {
                let preview = &body_str[..body_str.len().min(500)];
                warn!(
                    "RSS feed '{}' failed both RSS and Atom parsing (status: {}, content-type: {}), body preview: {}",
                    feed_entry.name, status, content_type, preview
                );
                Err(crate::error::CourierError::SourceFetch {
                    origin: feed_entry.name.clone(),
                    message: format!(
                        "not valid RSS or Atom: {}",
                        atom_err
                    ),
                })
            }
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
            let feed_articles = self.fetch_feed_with_retry(feed_entry).await;
            articles.extend(feed_articles);
        }

        Ok(articles)
    }
}
