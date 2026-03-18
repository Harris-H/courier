use async_trait::async_trait;
use reqwest::Client;
use tracing::{debug, warn};

use super::{Article, Source};
use crate::config::RedditConfig;
use crate::error::{CourierError, Result};

pub struct RedditSource {
    client: Client,
    config: RedditConfig,
}

impl RedditSource {
    pub fn new(config: RedditConfig) -> anyhow::Result<Self> {
        // Use native-tls to avoid TLS fingerprint-based blocking by Reddit
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .use_native_tls()
            .build()?;

        Ok(Self { client, config })
    }
}

#[async_trait]
impl Source for RedditSource {
    fn name(&self) -> &str {
        "reddit"
    }

    async fn fetch(&self) -> Result<Vec<Article>> {
        let mut articles = Vec::new();

        for subreddit in &self.config.subreddits {
            let url = format!(
                "https://www.reddit.com/r/{}/hot.rss?limit={}",
                subreddit, self.config.top_n
            );

            debug!("Fetching r/{} via Atom feed", subreddit);

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| CourierError::SourceFetch {
                    origin: format!("reddit/r/{}", subreddit),
                    message: format!("request failed: {}", e),
                })?;

            if !response.status().is_success() {
                warn!("Reddit r/{} returned status {}, skipping", subreddit, response.status());
                continue;
            }

            let body = response.text()
                .await
                .map_err(|e| CourierError::SourceFetch {
                    origin: format!("reddit/r/{}", subreddit),
                    message: format!("failed to read body: {}", e),
                })?;

            // Reddit returns Atom XML, not RSS 2.0
            let feed = body.parse::<atom_syndication::Feed>().map_err(|e| {
                warn!("Reddit r/{} Atom parse error: {}. Body preview: {}", subreddit, e, &body[..body.len().min(300)]);
                CourierError::SourceFetch {
                    origin: format!("reddit/r/{}", subreddit),
                    message: format!("Atom parse error: {}", e),
                }
            })?;

            let count_before = articles.len();
            for entry in feed.entries().iter().take(self.config.top_n) {
                let title = entry.title().as_str().to_string();
                if title.is_empty() {
                    continue;
                }

                let link = entry.links().first().map(|l| l.href().to_string());

                // Extract text from HTML content
                let summary = entry.content()
                    .and_then(|c| c.value())
                    .or_else(|| entry.summary().map(|s| s.as_str()))
                    .and_then(|html| {
                        let text = scraper::Html::parse_fragment(html)
                            .root_element()
                            .text()
                            .collect::<String>();
                        let trimmed = text.trim().to_string();
                        if trimmed.is_empty() { None } else {
                            // Truncate long summaries
                            Some(if trimmed.len() > 500 {
                                format!("{}...", &trimmed[..500])
                            } else {
                                trimmed
                            })
                        }
                    });

                articles.push(Article {
                    title,
                    url: link,
                    source: format!("r/{}", subreddit),
                    summary,
                    score: None,
                    comments_count: None,
                    published_at: entry.updated().to_rfc3339().into(),
                });
            }

            debug!("r/{}: fetched {} articles", subreddit, articles.len() - count_before);
        }

        if articles.is_empty() {
            warn!("Reddit: no articles fetched from any subreddit");
        }

        Ok(articles)
    }
}
