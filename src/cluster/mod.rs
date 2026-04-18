//! Cross-source article clustering.
//!
//! When multiple sources report the same story (e.g., HN + Reddit + RSS),
//! cluster them into a single entry with multi-source validation.
//!
//! Approach: greedy clustering with two similarity signals:
//! 1. URL matching — same canonical URL = definite match
//! 2. Title similarity — Jaccard similarity on word tokens (threshold-based)
//!
//! Inspired by last30days-skill's `dedupe.py` and fusion.py `candidate_key()`.

pub mod similarity;

use crate::reranker::RankedArticle;

/// A cluster of articles about the same story from one or more sources.
#[derive(Debug, Clone)]
pub struct ArticleCluster {
    /// The best representative article (highest rank_score)
    pub representative: RankedArticle,
    /// All articles in this cluster (including the representative)
    pub members: Vec<RankedArticle>,
    /// Unique source names that reported this story
    pub sources: Vec<String>,
    /// Multi-source validation label
    pub cross_source_label: String,
}

impl ArticleCluster {
    /// Number of distinct sources in this cluster
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Whether this cluster has cross-source validation (>1 source)
    pub fn is_cross_validated(&self) -> bool {
        self.sources.len() > 1
    }
}

/// Configuration for the clustering algorithm
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Jaccard similarity threshold for title matching (0.0–1.0)
    /// Default: 0.45 (fairly permissive to catch rephrased headlines)
    pub title_similarity_threshold: f64,
    /// Whether to use URL matching (same URL = same story)
    pub use_url_matching: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            title_similarity_threshold: 0.45,
            use_url_matching: true,
        }
    }
}

