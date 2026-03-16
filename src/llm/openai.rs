use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionRequestAssistantMessage,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;

use super::LlmClient;
use crate::config::LlmConfig;
use crate::error::{CourierError, Result};

const DEFAULT_SYSTEM_PROMPT: &str = r#"你是一个专业的科技新闻编辑。你的任务是将给定的新闻列表整理成一份精炼的中文日报摘要。

要求：
1. 按主题分类整理（如：AI/ML、编程语言、开源项目、行业动态等）
2. 每条新闻用 1-2 句话概括要点
3. 保留原文链接
4. 在末尾给出今日亮点总结（2-3 句话）
5. 使用简化 Markdown 格式输出：仅使用 **粗体**、[链接](url)、列表（- 或 1.）、分割线（---）。分类标题使用 **粗体** 而非 # 标题语法
6. 语言简洁有力，避免冗余"#;

pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
    model: String,
    max_tokens: u32,
    system_prompt: String,
}

impl OpenAIClient {
    pub fn new(config: &LlmConfig) -> Self {
        let openai_config = OpenAIConfig::new()
            .with_api_base(&config.api_base)
            .with_api_key(&config.api_key);

        let client = Client::with_config(openai_config);

        let system_prompt = config
            .system_prompt
            .clone()
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

        Self {
            client,
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            system_prompt,
        }
    }
}

#[async_trait]
impl LlmClient for OpenAIClient {
    async fn summarize(&self, content: &str, custom_prompt: Option<&str>) -> Result<String> {
        let system = custom_prompt.unwrap_or(&self.system_prompt);

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .max_tokens(self.max_tokens as u16)
            .messages(vec![
                ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage::from(system),
                ),
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage::from(format!(
                        "请整理以下新闻列表为日报摘要：\n\n{}",
                        content
                    )),
                ),
            ])
            .build()
            .map_err(|e| CourierError::Llm(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| CourierError::Llm(e.to_string()))?;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "No response from LLM".to_string());

        Ok(content)
    }

    async fn chat(&self, message: &str, history: &[(String, String)]) -> Result<String> {
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage::from(
                    "你是 Courier Bot，一个友好的个人助手。你可以帮助用户获取新闻摘要、回答问题。",
                ),
            ),
        ];

        // Add conversation history
        for (user_msg, assistant_msg) in history {
            messages.push(ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage::from(user_msg.as_str()),
            ));
            messages.push(ChatCompletionRequestMessage::Assistant(
                ChatCompletionRequestAssistantMessage::from(assistant_msg.as_str()),
            ));
        }

        // Add current message
        messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage::from(message),
        ));

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .max_tokens(self.max_tokens as u16)
            .messages(messages)
            .build()
            .map_err(|e| CourierError::Llm(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| CourierError::Llm(e.to_string()))?;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "No response".to_string());

        Ok(content)
    }
}
