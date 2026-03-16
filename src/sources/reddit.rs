use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use super::{Article, Source};
use crate::config::RedditConfig;
use crate::error::{CourierError, Result};

#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    title: String,
    url: Option<String>,
    permalink: String,
    score: i64,
    num_comments: u32,
    selftext: Option<String>,
    subreddit: String,
}

pub struct RedditSource {
    client: Client,
    config: RedditConfig,
}

impl RedditSource {
    pub fn new(config: RedditConfig) -> Self {
        let client = Client::builder()
            .user_agent("courier-bot/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
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
                "https://www.reddit.com/r/{}/hot.json?limit={}",
                subreddit, self.config.top_n
            );

            debug!("Fetching r/{}", subreddit);

            let listing: RedditListing = self
                .client
                .get(&url)
                .send()
                .await?
                .json()
                .await
                .map_err(|e| CourierError::SourceFetch {
                    origin: format!("reddit/r/{}", subreddit),
                    message: e.to_string(),
                })?;

            for child in listing.data.children {
                let post = child.data;
                let summary = post.selftext.filter(|t| !t.is_empty());
                articles.push(Article {
                    title: post.title,
                    url: Some(
                        post.url
                            .unwrap_or_else(|| format!("https://reddit.com{}", post.permalink)),
                    ),
                    source: format!("r/{}", post.subreddit),
                    summary,
                    score: Some(post.score),
                    comments_count: Some(post.num_comments),
                    published_at: None,
                });
            }
        }

        articles.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(articles)
    }
}
