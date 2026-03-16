# Courier 📬

> A lightweight personal news digest bot — fetch, summarize, and push, all in one go.

[中文文档](./README_CN.md)

## Features

- 📰 **Multi-source Fetching**: Hacker News / Reddit / RSS
- 🤖 **LLM-powered Summarization**: Generate daily digests via OpenAI-compatible APIs
- 📮 **Multi-channel Push**: Telegram / Feishu (Lark) / Email
- ⏰ **Cron Scheduling**: Flexible cron expression configuration
- 💬 **Chat Mode**: Interactive conversations via Telegram bot
- 🖥️ **Web Dashboard**: Vue.js-based management panel with real-time status

## Architecture

\Source(HN/Reddit/RSS) → LLM(Summarize) → Channel(TG/Feishu/Email)
         ↑                                        ↑
         └──────── Scheduler(Cron) ───────────────┘
                        + Chat Mode
                        + Web Dashboard
\
## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust, Tokio, Axum |
| Frontend | Vue 3, TypeScript, Tailwind CSS, Vite |
| Database | SQLite (rusqlite) |
| LLM | OpenAI-compatible API (async-openai) |
| Bot | Teloxide (Telegram) |
| Email | Lettre (SMTP) |

## Quick Start

### Prerequisites

- Rust 1.70+ (with Cargo)
- Node.js 18+ (for frontend development)

### 1. Configure

\\ash
cp config.example.toml config.toml
# Edit config.toml with your API keys and preferences
\
### 2. Run

\\ash
cargo run --release
\
The dashboard will be available at \http://localhost:9090\.

### 3. Docker

\\ash
# Build frontend first
cd web && npm install && npm run build && cd ..

# Build and run
docker build -t courier .
docker run -d \\
  --name courier \\
  -p 9090:9090 \\
  -v ./config.toml:/app/config.toml:ro \\
  -v courier_data:/app/data \\
  -e TZ=Asia/Shanghai \\
  courier
\
## Configuration

See [config.example.toml](./config.example.toml) for all available options.

### Supported LLM Models

| Model ID | Name | Provider |
|----------|------|----------|
| \doubao-seed-2-0-lite-260215\ | Doubao Seed 2.0 Lite (default) | Volcengine (ARK) |
| \glm-4-7-251222\ | GLM-4.7B | Zhipu AI |
| \deepseek-v3-2-251201\ | DeepSeek V3.2 | DeepSeek |
| \kimi-k2-thinking-251104\ | Kimi K2 Thinking | Moonshot AI |

### Key Configuration Sections

- **Sources**: Enable/disable HN, Reddit, RSS with per-source settings
- **LLM**: API endpoint, model selection, custom prompts
- **Channels**: Telegram bot token, Feishu webhook, Email SMTP
- **Schedules**: Multiple cron jobs with different source/channel combinations

## Dashboard

The web dashboard provides:

- 📊 **Overview**: Uptime, task count, success rate
- ⏰ **Task Management**: Edit schedules, rename tasks, trigger manual runs
- 📋 **Execution History**: View past digests with expandable content
- ⚙️ **Configuration**: Switch LLM models, update channel settings

## Deployment

See [DEPLOY.md](./DEPLOY.md) for detailed Docker deployment instructions.

## License

[MIT](./LICENSE)
