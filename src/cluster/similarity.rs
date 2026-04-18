//! Text and URL similarity functions for cross-source clustering.
//!
//! Uses lightweight, dependency-free algorithms:
//! - Jaccard similarity on word-level tokens for titles
//! - Canonical URL comparison for URL matching

use std::collections::HashSet;

/// Compute word-level Jaccard similarity between two strings.
///
/// Jaccard(A, B) = |A ∩ B| / |A ∪ B|
///
/// Preprocessing: lowercase, strip punctuation, split on whitespace.
/// Returns 0.0 for empty inputs, 1.0 for identical strings.
pub fn title_similarity(a: &str, b: &str) -> f64 {
    let tokens_a = tokenize(a);
    let tokens_b = tokenize(b);

    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0; // Both empty = identical
    }
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<&str> = tokens_a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = tokens_b.iter().map(|s| s.as_str()).collect();

    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    intersection / union
}

/// Tokenize a string into lowercase words, stripping common punctuation.
fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '-' || c == '–' || c == '—')
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c > '\u{007F}') // keep CJK/unicode chars
                .collect::<String>()
        })
        .filter(|word| !word.is_empty() && !is_stop_word(word))
        .collect()
}

/// Common English stop words that add noise to similarity comparison.
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "the"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "and"
            | "or"
            | "but"
            | "not"
            | "no"
            | "it"
            | "its"
            | "this"
            | "that"
            | "has"
            | "have"
            | "had"
            | "do"
            | "does"
    )
}

/// Check if two URLs point to the same resource.
///
/// Normalizes URLs by:
/// 1. Stripping protocol (http vs https)
/// 2. Stripping www. prefix
/// 3. Stripping trailing slashes
/// 4. Stripping common tracking parameters (utm_*, ref, source)
/// 5. Case-insensitive comparison
pub fn urls_match(a: &str, b: &str) -> bool {
    let norm_a = normalize_url(a);
    let norm_b = normalize_url(b);
    norm_a == norm_b
}

/// Normalize a URL for comparison
fn normalize_url(url: &str) -> String {
    let mut s = url.to_lowercase();

    // Strip protocol
    for prefix in &["https://", "http://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }

    // Strip www.
    if let Some(rest) = s.strip_prefix("www.") {
        s = rest.to_string();
    }

    // Strip query parameters (simple approach: remove everything after ?)
    if let Some(idx) = s.find('?') {
        s.truncate(idx);
    }

    // Strip fragment
    if let Some(idx) = s.find('#') {
        s.truncate(idx);
    }

    // Strip trailing slash
    while s.ends_with('/') {
        s.pop();
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- title_similarity tests ---

    #[test]
    fn identical_titles_return_one() {
        assert!(
            (title_similarity("Rust 2026 Released", "Rust 2026 Released") - 1.0).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn completely_different_titles_return_zero() {
        assert!(title_similarity("Rust programming language", "Python data science") < 0.1);
    }

    #[test]
    fn similar_titles_high_score() {
        let sim = title_similarity(
            "OpenAI announces GPT-5 model",
            "OpenAI announces new GPT-5 model release",
        );
        assert!(sim > 0.4, "Expected >0.4, got {}", sim);
    }

    #[test]
    fn case_insensitive() {
        let sim = title_similarity("Rust Release", "rust release");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_strings() {
        assert!((title_similarity("", "") - 1.0).abs() < f64::EPSILON);
        assert!((title_similarity("hello", "")).abs() < f64::EPSILON);
        assert!((title_similarity("", "world")).abs() < f64::EPSILON);
    }

    #[test]
    fn stop_words_ignored() {
        // "The Rust Programming Language" vs "Rust Programming Language"
        // Stop word "the" is removed, so these should be identical
        let sim = title_similarity("The Rust Programming Language", "Rust Programming Language");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn punctuation_stripped() {
        let sim = title_similarity("Hello, World!", "Hello World");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn chinese_titles_work() {
        let sim = title_similarity("Rust 发布 2026 版本", "Rust 发布 2026 版本更新");
        assert!(sim > 0.5, "Chinese similarity should work, got {}", sim);
    }

    // --- urls_match tests ---

    #[test]
    fn same_url_matches() {
        assert!(urls_match(
            "https://example.com/article",
            "https://example.com/article"
        ));
    }

    #[test]
    fn http_vs_https_matches() {
        assert!(urls_match(
            "http://example.com/article",
            "https://example.com/article"
        ));
    }

    #[test]
    fn www_stripped() {
        assert!(urls_match(
            "https://www.example.com/article",
            "https://example.com/article"
        ));
    }

    #[test]
    fn trailing_slash_stripped() {
        assert!(urls_match(
            "https://example.com/article/",
            "https://example.com/article"
        ));
    }

    #[test]
    fn query_params_stripped() {
        assert!(urls_match(
            "https://example.com/article?utm_source=twitter&ref=123",
            "https://example.com/article"
        ));
    }

    #[test]
    fn fragment_stripped() {
        assert!(urls_match(
            "https://example.com/article#section-2",
            "https://example.com/article"
        ));
    }

    #[test]
    fn different_urls_dont_match() {
        assert!(!urls_match(
            "https://example.com/article-a",
            "https://example.com/article-b"
        ));
    }

    #[test]
    fn different_domains_dont_match() {
        assert!(!urls_match(
            "https://hackernews.com/item/123",
            "https://reddit.com/r/rust/123"
        ));
    }

    // --- tokenize tests ---

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("Hello World Rust");
        assert_eq!(tokens, vec!["hello", "world", "rust"]);
    }

    #[test]
    fn tokenize_removes_stop_words() {
        let tokens = tokenize("The quick fox is in the box");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"in".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(tokens.contains(&"box".to_string()));
    }

    #[test]
    fn tokenize_handles_dashes() {
        let tokens = tokenize("GPT-5 cross-encoder");
        assert!(tokens.contains(&"gpt".to_string()));
        assert!(tokens.contains(&"5".to_string()));
        assert!(tokens.contains(&"cross".to_string()));
        assert!(tokens.contains(&"encoder".to_string()));
    }
}
