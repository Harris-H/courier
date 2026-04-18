use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use courier::channels::Channel;
use courier::error::Result;
use courier::llm::LlmClient;
use courier::sources::{Article, Source};

// ─── MockSource ───────────────────────────────────────────────

pub struct MockSource {
    pub name: String,
    pub articles: Vec<Article>,
    pub should_fail: bool,
}

impl MockSource {
    pub fn with_articles(name: &str, articles: Vec<Article>) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            articles,
            should_fail: false,
        })
    }

    pub fn failing(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            articles: vec![],
            should_fail: true,
        })
    }
}

#[async_trait]
impl Source for MockSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn fetch(&self) -> Result<Vec<Article>> {
        if self.should_fail {
            Err(courier::error::CourierError::SourceFetch {
                origin: self.name.clone(),
                message: "mock failure".into(),
            })
        } else {
            Ok(self.articles.clone())
        }
    }
}

// ─── MockChannel ──────────────────────────────────────────────

pub struct MockChannel {
    pub name: String,
    pub sent: Mutex<Vec<(String, String)>>,
    pub should_fail: bool,
}

impl MockChannel {
    pub fn new(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            sent: Mutex::new(vec![]),
            should_fail: false,
        })
    }

    pub fn failing(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            sent: Mutex::new(vec![]),
            should_fail: true,
        })
    }
}

#[async_trait]
impl Channel for MockChannel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn send(&self, title: &str, content: &str) -> Result<()> {
        if self.should_fail {
            Err(courier::error::CourierError::ChannelSend {
                channel: self.name.clone(),
                message: "mock failure".into(),
            })
        } else {
            self.sent
                .lock()
                .await
                .push((title.to_string(), content.to_string()));
            Ok(())
        }
    }
}

// ─── MockLlm ─────────────────────────────────────────────────

pub struct MockLlm {
    pub response: String,
    pub should_fail: bool,
    pub calls: Mutex<Vec<String>>,
}

impl MockLlm {
    pub fn with_response(response: &str) -> Arc<Self> {
        Arc::new(Self {
            response: response.to_string(),
            should_fail: false,
            calls: Mutex::new(vec![]),
        })
    }

    #[allow(dead_code)]
    pub fn failing() -> Arc<Self> {
        Arc::new(Self {
            response: String::new(),
            should_fail: true,
            calls: Mutex::new(vec![]),
        })
    }
}

#[async_trait]
impl LlmClient for MockLlm {
    async fn summarize(&self, content: &str, _custom_prompt: Option<&str>) -> Result<String> {
        self.calls.lock().await.push(content.to_string());
        if self.should_fail {
            Err(courier::error::CourierError::Llm("mock LLM failure".into()))
        } else {
            Ok(self.response.clone())
        }
    }

    async fn chat(&self, message: &str, _history: &[(String, String)]) -> Result<String> {
        self.calls.lock().await.push(message.to_string());
        if self.should_fail {
            Err(courier::error::CourierError::Llm("mock LLM failure".into()))
        } else {
            Ok(self.response.clone())
        }
    }
}

// ─── Fixture helpers ──────────────────────────────────────────

pub fn make_article(title: &str, source: &str) -> Article {
    Article {
        title: title.to_string(),
        url: Some(format!(
            "https://example.com/{}",
            title.to_lowercase().replace(' ', "-")
        )),
        source: source.to_string(),
        summary: Some(format!("Summary of {}", title)),
        score: Some(100),
        comments_count: Some(10),
        published_at: None,
    }
}

pub fn sample_articles() -> Vec<Article> {
    vec![
        make_article("Rust 2026 Released", "Hacker News"),
        make_article("New AI Model Breaks Records", "Hacker News"),
        make_article("Linux 7.0 Announced", "r/linux"),
    ]
}
