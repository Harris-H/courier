use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::debug;

use super::{Article, Source};
use crate::config::HackerNewsConfig;
use crate::error::{CourierError, Result};

const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";

#[derive(Debug, Deserialize)]
struct HnItem {
    #[allow(dead_code)]
    id: u64,
    title: Option<String>,
    url: Option<String>,
    score: Option<i64>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    item_type: Option<String>,
    descendants: Option<u32>,
    #[serde(default)]
    text: Option<String>,
}

pub struct HackerNewsSource {
    client: Client,
    config: HackerNewsConfig,
}

impl HackerNewsSource {
    pub fn new(config: HackerNewsConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client, config }
    }
}

#[async_trait]
impl Source for HackerNewsSource {
    fn name(&self) -> &str {
        "hackernews"
    }

    async fn fetch(&self) -> Result<Vec<Article>> {
        let url = format!("{}/topstories.json", HN_API_BASE);
        let ids: Vec<u64> = self
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| CourierError::SourceFetch {
                origin: "hackernews".into(),
                message: e.to_string(),
            })?;

        let top_ids: Vec<u64> = ids.into_iter().take(self.config.top_n).collect();
        debug!("Fetching {} HN stories", top_ids.len());

        let mut articles = Vec::new();
        // Fetch items concurrently in batches
        let mut handles = Vec::new();
        for id in top_ids {
            let client = self.client.clone();
            handles.push(tokio::spawn(async move {
                let url = format!("{}/item/{}.json", HN_API_BASE, id);
                let resp = client.get(&url).send().await;
                match resp {
                    Ok(r) => r.json::<HnItem>().await.ok(),
                    Err(_) => None,
                }
            }));
        }

        for handle in handles {
            if let Ok(Some(item)) = handle.await {
                if let Some(title) = item.title {
                    articles.push(Article {
                        title,
                        url: item.url,
                        source: "Hacker News".to_string(),
                        summary: item.text,
                        score: item.score,
                        comments_count: item.descendants,
                        published_at: None,
                    });
                }
            }
        }

        // Sort by score descending
        articles.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(articles)
    }
}
