# Courier 📬

> A lightweight personal news digest bot — fetch, summarize, and push, all in one go.

[中文文档](./README_CN.md)

## Features

- 📰 **Multi-source Fetching**: Hacker News / Reddit / RSS (with RSSHub support)
- 🤖 **LLM-powered Summarization**: Generate daily digests via OpenAI-compatible APIs
- 📮 **Multi-channel Push**: Telegram / Feishu (Lark) / Email (SMTP with Markdown→HTML rendering)
- ⏰ **Cron Scheduling**: Flexible cron expression configuration
- 💬 **Chat Mode**: Interactive conversations via Telegram bot
- 🖥️ **Web Dashboard**: Vue.js-based management panel with real-time status
- 🔄 **Hot-reload Config**: Update email, channel, and LLM settings without restarting
- 🔒 **Security**: Optional API key authentication, sensitive data masking, input validation

## Architecture

```
Source(HN/Reddit/RSS) → LLM(Summarize) → Channel(TG/Feishu/Email)
         ↑                                        ↑
         └──────── Scheduler(Cron) ───────────────┘
                        + Chat Mode
                        + Web Dashboard (Hot-reload)
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust, Tokio, Axum |
| Frontend | Vue 3, TypeScript, Tailwind CSS, Vite, ECharts |
| Database | SQLite (rusqlite) |
| LLM | OpenAI-compatible API (async-openai) |
| Bot | Teloxide (Telegram) |
| Email | Lettre (SMTP), pulldown-cmark (Markdown→HTML) |

## Quick Start

### Prerequisites

- Rust 1.70+ (with Cargo)
- Node.js 18+ (for frontend development)

### 1. Configure

```bash
cp config.example.toml config.toml
cp deploy/docker-compose.example.yml deploy/docker-compose.yml
cp deploy/docker-compose.dev.example.yml deploy/docker-compose.dev.yml
# Edit config.toml with your API keys and preferences
# Edit docker-compose files to add tokens (e.g., GITHUB_ACCESS_TOKEN)
```

### 2. Run

```bash
cargo run --release
```

The dashboard will be available at `http://localhost:9090`.

### 3. Docker (Production)

```bash
# Build frontend first
cd web && npm install && npm run build && cd ..

# Build and run with docker-compose (includes RSSHub)
docker compose -f deploy/docker-compose.yml up -d
```

### 4. Local Development

For rapid iteration, run only RSSHub in Docker while running the backend and frontend locally:

```bash
# One-click startup (Linux/macOS)
./scripts/dev.sh

# One-click startup (Windows PowerShell)
.\scripts\dev.ps1
```

Or start each service manually:

```bash
# Start RSSHub only
docker compose -f deploy/docker-compose.dev.yml up -d

# Update config.toml: change "rsshub:1200" to "localhost:1200" in RSS feed URLs

# Run backend
cargo run -- config.toml

# Run frontend (in a separate terminal)
cd web && npm install && npm run dev
```

> See [DEPLOY.md](./DEPLOY.md) for full deployment and development instructions.

## Configuration

See [config.example.toml](./config.example.toml) for all available options.

### Supported LLM Models

All models are accessed via [Volcengine ARK](https://www.volcengine.com/docs/82379) OpenAI-compatible API. You can use either model IDs or inference endpoint IDs (e.g., `ep-xxxx`) as the model value.

| Model ID / Endpoint | Name | Provider |
|----------|------|----------|
| `ep-20260404123347-5lprz` | Doubao Seed 2.0 Lite | Volcengine (ARK) |
| `ep-20260404125954-zfgwz` | GLM-4.7B | Zhipu AI |
| `ep-20260404125909-wzgdz` | DeepSeek V3.2 | DeepSeek |
| `kimi-k2-thinking-251104` | Kimi K2 Thinking | Moonshot AI |

> **Tip:** On Volcengine ARK, you can create inference endpoints for any supported model. Use the endpoint ID (format: `ep-xxxxxxxxxxxx-xxxxx`) as the `model` value in your config.

### Key Configuration Sections

- **Sources**: Enable/disable HN, Reddit, RSS with per-source settings
- **LLM**: API endpoint, model selection, custom prompts
- **Channels**: Telegram bot token, Feishu webhook, Email SMTP
- **Schedules**: Multiple cron jobs with different source/channel combinations

### Timezone

Cron expressions are interpreted according to the `timezone` setting in `[general]`:

```toml
[general]
timezone = "Asia/Shanghai"  # Cron expressions use this timezone
```

If not specified, defaults to `"UTC"`. With the above setting, `cron = "0 0 10 * * *"` triggers at **10:00 AM Beijing time**.

Log timestamps also follow the `TZ` environment variable set in your Docker environment.

> **Note:** The `timezone` config controls cron scheduling. The `TZ` environment variable (set in `deploy/docker-compose.yml`) controls log output timestamps. Make sure both are consistent.

### Security

Courier supports optional API key authentication for the dashboard. Add the following to your `config.toml`:

```toml
[general]
api_key = "your-secret-api-key"
```

When configured, all API requests must include a `Authorization: Bearer <api_key>` header. If `api_key` is not set or empty, the dashboard is accessible without authentication.

Sensitive information (webhook URLs, API endpoints) is automatically masked in API responses, showing only the domain (e.g., `https://example.com/*****`).

## Dashboard

The web dashboard provides:

- 📊 **Overview**: Uptime, task count, success rate, execution count & duration charts (ECharts)
- ⏰ **Task Management**: Edit schedules, rename tasks, switch push channels, trigger manual runs
- 📋 **Execution History**: View past digests with expandable content
- ⚙️ **Configuration**: Switch LLM models, adjust max tokens, configure email SMTP, update channel settings — all with hot-reload (no restart needed)

### Email Channel

Courier supports sending digests via email with rich HTML rendering:

- SMTP configuration via web dashboard (host, port, username, password, from, to)
- Markdown content is automatically converted to styled HTML emails
- Smart "from" address: provide just a display name and it auto-constructs `"Name <smtp_username>"`
- Password security: never exposed in API responses, only `has_password` flag is returned
- Hot-reload: enable/disable and update settings without restarting

## Deployment

See [DEPLOY.md](./DEPLOY.md) for detailed Docker deployment instructions.

## License

[MIT](./LICENSE)
