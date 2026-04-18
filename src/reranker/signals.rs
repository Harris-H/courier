//! Scoring signals for article reranking.
//!
//! Inspired by last30days-skill's `signals.py`:
//! - Engagement: log1p-normalized platform-specific metrics
//! - Freshness: exponential decay based on published_at
//! - Source quality: editorial signal-to-noise weight per source

use crate::sources::Article;

/// Source quality weights (editorial signal-to-noise ratio).
/// Higher = more reliable/curated content.
///
/// Based on last30days-skill's SOURCE_QUALITY mapping, adapted for Courier's sources.
pub fn source_quality(source: &str) -> f64 {
    match source.to_lowercase().as_str() {
        "hackernews" => 0.85,
        "reddit" => 0.65,
        // RSS feeds vary but are generally curated by the user
        _ => 0.75,
    }
}

/// Compute raw engagement score for an article.
///
/// Uses log1p normalization (like last30days-skill) to compress large ranges:
/// - HN: 0.55 × log1p(score) + 0.45 × log1p(comments)
/// - Reddit (Atom): limited data, uses whatever is available
/// - RSS: typically no engagement data → 0.0
pub fn engagement_raw(article: &Article) -> f64 {
    let score_val = article.score.unwrap_or(0).max(0) as f64;
    let comments_val = article.comments_count.unwrap_or(0) as f64;

    match article.source.to_lowercase().as_str() {
        "hackernews" => {
            // HN has rich engagement: score (upvotes) + descendants (comments)
            0.55 * (score_val + 1.0).ln() + 0.45 * (comments_val + 1.0).ln()
        }
        "reddit" => {
            // Reddit Atom feed has limited engagement data
            // Score may be available, comments less reliably
            0.60 * (score_val + 1.0).ln() + 0.40 * (comments_val + 1.0).ln()
        }
        _ => {
            // RSS and other sources typically lack engagement metrics
            if score_val > 0.0 || comments_val > 0.0 {
                0.50 * (score_val + 1.0).ln() + 0.50 * (comments_val + 1.0).ln()
            } else {
                0.0
            }
        }
    }
}

/// Min-max normalize a slice of values to [0.0, 1.0].
///
/// If all values are equal (max == min), returns 0.0 for all items
/// (no differentiation possible).
pub fn min_max_normalize(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    if range < f64::EPSILON {
        return vec![0.0; values.len()];
    }

    values.iter().map(|v| (v - min) / range).collect()
}

/// Compute freshness score (0.0–1.0) based on published_at.
///
/// Uses exponential decay: score = e^(-λ × age_hours)
/// where λ = ln(2) / half_life_hours (half-life = 24 hours).
///
/// - Just published → ~1.0
/// - 24 hours ago → ~0.5
/// - 48 hours ago → ~0.25
/// - No timestamp → 0.5 (neutral, don't penalize missing data)
pub fn freshness_score(article: &Article) -> f64 {
    const HALF_LIFE_HOURS: f64 = 24.0;
    let lambda = (2.0_f64).ln() / HALF_LIFE_HOURS;

    let Some(published) = &article.published_at else {
        return 0.5; // Neutral for missing timestamps
    };

    // Try parsing common date formats
    let parsed = chrono::DateTime::parse_from_rfc3339(published)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc2822(published).map(|dt| dt.with_timezone(&chrono::Utc))
        })
        .or_else(|_| {
            // Unix timestamp
            published
                .parse::<i64>()
                .ok()
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .ok_or_else(|| chrono::DateTime::parse_from_rfc3339("invalid").unwrap_err())
        });

    match parsed {
        Ok(published_dt) => {
            let age_hours = (chrono::Utc::now() - published_dt).num_minutes() as f64 / 60.0;
            if age_hours < 0.0 {
                1.0 // Future date, treat as just published
            } else {
                (-lambda * age_hours).exp()
            }
        }
        Err(_) => 0.5, // Unparseable date → neutral
    }
}

