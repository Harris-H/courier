pub mod email;
pub mod feishu;
pub mod telegram;

use async_trait::async_trait;

use crate::error::Result;

/// Trait for message delivery channels
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel identifier
    fn name(&self) -> &str;

    /// Send a digest message
    async fn send(&self, title: &str, content: &str) -> Result<()>;
}
