# Courier 📬

> 轻量级个人新闻摘要推送 Bot — 抓取、总结、推送，一气呵成。

[English](./README.md)

## 功能

- 📰 **多数据源抓取**: Hacker News / Reddit / RSS
- 🤖 **LLM 智能摘要**: 调用 OpenAI 兼容 API 生成日报
- 📮 **多通道推送**: Telegram / 飞书 / Email
- ⏰ **定时调度**: Cron 表达式灵活配置
- 💬 **聊天模式**: 通过 Telegram Bot 交互式对话
- 🖥️ **Web 仪表盘**: Vue.js 管理面板，实时查看状态
- 🔒 **安全特性**: 可选 API 密钥认证、敏感信息脱敏、输入校验

## 架构

```
Source(HN/Reddit/RSS) → LLM(摘要) → Channel(TG/飞书/Email)
         ↑                                    ↑
         └──────── Scheduler(Cron) ───────────┘
                        + 聊天模式
                        + Web 仪表盘
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust, Tokio, Axum |
| 前端 | Vue 3, TypeScript, Tailwind CSS, Vite |
| 数据库 | SQLite (rusqlite) |
| LLM | OpenAI 兼容 API (async-openai) |
| Bot | Teloxide (Telegram) |
| 邮件 | Lettre (SMTP) |

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

| 模型 ID | 名称 | 提供商 |
|---------|------|--------|
| `doubao-seed-2-0-lite-260215` | Doubao Seed 2.0 Lite（默认） | 火山方舟（ARK） |
| `glm-4-7-251222` | GLM-4.7B | 智谱 AI |
| `deepseek-v3-2-251201` | DeepSeek V3.2 | DeepSeek |
| `kimi-k2-thinking-251104` | Kimi K2 Thinking | Moonshot AI |

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
- ⏰ **任务管理**: 编辑计划、重命名任务、手动触发执行，含输入校验
- 📋 **执行历史**: 查看历史摘要内容
- ⚙️ **配置管理**: 切换 LLM 模型、更新渠道设置，含表单验证

## 部署

详见 [DEPLOY.md](./DEPLOY.md) 了解 Docker 部署详细步骤。

## 开源协议

[MIT](./LICENSE)
