# Courier 📬

> 轻量级个人新闻摘要推送 Bot — 抓取、总结、推送，一气呵成。

[English](./README.md)

## 功能

- 📰 **多数据源抓取**: Hacker News / Reddit / RSS（支持 RSSHub）
- 🤖 **LLM 智能摘要**: 调用 OpenAI 兼容 API 生成日报
- 📮 **多通道推送**: Telegram / 飞书 / Email（SMTP，Markdown→HTML 富文本渲染）
- ⏰ **定时调度**: Cron 表达式灵活配置
- 💬 **聊天模式**: 通过 Telegram Bot 交互式对话
- 🖥️ **Web 仪表盘**: Vue.js 管理面板，实时查看状态
- 🔄 **热重载配置**: 邮件、频道、LLM 配置修改无需重启即刻生效
- 🔒 **安全特性**: 可选 API 密钥认证、敏感信息脱敏、输入校验

## 架构

```
Source(HN/Reddit/RSS) → LLM(摘要) → Channel(TG/飞书/Email)
         ↑                                    ↑
         └──────── Scheduler(Cron) ───────────┘
                        + 聊天模式
                        + Web 仪表盘（热重载）
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust, Tokio, Axum |
| 前端 | Vue 3, TypeScript, Tailwind CSS, Vite |
| 数据库 | SQLite (rusqlite) |
| LLM | OpenAI 兼容 API (async-openai) |
| Bot | Teloxide (Telegram) |
| 邮件 | Lettre (SMTP)、pulldown-cmark (Markdown→HTML) |

## 快速开始

### 前置要求

- Rust 1.70+ (含 Cargo)
- Node.js 18+ (前端开发)

### 1. 配置

```bash
cp config.example.toml config.toml
# 编辑 config.toml，填入你的 API keys
```

### 2. 运行

```bash
cargo run --release
```

仪表盘地址：`http://localhost:9090`

### 3. Docker 部署

```bash
# 先构建前端
cd web && npm install && npm run build && cd ..

# 构建并运行
docker build -t courier .
docker run -d \
  --name courier \
  -p 9090:9090 \
  -v ./config.toml:/app/config.toml:ro \
  -v courier_data:/app/data \
  -e TZ=Asia/Shanghai \
  courier
```

## 配置说明

参见 [config.example.toml](./config.example.toml) 了解所有可用选项。

### 支持的 LLM 模型

所有模型通过[火山方舟（Volcengine ARK）](https://www.volcengine.com/docs/82379) OpenAI 兼容 API 接入。可使用模型 ID 或推理接入点 ID（如 `ep-xxxx`）作为 model 值。

| 模型 ID / 接入点 | 名称 | 提供商 |
|---------|------|--------|
| `ep-20260404123347-5lprz` | Doubao Seed 2.0 Lite | 火山方舟（ARK） |
| `ep-20260404125954-zfgwz` | GLM-4.7B | 智谱 AI |
| `ep-20260404125909-wzgdz` | DeepSeek V3.2 | DeepSeek |
| `kimi-k2-thinking-251104` | Kimi K2 Thinking | Moonshot AI |

> **提示：** 在火山方舟平台上，你可以为任何支持的模型创建推理接入点。使用接入点 ID（格式：`ep-xxxxxxxxxxxx-xxxxx`）作为配置中的 `model` 值即可。

### 主要配置项

- **数据源 (Sources)**: 启用/禁用 HN、Reddit、RSS，每个源可独立配置
- **大语言模型 (LLM)**: API 地址、模型选择、自定义提示词
- **推送渠道 (Channels)**: Telegram Bot Token、飞书 Webhook、邮件 SMTP
- **定时任务 (Schedules)**: 多个 Cron 任务，可分别配置数据源和推送渠道

### 时区配置

Cron 表达式按 `[general]` 中的 `timezone` 设置解析：

```toml
[general]
timezone = "Asia/Shanghai"  # Cron 表达式使用此时区
```

未设置时默认为 `"UTC"`。设置为 `Asia/Shanghai` 后，`cron = "0 0 10 * * *"` 表示**北京时间上午 10:00** 触发。

日志时间戳同样遵循 Docker 环境中设置的 `TZ` 环境变量。

> **注意：** `timezone` 配置控制 cron 调度时区。`docker-compose.yml` 中的 `TZ` 环境变量控制日志输出时区。请确保两者保持一致。

### 安全特性

Courier 支持可选的 API 密钥认证。在 `config.toml` 中添加以下配置即可启用：

```toml
[general]
api_key = "your-secret-api-key"
```

启用后，所有 API 请求需携带 `Authorization: Bearer <api_key>` 请求头。若未设置 `api_key` 或为空，仪表盘无需认证即可访问。

敏感信息（Webhook 地址、API 端点）在 API 响应中自动脱敏，仅显示域名（如 `https://example.com/*****`）。

## 仪表盘

Web 仪表盘提供：

- 📊 **概览**: 运行时间、任务数量、成功率
- ⏰ **任务管理**: 编辑计划、重命名任务、切换推送渠道、手动触发执行
- 📋 **执行历史**: 查看历史摘要内容
- ⚙️ **配置管理**: 切换 LLM 模型、调整 max_tokens、配置邮件 SMTP、更新渠道设置——全部支持热重载（无需重启）

### 邮件渠道

Courier 支持通过邮件推送日报摘要，邮件内容自动渲染为精美 HTML：

- 通过 Web 仪表盘配置 SMTP（服务器地址、端口、用户名、密码、发件人、收件人）
- Markdown 内容自动转换为带样式的 HTML 邮件
- 智能发件人地址：只输入显示名称即可自动构造 `"名称 <SMTP用户名>"` 格式
- 密码安全：API 响应中不会返回 SMTP 密码，仅返回 `has_password` 标志
- 热重载：启用/禁用、更新配置无需重启服务

## 部署

详见 [DEPLOY.md](./DEPLOY.md) 了解 Docker 部署详细步骤。

## 开源协议

[MIT](./LICENSE)
