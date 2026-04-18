mod common;

use std::sync::Arc;

use courier::config::ScheduleConfig;
use courier::scheduler::task::DigestTask;

use common::{sample_articles, MockChannel, MockLlm, MockSource};

#[tokio::test]
async fn digest_task_fetches_summarizes_and_pushes() {
    // Arrange
    let source = MockSource::with_articles("test-source", sample_articles());
    let llm = MockLlm::with_response("## Today's Digest\n- Rust 2026 released");
    let channel = MockChannel::new("test-channel");

    let config = ScheduleConfig {
        name: "Test Digest".to_string(),
        cron: "0 0 9 * * *".to_string(),
        sources: vec!["test-source".to_string()],
        channels: vec!["test-channel".to_string()],
        prompt_template: None,
        enabled: Some(true),
        run_on_start: None,
        max_retries: Some(0),
    };

    let task = DigestTask::new(
        &config,
        vec![source as Arc<dyn courier::sources::Source>],
        llm.clone() as Arc<dyn courier::llm::LlmClient>,
        vec![channel.clone() as Arc<dyn courier::channels::Channel>],
    );

    // Act
    let stats = task.execute().await.expect("Task should succeed");

    // Assert
    assert_eq!(stats.articles_fetched, 3);
    assert!(stats.digest_content.contains("Today's Digest"));
    assert_eq!(stats.channels_sent, 1);
    assert_eq!(stats.channels_failed, 0);

    // Verify LLM was called with article content
    let llm_calls = llm.calls.lock().await;
    assert_eq!(llm_calls.len(), 1);
    assert!(llm_calls[0].contains("Rust 2026 Released"));

    // Verify channel received the digest
    let sent = channel.sent.lock().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].1.contains("Today's Digest"));
}

#[tokio::test]
async fn digest_task_fails_when_no_articles() {
    let source = MockSource::with_articles("empty-source", vec![]);
    let llm = MockLlm::with_response("digest");
    let channel = MockChannel::new("channel");

    let config = ScheduleConfig {
        name: "Empty Test".to_string(),
        cron: "0 0 9 * * *".to_string(),
        sources: vec![],
        channels: vec![],
        prompt_template: None,
        enabled: Some(true),
        run_on_start: None,
        max_retries: Some(0),
    };

    let task = DigestTask::new(
        &config,
        vec![source as Arc<dyn courier::sources::Source>],
        llm as Arc<dyn courier::llm::LlmClient>,
        vec![channel as Arc<dyn courier::channels::Channel>],
    );

    let result = task.execute().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No articles"));
}

#[tokio::test]
async fn digest_task_continues_when_one_source_fails() {
    let good_source = MockSource::with_articles("good", sample_articles());
    let bad_source = MockSource::failing("bad");
    let llm = MockLlm::with_response("digest content");
    let channel = MockChannel::new("ch");

    let config = ScheduleConfig {
        name: "Mixed Sources".to_string(),
        cron: "0 0 9 * * *".to_string(),
        sources: vec![],
        channels: vec![],
        prompt_template: None,
        enabled: Some(true),
        run_on_start: None,
        max_retries: Some(0),
    };

    let task = DigestTask::new(
        &config,
        vec![
            good_source as Arc<dyn courier::sources::Source>,
            bad_source as Arc<dyn courier::sources::Source>,
        ],
        llm as Arc<dyn courier::llm::LlmClient>,
        vec![channel as Arc<dyn courier::channels::Channel>],
    );

    let stats = task
        .execute()
        .await
        .expect("Should succeed with partial sources");
    assert_eq!(stats.articles_fetched, 3); // only from good source
}

#[tokio::test]
async fn digest_task_reports_channel_failures() {
    let source = MockSource::with_articles("src", sample_articles());
    let llm = MockLlm::with_response("digest");
    let good_ch = MockChannel::new("good-ch");
    let bad_ch = MockChannel::failing("bad-ch");

    let config = ScheduleConfig {
        name: "Channel Fail".to_string(),
        cron: "0 0 9 * * *".to_string(),
        sources: vec![],
        channels: vec![],
        prompt_template: None,
        enabled: Some(true),
        run_on_start: None,
        max_retries: Some(0),
    };

    let task = DigestTask::new(
        &config,
        vec![source as Arc<dyn courier::sources::Source>],
        llm as Arc<dyn courier::llm::LlmClient>,
        vec![
            good_ch as Arc<dyn courier::channels::Channel>,
            bad_ch as Arc<dyn courier::channels::Channel>,
        ],
    );

    let stats = task
        .execute()
        .await
        .expect("Should succeed despite channel failure");
    assert_eq!(stats.channels_sent, 1);
    assert_eq!(stats.channels_failed, 1);
}

#[tokio::test]
async fn digest_task_uses_custom_prompt_template() {
    let source = MockSource::with_articles("src", sample_articles());
    let llm = MockLlm::with_response("custom response");
    let channel = MockChannel::new("ch");

    let config = ScheduleConfig {
        name: "Custom Prompt".to_string(),
        cron: "0 0 9 * * *".to_string(),
        sources: vec![],
        channels: vec![],
        prompt_template: Some("Summarize in English only".to_string()),
        enabled: Some(true),
        run_on_start: None,
        max_retries: Some(0),
    };

    let task = DigestTask::new(
        &config,
        vec![source as Arc<dyn courier::sources::Source>],
        llm as Arc<dyn courier::llm::LlmClient>,
        vec![channel as Arc<dyn courier::channels::Channel>],
    );

    let stats = task.execute().await.expect("Should succeed");
    assert!(stats.digest_content.contains("custom response"));
}