/// Cluster ranked articles by similarity.
///
/// Uses greedy single-linkage clustering:
/// 1. For each article, check if it belongs to an existing cluster
/// 2. If yes (URL match or title similarity above threshold), merge
/// 3. If no, create a new cluster
///
/// Returns clusters sorted by the representative's rank_score (descending).
pub fn cluster_articles(
    articles: Vec<RankedArticle>,
    config: &ClusterConfig,
) -> Vec<ArticleCluster> {
    if articles.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<ArticleCluster> = Vec::new();

    for article in articles {
        let mut merged = false;

        for cluster in &mut clusters {
            if should_merge(&article, cluster, config) {
                // Add to existing cluster
                let source = article.article.source.clone();
                if !cluster.sources.contains(&source) {
                    cluster.sources.push(source);
                }
                // Update representative if this article has higher rank_score
                if article.meta.rank_score > cluster.representative.meta.rank_score {
                    cluster.representative = article.clone();
                }
                cluster.members.push(article.clone());
                update_cross_source_label(cluster);
                merged = true;
                break;
            }
        }

        if !merged {
            let source = article.article.source.clone();
            clusters.push(ArticleCluster {
                representative: article.clone(),
                members: vec![article],
                sources: vec![source],
                cross_source_label: String::new(),
            });
        }
    }

    // Sort clusters by representative rank_score
    clusters.sort_by(|a, b| {
        b.representative
            .meta
            .rank_score
            .partial_cmp(&a.representative.meta.rank_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    clusters
}

/// Check if an article should merge into an existing cluster
fn should_merge(article: &RankedArticle, cluster: &ArticleCluster, config: &ClusterConfig) -> bool {
    // Check against all cluster members (single-linkage)
    for member in &cluster.members {
        // URL match (strongest signal)
        if config.use_url_matching {
            if let (Some(url_a), Some(url_b)) = (&article.article.url, &member.article.url) {
                if similarity::urls_match(url_a, url_b) {
                    return true;
                }
            }
        }

        // Title similarity
        let sim = similarity::title_similarity(&article.article.title, &member.article.title);
        if sim >= config.title_similarity_threshold {
            return true;
        }
    }

    false
}

/// Update the cross-source validation label
fn update_cross_source_label(cluster: &mut ArticleCluster) {
    if cluster.sources.len() >= 3 {
        cluster.cross_source_label = format!("🔗 {}源验证", cluster.sources.len());
    } else if cluster.sources.len() == 2 {
        cluster.cross_source_label = "🔗 双源验证".to_string();
    } else {
        cluster.cross_source_label = String::new();
    }
}

/// Format clustered articles for LLM consumption.
///
/// Each cluster shows:
/// - The representative article with heat label
/// - Cross-source validation badge if multi-source
/// - Source list for multi-source clusters
pub fn format_clustered_articles(clusters: &[ArticleCluster]) -> String {
    clusters
        .iter()
        .enumerate()
        .map(|(i, cluster)| {
            let r = &cluster.representative;
            let a = &r.article;

            let mut entry = format!(
                "{}. {} [{}] {}",
                i + 1,
                r.meta.heat_label,
                a.source,
                a.title
            );

            // Add cross-source badge
            if cluster.is_cross_validated() {
                let sources_str = cluster.sources.join(", ");
                entry.push_str(&format!(
                    " {} (also on: {})",
                    cluster.cross_source_label, sources_str
                ));
            }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reranker::RankMeta;
    use crate::sources::Article;

    fn make_ranked(title: &str, source: &str, url: &str, rank_score: f64) -> RankedArticle {
        RankedArticle {
            article: Article {
                title: title.to_string(),
                url: Some(url.to_string()),
                source: source.to_string(),
                summary: None,
                score: Some(100),
                comments_count: Some(50),
                published_at: None,
            },
            meta: RankMeta {
                engagement: 0.5,
                freshness: 0.5,
                source_quality: 0.8,
                rank_score,
                heat_label: if rank_score >= 0.7 {
                    "🔥热门"
                } else {
                    "📰普通"
                },
            },
        }
    }

    #[test]
    fn cluster_empty_returns_empty() {
        let result = cluster_articles(vec![], &ClusterConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn cluster_single_article_creates_one_cluster() {
        let articles = vec![make_ranked(
            "Rust 2026",
            "hackernews",
            "https://example.com/rust",
            0.8,
        )];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].source_count(), 1);
        assert!(!clusters[0].is_cross_validated());
    }

    #[test]
    fn cluster_same_url_merges() {
        let articles = vec![
            make_ranked(
                "Rust 2026 Release",
                "hackernews",
                "https://blog.rust-lang.org/2026",
                0.9,
            ),
            make_ranked(
                "Rust 2026 is out!",
                "reddit",
                "https://blog.rust-lang.org/2026",
                0.7,
            ),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].source_count(), 2);
        assert!(clusters[0].is_cross_validated());
        assert_eq!(clusters[0].members.len(), 2);
    }

    #[test]
    fn cluster_similar_titles_merges() {
        let articles = vec![
            make_ranked(
                "OpenAI announces GPT-5 model",
                "hackernews",
                "https://hn.com/1",
                0.9,
            ),
            make_ranked(
                "OpenAI announces GPT-5 new model",
                "reddit",
                "https://reddit.com/1",
                0.7,
            ),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1, "similar titles should cluster");
        assert!(clusters[0].is_cross_validated());
    }

    #[test]
    fn cluster_different_stories_stay_separate() {
        let articles = vec![
            make_ranked(
                "Rust 2026 Release Notes",
                "hackernews",
                "https://rust.org/2026",
                0.8,
            ),
            make_ranked(
                "Python 4.0 Beta Available",
                "reddit",
                "https://python.org/4.0",
                0.7,
            ),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn cluster_picks_highest_rank_as_representative() {
        let articles = vec![
            make_ranked("Big News", "reddit", "https://example.com/news", 0.6),
            make_ranked("Big News", "hackernews", "https://example.com/news", 0.9),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].representative.article.source, "hackernews");
    }

    #[test]
    fn cluster_sorted_by_representative_rank() {
        let articles = vec![
            make_ranked("Low story", "hackernews", "https://low.com", 0.3),
            make_ranked("High story", "reddit", "https://high.com", 0.9),
            make_ranked("Mid story", "hackernews", "https://mid.com", 0.6),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert!(
            clusters[0].representative.meta.rank_score
                >= clusters[1].representative.meta.rank_score
        );
        assert!(
            clusters[1].representative.meta.rank_score
                >= clusters[2].representative.meta.rank_score
        );
    }

    #[test]
    fn cluster_three_sources_label() {
        let articles = vec![
            make_ranked("Same Story", "hackernews", "https://example.com/same", 0.9),
            make_ranked("Same Story", "reddit", "https://example.com/same", 0.7),
            make_ranked("Same Story", "rss", "https://example.com/same", 0.5),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].source_count(), 3);
        assert!(clusters[0].cross_source_label.contains("3源验证"));
    }

    #[test]
    fn format_clustered_includes_cross_source_badge() {
        let articles = vec![
            make_ranked(
                "Shared Story",
                "hackernews",
                "https://example.com/shared",
                0.8,
            ),
            make_ranked("Shared Story", "reddit", "https://example.com/shared", 0.6),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        let formatted = format_clustered_articles(&clusters);
        assert!(formatted.contains("🔗 双源验证"));
        assert!(formatted.contains("hackernews"));
        assert!(formatted.contains("reddit"));
    }

    #[test]
    fn format_clustered_empty_returns_empty() {
        assert!(format_clustered_articles(&[]).is_empty());
    }

    #[test]
    fn cluster_preserves_all_articles_total() {
        let articles = vec![
            make_ranked("A", "hackernews", "https://a.com", 0.9),
            make_ranked("A", "reddit", "https://a.com", 0.7),
            make_ranked("B", "hackernews", "https://b.com", 0.5),
        ];
        let clusters = cluster_articles(articles, &ClusterConfig::default());
        let total: usize = clusters.iter().map(|c| c.members.len()).sum();
        assert_eq!(total, 3);
    }
}
