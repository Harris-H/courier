use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::channels::Channel;
use crate::config::ScheduleConfig;
use crate::llm::LlmClient;
use crate::reranker::{HeuristicReranker, RankedArticle, Reranker};
use crate::sources::{Article, Source};

/// Stats returned after successful task execution
#[derive(Debug)]
pub struct TaskStats {
    pub articles_fetched: usize,
    pub digest_length: usize,
    pub digest_content: String,
    pub channels_sent: usize,
    pub channels_failed: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Running,
    Success,
    Failed,
}

/// A digest task: fetch from sources → summarize via LLM → push to channels
pub struct DigestTask {
    pub name: String,
    pub sources: Vec<Arc<dyn Source>>,
    pub llm: Arc<dyn LlmClient>,
    pub channels: RwLock<Vec<Arc<dyn Channel>>>,
    pub prompt_template: Option<String>,
    pub max_retries: u32,
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
            channels: RwLock::new(channels),
            prompt_template: config.prompt_template.clone(),
            max_retries: config.max_retries.unwrap_or(2),
        }
    }

    pub async fn execute(&self) -> anyhow::Result<TaskStats> {
        info!("📰 Running digest: {}", self.name);

        // 1. Fetch articles from all sources concurrently
        let all_articles = self.fetch_all_sources().await;

        if all_articles.is_empty() {
            return Err(anyhow::anyhow!(
                "No articles fetched for '{}': all sources returned empty (possibly due to network issues)",
                self.name
            ));
        }

        info!("Collected {} articles total", all_articles.len());

        // 2. Rerank: score articles by engagement + freshness + source quality
        let reranker = HeuristicReranker::default();
        let ranked = reranker.rerank(all_articles);
        let articles_fetched = ranked.len();

        info!(
            "Reranked {} articles (top: {} {:.2})",
            ranked.len(),
            ranked.first().map(|r| r.meta.heat_label).unwrap_or(""),
            ranked.first().map(|r| r.meta.rank_score).unwrap_or(0.0),
        );

        // 3. Build content for LLM (with heat labels)
        let content = Self::format_ranked_articles(&ranked);

        // 4. Summarize via LLM (with retry)
        let digest = self.summarize_with_retry(&content).await?;
        info!("Digest generated ({} chars)", digest.len());

        // 5. Push to all channels concurrently
        let title = format!(
            "📬 {} - {}",
            self.name,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );
        let (sent, failed) = self.push_to_channels(&title, &digest).await;

        Ok(TaskStats {
            articles_fetched,
            digest_length: digest.len(),
            digest_content: digest,
            channels_sent: sent,
            channels_failed: failed,
        })
    }

    /// Fetch from all sources concurrently, with per-source retry
    async fn fetch_all_sources(&self) -> Vec<Article> {
        let max_source_retries = self.max_retries;
        let handles: Vec<_> = self
            .sources
            .iter()
            .map(|source| {
                let source = source.clone();
                tokio::spawn(async move {
                    for attempt in 0..=max_source_retries {
                        if attempt > 0 {
                            let delay = Duration::from_secs(2u64.pow(attempt));
                            warn!(
                                "Retrying source '{}' (attempt {}/{}) after {}s",
                                source.name(),
                                attempt + 1,
                                max_source_retries + 1,
                                delay.as_secs()
                            );
                            tokio::time::sleep(delay).await;
                        }
                        match source.fetch().await {
                            Ok(articles) if !articles.is_empty() => {
                                info!("✔ {} → {} articles", source.name(), articles.len());
                                return articles;
                            }
                            Ok(_) => {
                                warn!("⚠ {} → 0 articles (empty)", source.name());
                            }
                            Err(e) => {
                                error!("✘ {} → failed: {}", source.name(), e);
                            }
                        }
                    }
                    warn!("✘ {} → all retries exhausted, skipping", source.name());
                    Vec::new()
                })
            })
            .collect();

        let mut all_articles = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(articles) => all_articles.extend(articles),
                Err(e) => error!("Source task panicked: {}", e),
            }
        }

        all_articles
    }

    /// Format articles into a text block for LLM consumption
    #[cfg(test)]
    fn format_articles(articles: &[Article]) -> String {
        articles
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let mut entry = format!("{}. [{}] {}", i + 1, a.source, a.title);
                if let Some(url) = &a.url {
                    entry.push_str(&format!("\n   URL: {}", url));
                }
                if let Some(score) = a.score {
                    entry.push_str(&format!(" | Score: {}", score));
                }
                if let Some(comments) = a.comments_count {
                    entry.push_str(&format!(" | Comments: {}", comments));
                }
                if let Some(summary) = &a.summary {
                    let truncated: String = summary.chars().take(200).collect();
                    entry.push_str(&format!("\n   {}", truncated));
                }
                entry
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format ranked articles with heat labels for LLM consumption.
    /// Articles are already sorted by rank_score (highest first).
    fn format_ranked_articles(ranked: &[RankedArticle]) -> String {
        ranked
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let a = &r.article;
                let mut entry = format!(
                    "{}. {} [{}] {}",
                    i + 1,
                    r.meta.heat_label,
                    a.source,
                    a.title
                );
                if let Some(url) = &a.url {
                    entry.push_str(&format!("\n   URL: {}", url));
                }
                if let Some(score) = a.score {
                    entry.push_str(&format!(" | Score: {}", score));
                }
                if let Some(comments) = a.comments_count {
                    entry.push_str(&format!(" | Comments: {}", comments));
                }
                entry.push_str(&format!(
                    " | Rank: {:.2} (E:{:.0}% F:{:.0}% Q:{:.0}%)",
                    r.meta.rank_score,
                    r.meta.engagement * 100.0,
                    r.meta.freshness * 100.0,
                    r.meta.source_quality * 100.0,
                ));
                if let Some(summary) = &a.summary {
                    let truncated: String = summary.chars().take(200).collect();
                    entry.push_str(&format!("\n   {}", truncated));
                }
                entry
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Call LLM with retry on transient failures
    async fn summarize_with_retry(&self, content: &str) -> anyhow::Result<String> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_secs(2u64.pow(attempt));
                warn!(
                    "Retrying LLM summarize (attempt {}/{}) after {}s",
                    attempt + 1,
                    self.max_retries + 1,
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
            }

            match self
                .llm
                .summarize(content, self.prompt_template.as_deref())
                .await
            {
                Ok(digest) => return Ok(digest),
                Err(e) => {
                    warn!("LLM call failed: {}", e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .map(|e| anyhow::anyhow!(e))
            .unwrap_or_else(|| anyhow::anyhow!("LLM summarize failed with no error")))
    }

    /// Push digest to all channels concurrently
    async fn push_to_channels(&self, title: &str, digest: &str) -> (usize, usize) {
        let channels = self.channels.read().await;
        let handles: Vec<_> = channels
            .iter()
            .map(|channel| {
                let channel = channel.clone();
                let title = title.to_string();
                let digest = digest.to_string();
                tokio::spawn(async move {
                    match channel.send(&title, &digest).await {
                        Ok(_) => {
                            info!("📤 Sent via {}", channel.name());
                            true
                        }
                        Err(e) => {
                            error!("📤 Failed via {}: {}", channel.name(), e);
                            false
                        }
                    }
                })
            })
            .collect();

        let mut sent = 0usize;
        let mut failed = 0usize;

        for handle in handles {
            match handle.await {
                Ok(true) => sent += 1,
                Ok(false) => failed += 1,
                Err(e) => {
                    error!("Channel task panicked: {}", e);
                    failed += 1;
                }
            }
        }

        (sent, failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(title: &str, source: &str) -> Article {
        Article {
            title: title.to_string(),
            url: Some("https://example.com".to_string()),
            source: source.to_string(),
            summary: None,
            score: None,
            comments_count: None,
            published_at: None,
        }
    }

    #[test]
    fn format_articles_includes_title_and_source() {
        let articles = vec![make_article("Rust 2026", "Hacker News")];
        let formatted = DigestTask::format_articles(&articles);
        assert!(formatted.contains("Rust 2026"));
        assert!(formatted.contains("[Hacker News]"));
    }

    #[test]
    fn format_articles_includes_url() {
        let articles = vec![make_article("Test", "HN")];
        let formatted = DigestTask::format_articles(&articles);
        assert!(formatted.contains("URL: https://example.com"));
    }

    #[test]
    fn format_articles_includes_score_and_comments() {
        let articles = vec![Article {
            title: "Hot Post".to_string(),
            url: None,
            source: "HN".to_string(),
            summary: None,
            score: Some(256),
            comments_count: Some(42),
            published_at: None,
        }];
        let formatted = DigestTask::format_articles(&articles);
        assert!(formatted.contains("Score: 256"));
        assert!(formatted.contains("Comments: 42"));
    }

    #[test]
    fn format_articles_truncates_long_summary() {
        let long_summary = "x".repeat(300);
        let articles = vec![Article {
            title: "Test".to_string(),
            url: None,
            source: "HN".to_string(),
            summary: Some(long_summary.clone()),
            score: None,
            comments_count: None,
            published_at: None,
        }];
        let formatted = DigestTask::format_articles(&articles);
        // Summary should be truncated to 200 chars
        assert!(formatted.contains(&"x".repeat(200)));
        assert!(!formatted.contains(&"x".repeat(300)));
    }

    #[test]
    fn format_articles_numbers_sequentially() {
        let articles = vec![
            make_article("First", "A"),
            make_article("Second", "B"),
            make_article("Third", "C"),
        ];
        let formatted = DigestTask::format_articles(&articles);
        assert!(formatted.contains("1. [A] First"));
        assert!(formatted.contains("2. [B] Second"));
        assert!(formatted.contains("3. [C] Third"));
    }

    #[test]
    fn format_articles_empty_returns_empty_string() {
        let formatted = DigestTask::format_articles(&[]);
        assert!(formatted.is_empty());
    }

    #[test]
    fn format_ranked_articles_includes_heat_labels() {
        let ranked = vec![RankedArticle {
            article: make_article("Hot Post", "hackernews"),
            meta: crate::reranker::RankMeta {
                engagement: 0.9,
                freshness: 0.8,
                source_quality: 0.85,
                rank_score: 0.85,
                heat_label: "🔥热门",
            },
        }];
        let formatted = DigestTask::format_ranked_articles(&ranked);
        assert!(formatted.contains("🔥热门"));
        assert!(formatted.contains("[hackernews]"));
        assert!(formatted.contains("Hot Post"));
        assert!(formatted.contains("Rank: 0.85"));
    }

    #[test]
    fn format_ranked_articles_includes_signal_percentages() {
        let ranked = vec![RankedArticle {
            article: make_article("Test", "reddit"),
            meta: crate::reranker::RankMeta {
                engagement: 0.5,
                freshness: 0.75,
                source_quality: 0.65,
                rank_score: 0.6,
                heat_label: "📈上升",
            },
        }];
        let formatted = DigestTask::format_ranked_articles(&ranked);
        assert!(formatted.contains("E:50%"));
        assert!(formatted.contains("F:75%"));
        assert!(formatted.contains("Q:65%"));
    }

    #[test]
    fn format_ranked_articles_empty_returns_empty_string() {
        let formatted = DigestTask::format_ranked_articles(&[]);
        assert!(formatted.is_empty());
    }
}
