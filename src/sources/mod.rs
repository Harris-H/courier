pub mod hackernews;
pub mod reddit;
pub mod rss;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// An article fetched from a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub title: String,
    pub url: Option<String>,
    pub source: String,
    pub summary: Option<String>,
    pub score: Option<i64>,
    pub comments_count: Option<u32>,
    pub published_at: Option<String>,
}

/// Trait for news/content sources
#[async_trait]
pub trait Source: Send + Sync {
    /// Source identifier
    fn name(&self) -> &str;

    /// Fetch articles from this source
    async fn fetch(&self) -> Result<Vec<Article>>;
}
