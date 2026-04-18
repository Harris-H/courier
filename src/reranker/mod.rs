pub mod signals;

use crate::sources::Article;

/// Metadata attached to each article after reranking
#[derive(Debug, Clone)]
pub struct RankMeta {
    /// Normalized engagement score (0.0–1.0)
    pub engagement: f64,
    /// Freshness decay score (0.0–1.0, 1.0 = just published)
    pub freshness: f64,
    /// Source editorial quality weight (0.0–1.0)
    pub source_quality: f64,
    /// Composite rank score (higher = more important)
    pub rank_score: f64,
    /// Heat label for display: 🔥热门 / 📈上升 / 📰普通
    pub heat_label: &'static str,
}

/// An article paired with its ranking metadata
#[derive(Debug, Clone)]
pub struct RankedArticle {
    pub article: Article,
    pub meta: RankMeta,
}

/// Trait for reranking strategies.
///
/// Inspired by RAG rerankers (FlashRank, Cohere, BGE-reranker) but adapted
/// for news digest: score articles by engagement + freshness + source quality
/// rather than query-document relevance.
pub trait Reranker: Send + Sync {
    /// Rerank articles in-place, returning them sorted by descending rank_score
    /// with metadata attached.
    fn rerank(&self, articles: Vec<Article>) -> Vec<RankedArticle>;
}

/// Heuristic reranker: weighted combination of engagement, freshness, source quality.
///
/// Modeled after last30days-skill's `signals.py` local_rank_score formula,
/// adapted for Courier's 3-source setup (HN/Reddit/RSS).
///
/// Formula: rank_score = w_e × engagement + w_f × freshness + w_q × source_quality
pub struct HeuristicReranker {
    pub weight_engagement: f64,
    pub weight_freshness: f64,
    pub weight_source_quality: f64,
}

impl Default for HeuristicReranker {
    fn default() -> Self {
        Self {
            weight_engagement: 0.50,
            weight_freshness: 0.35,
            weight_source_quality: 0.15,
        }
    }
}

impl HeuristicReranker {
    pub fn new(weight_engagement: f64, weight_freshness: f64, weight_source_quality: f64) -> Self {
        Self {
            weight_engagement,
            weight_freshness,
            weight_source_quality,
        }
    }
}

impl Reranker for HeuristicReranker {
    fn rerank(&self, articles: Vec<Article>) -> Vec<RankedArticle> {
        if articles.is_empty() {
            return Vec::new();
        }

        // Phase 1: Compute raw engagement scores and normalize across the batch
        let raw_engagements: Vec<f64> = articles.iter().map(signals::engagement_raw).collect();
        let norm_engagements = signals::min_max_normalize(&raw_engagements);

        // Phase 2: Compute freshness + source quality per article
        let mut ranked: Vec<RankedArticle> = articles
            .into_iter()
            .zip(norm_engagements)
            .map(|(article, engagement)| {
                let freshness = signals::freshness_score(&article);
                let source_quality = signals::source_quality(&article.source);

                let rank_score = self.weight_engagement * engagement
                    + self.weight_freshness * freshness
                    + self.weight_source_quality * source_quality;

                let heat_label = signals::heat_label(rank_score);

                RankedArticle {
                    article,
                    meta: RankMeta {
                        engagement,
                        freshness,
                        source_quality,
                        rank_score,
                        heat_label,
                    },
                }
            })
            .collect();

        // Phase 3: Sort by rank_score descending
        ranked.sort_by(|a, b| {
            b.meta
                .rank_score
                .partial_cmp(&a.meta.rank_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(
        title: &str,
        source: &str,
        score: Option<i64>,
        comments: Option<u32>,
    ) -> Article {
        Article {
            title: title.to_string(),
            url: Some("https://example.com".to_string()),
            source: source.to_string(),
            summary: None,
            score,
            comments_count: comments,
            published_at: None,
        }
    }

    #[test]
    fn rerank_empty_returns_empty() {
        let reranker = HeuristicReranker::default();
        let result = reranker.rerank(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn rerank_single_article_gets_metadata() {
        let reranker = HeuristicReranker::default();
        let articles = vec![make_article("Test", "hackernews", Some(100), Some(50))];
        let result = reranker.rerank(articles);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].article.title, "Test");
        // Single item normalized engagement → 0.0 (min == max)
        assert!(result[0].meta.rank_score >= 0.0);
        assert!(!result[0].meta.heat_label.is_empty());
    }

    #[test]
    fn rerank_sorts_by_engagement_descending() {
        let reranker = HeuristicReranker::default();
        let articles = vec![
            make_article("Low", "hackernews", Some(10), Some(2)),
            make_article("High", "hackernews", Some(500), Some(200)),
            make_article("Medium", "hackernews", Some(100), Some(30)),
        ];
        let result = reranker.rerank(articles);

        assert_eq!(result[0].article.title, "High");
        assert_eq!(result[1].article.title, "Medium");
        assert_eq!(result[2].article.title, "Low");
    }

    #[test]
    fn rerank_higher_source_quality_boosts_rank() {
        // HN (0.85) vs Reddit (0.65) — same engagement/freshness
        let reranker = HeuristicReranker::new(0.0, 0.0, 1.0); // only source quality
        let articles = vec![
            make_article("Reddit Post", "reddit", Some(100), Some(50)),
            make_article("HN Post", "hackernews", Some(100), Some(50)),
        ];
        let result = reranker.rerank(articles);

        assert_eq!(result[0].article.title, "HN Post");
        assert!(result[0].meta.source_quality > result[1].meta.source_quality);
    }

    #[test]
    fn heat_labels_assigned_correctly() {
        let reranker = HeuristicReranker::default();
        let articles = vec![
            make_article("Hot", "hackernews", Some(1000), Some(500)),
            make_article("Warm", "hackernews", Some(50), Some(10)),
            make_article("Cold", "hackernews", Some(1), Some(0)),
        ];
        let result = reranker.rerank(articles);

        // Top article should have 🔥 or 📈, bottom should have 📰
        assert!(
            result[0].meta.heat_label.contains('🔥') || result[0].meta.heat_label.contains('📈')
        );
        assert!(result.last().unwrap().meta.heat_label.contains('📰'));
    }

    #[test]
    fn rerank_preserves_all_articles() {
        let reranker = HeuristicReranker::default();
        let articles = vec![
            make_article("A", "hackernews", Some(10), Some(1)),
            make_article("B", "reddit", None, None),
            make_article("C", "rss", None, None),
        ];
        let result = reranker.rerank(articles);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn metadata_scores_within_bounds() {
        let reranker = HeuristicReranker::default();
        let articles = vec![
            make_article("A", "hackernews", Some(999), Some(300)),
            make_article("B", "reddit", Some(1), Some(0)),
        ];
        let result = reranker.rerank(articles);

        for ranked in &result {
            assert!(ranked.meta.engagement >= 0.0 && ranked.meta.engagement <= 1.0);
            assert!(ranked.meta.freshness >= 0.0 && ranked.meta.freshness <= 1.0);
            assert!(ranked.meta.source_quality >= 0.0 && ranked.meta.source_quality <= 1.0);
            assert!(ranked.meta.rank_score >= 0.0 && ranked.meta.rank_score <= 1.0);
        }
    }
}
