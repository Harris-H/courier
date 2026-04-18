use std::sync::Arc;

use courier::llm::LlmClient;
use courier::sources::Source;

/// Handles chat commands and conversations
pub struct ChatHandler {
    llm: Arc<dyn LlmClient>,
    sources: Vec<Arc<dyn Source>>,
}

impl ChatHandler {
    pub fn new(llm: Arc<dyn LlmClient>, sources: Vec<Arc<dyn Source>>) -> Self {
        Self { llm, sources }
    }

    pub async fn handle_command(&self, command: &str, _args: &str) -> String {
        match command {
            "/help" | "/start" => self.help_text(),
            "/digest" => self.generate_digest().await,
            "/sources" => self.list_sources(),
            _ => format!("未知命令: {}。输入 /help 查看可用命令。", command),
        }
    }

    pub async fn handle_message(&self, message: &str) -> String {
        // Check if it's a command
        if message.starts_with('/') {
            let parts: Vec<&str> = message.splitn(2, ' ').collect();
            let command = parts[0];
            let args = parts.get(1).unwrap_or(&"");
            return self.handle_command(command, args).await;
        }

        // Otherwise, chat with LLM
        match self.llm.chat(message, &[]).await {
            Ok(response) => response,
            Err(e) => format!("❌ 处理消息时出错: {}", e),
        }
    }

    fn help_text(&self) -> String {
        "📬 *Courier Bot*\n\n\
        可用命令：\n\
        /help - 显示帮助信息\n\
        /digest - 立即生成今日摘要\n\
        /sources - 查看已启用的数据源\n\n\
        你也可以直接发消息和我聊天 💬"
            .to_string()
    }

    async fn generate_digest(&self) -> String {
        let mut all_articles = Vec::new();

        for source in &self.sources {
            match source.fetch().await {
                Ok(articles) => all_articles.extend(articles),
                Err(e) => {
                    return format!("❌ 获取 {} 数据时出错: {}", source.name(), e);
                }
            }
        }

        if all_articles.is_empty() {
            return "📭 没有获取到任何新闻。".to_string();
        }

        let content = all_articles
            .iter()
            .enumerate()
            .map(|(i, a)| {
                format!(
                    "{}. [{}] {}\n   URL: {}",
                    i + 1,
                    a.source,
                    a.title,
                    a.url.as_deref().unwrap_or("N/A"),
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        match self.llm.summarize(&content, None).await {
            Ok(digest) => format!("📬 *今日科技日报*\n\n{}", digest),
            Err(e) => format!("❌ 生成摘要时出错: {}", e),
        }
    }

    fn list_sources(&self) -> String {
        let sources: Vec<String> = self
            .sources
            .iter()
            .map(|s| format!("• {}", s.name()))
            .collect();
        format!("📰 已启用的数据源：\n{}", sources.join("\n"))
    }
}
