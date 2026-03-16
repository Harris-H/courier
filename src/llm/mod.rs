pub mod openai;

use async_trait::async_trait;

use crate::error::Result;

/// Trait for LLM clients
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Summarize raw content into a digest
    async fn summarize(&self, content: &str, custom_prompt: Option<&str>) -> Result<String>;

    /// Chat with a user message, returning the response
    async fn chat(&self, message: &str, history: &[(String, String)]) -> Result<String>;
}