/// Assign a heat label based on rank_score percentile position.
///
/// Called after scores are computed. The thresholds map to:
/// - 🔥热门 (Hot): top tier (rank_score >= 0.7)
/// - 📈上升 (Rising): middle tier (rank_score >= 0.4)
/// - 📰普通 (Normal): baseline
pub fn heat_label(rank_score: f64) -> &'static str {
    if rank_score >= 0.7 {
        "🔥热门"
    } else if rank_score >= 0.4 {
        "📈上升"
    } else {
        "📰普通"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article_with(
        source: &str,
        score: Option<i64>,
        comments: Option<u32>,
        published: Option<&str>,
    ) -> Article {
        Article {
            title: "test".to_string(),
            url: None,
            source: source.to_string(),
            summary: None,
            score,
            comments_count: comments,
            published_at: published.map(|s| s.to_string()),
        }
    }

    // --- source_quality tests ---

    #[test]
    fn source_quality_hackernews_highest() {
        assert!(source_quality("hackernews") > source_quality("reddit"));
        assert!(source_quality("hackernews") > source_quality("some_rss"));
    }

    #[test]
    fn source_quality_case_insensitive() {
        assert_eq!(source_quality("HackerNews"), source_quality("hackernews"));
    }

    #[test]
    fn source_quality_unknown_gets_default() {
        let q = source_quality("unknown_source");
        assert!(q > 0.0 && q < 1.0);
    }

    // --- engagement_raw tests ---

    #[test]
    fn engagement_zero_when_no_metrics() {
        let article = make_article_with("rss", None, None, None);
        assert_eq!(engagement_raw(&article), 0.0);
    }

    #[test]
    fn engagement_hn_uses_score_and_comments() {
        let high = make_article_with("hackernews", Some(500), Some(200), None);
        let low = make_article_with("hackernews", Some(5), Some(1), None);
        assert!(engagement_raw(&high) > engagement_raw(&low));
    }

    #[test]
    fn engagement_log1p_compresses_large_values() {
        let a = make_article_with("hackernews", Some(100), Some(50), None);
        let b = make_article_with("hackernews", Some(10000), Some(5000), None);
        // 100x difference in raw values should not be 100x in score
        let ratio = engagement_raw(&b) / engagement_raw(&a);
        assert!(ratio < 3.0, "log1p should compress: ratio={}", ratio);
    }

    #[test]
    fn engagement_negative_score_treated_as_zero() {
        let article = make_article_with("hackernews", Some(-10), None, None);
        let zero = make_article_with("hackernews", Some(0), None, None);
        assert_eq!(engagement_raw(&article), engagement_raw(&zero));
    }

    // --- min_max_normalize tests ---

    #[test]
    fn normalize_empty_returns_empty() {
        assert!(min_max_normalize(&[]).is_empty());
    }

    #[test]
    fn normalize_single_value_returns_zero() {
        assert_eq!(min_max_normalize(&[42.0]), vec![0.0]);
    }

    #[test]
    fn normalize_equal_values_returns_zeros() {
        assert_eq!(min_max_normalize(&[5.0, 5.0, 5.0]), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn normalize_range_maps_to_zero_one() {
        let result = min_max_normalize(&[0.0, 50.0, 100.0]);
        assert!((result[0] - 0.0).abs() < f64::EPSILON);
        assert!((result[1] - 0.5).abs() < f64::EPSILON);
        assert!((result[2] - 1.0).abs() < f64::EPSILON);
    }

    // --- freshness_score tests ---

    #[test]
    fn freshness_no_timestamp_returns_neutral() {
        let article = make_article_with("hackernews", None, None, None);
        assert!((freshness_score(&article) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn freshness_recent_article_scores_high() {
        let now = chrono::Utc::now().to_rfc3339();
        let article = make_article_with("hackernews", None, None, Some(&now));
        assert!(freshness_score(&article) > 0.95);
    }

    #[test]
    fn freshness_one_day_old_is_roughly_half() {
        let yesterday = (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        let article = make_article_with("hackernews", None, None, Some(&yesterday));
        let score = freshness_score(&article);
        assert!(
            (score - 0.5).abs() < 0.05,
            "24h old should be ~0.5, got {}",
            score
        );
    }

    #[test]
    fn freshness_old_article_scores_low() {
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
        let article = make_article_with("hackernews", None, None, Some(&week_ago));
        assert!(freshness_score(&article) < 0.01);
    }

    #[test]
    fn freshness_invalid_date_returns_neutral() {
        let article = make_article_with("hackernews", None, None, Some("not-a-date"));
        assert!((freshness_score(&article) - 0.5).abs() < f64::EPSILON);
    }

    // --- heat_label tests ---

    #[test]
    fn heat_label_thresholds() {
        assert!(heat_label(0.8).contains('🔥'));
        assert!(heat_label(0.5).contains('📈'));
        assert!(heat_label(0.2).contains('📰'));
    }

    #[test]
    fn heat_label_boundaries() {
        assert!(heat_label(0.7).contains('🔥'));
        assert!(heat_label(0.4).contains('📈'));
        assert!(heat_label(0.39).contains('📰'));
    }
}
