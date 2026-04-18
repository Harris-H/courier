# Courier 📬

> 轻量级个人新闻摘要推送 Bot — 抓取、总结、推送，一气呵成。

[English](./README.md)

## 功能

- 📰 **多数据源抓取**: Hacker News / Reddit / RSS（支持 RSSHub）
- 🤖 **LLM 智能摘要**: 调用 OpenAI 兼容 API 生成日报
- 📮 **多通道推送**: Telegram / 飞书 / Email（SMTP，Markdown→HTML 富文本渲染）
- 📊 **智能重排序**: 启发式评分 = 互动热度(50%) + 新鲜度(35%) + 源质量(15%)，附带热度标签（🔥热门 / 📈上升 / 📰普通）
- 🔗 **跨源聚类去重**: 通过 URL 匹配 + Jaccard 标题相似度合并同一新闻，附带多源验证标签（🔗 双源验证 / 🔗 N源验证）
- ⏰ **定时调度**: Cron 表达式灵活配置
- 💬 **聊天模式**: 通过 Telegram Bot 交互式对话
- 🖥️ **Web 仪表盘**: Vue.js 管理面板，实时查看状态
- 🔄 **热重载配置**: 邮件、频道、LLM 配置修改无需重启即刻生效
- 🔒 **安全特性**: 可选 API 密钥认证、敏感信息脱敏、输入校验

## 架构

```
Source(HN/Reddit/RSS) → Rerank(评分) → Cluster(去重) → LLM(摘要) → Channel(TG/飞书/Email)
         ↑                                                                    ↑
         └──────────────────── Scheduler(Cron) ─────────────────────────────┘
                                    + 聊天模式
                                    + Web 仪表盘（热重载）
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust, Tokio, Axum |
| 前端 | Vue 3, TypeScript, Tailwind CSS, Vite, ECharts |
| 数据库 | SQLite (rusqlite) |
| LLM | OpenAI 兼容 API (async-openai) |
| 重排序 | 启发式评分（互动热度 × 新鲜度 × 源质量） |
| 聚类去重 | Jaccard 相似度 + URL 规范化匹配 |
| Bot | Teloxide (Telegram) |
| 邮件 | Lettre (SMTP)、pulldown-cmark (Markdown→HTML) |

## 快速开始

### 前置要求

- Rust 1.70+ (含 Cargo)
- Node.js 18+ (前端开发)

### 1. 配置

```bash
cp config.example.toml config.toml
cp deploy/docker-compose.example.yml deploy/docker-compose.yml
cp deploy/docker-compose.dev.example.yml deploy/docker-compose.dev.yml
# 编辑 config.toml，填入你的 API keys
# 编辑 docker-compose 文件，添加 token（如 GITHUB_ACCESS_TOKEN）
```

### 2. 运行

```bash
cargo run --release
```

仪表盘地址：`http://localhost:9090`

### 3. Docker 部署（生产环境）

```bash
# 先构建前端
cd web && npm install && npm run build && cd ..

# 使用 docker-compose 构建并运行（包含 RSSHub）
docker compose -f deploy/docker-compose.yml up -d
```

### 4. 本地开发

本地开发时，仅运行 RSSHub 容器，前后端直接在本地启动，方便快速迭代：

```bash
# 一键启动（Linux/macOS）
./scripts/dev.sh

# 一键启动（Windows PowerShell）
.\scripts\dev.ps1
```

或手动分别启动：

```bash
# 仅启动 RSSHub
docker compose -f deploy/docker-compose.dev.yml up -d

# 修改 config.toml：将 RSS feed URL 中的 "rsshub:1200" 改为 "localhost:1200"

# 启动后端
cargo run -- config.toml

# 启动前端（另开终端）
cd web && npm install && npm run dev
```

> 详见 [DEPLOY.md](./DEPLOY.md) 了解完整的部署和开发说明。

## 处理流水线

每个日报任务经过 6 阶段流水线处理：

1. **抓取 (Fetch)** — 并发拉取所有配置源的文章（支持每源独立重试）
2. **重排序 (Rerank)** — 使用 `HeuristicReranker` 对每篇文章评分：
   - **互动热度** (50%)：标准化的 score + 评论数（对数缩放）
   - **新鲜度** (35%)：指数衰减，半衰期 = 12 小时
   - **源质量** (15%)：编辑质量权重（HN 0.85 > Reddit 0.65 > RSS 0.50）
   - 分配热度标签：🔥热门 (≥0.7) / 📈上升 (≥0.4) / 📰普通
3. **聚类去重 (Cluster)** — 跨源合并同一新闻：
   - URL 规范化匹配（最强信号）
   - 标题词组 Jaccard 相似度（阈值：0.45）
   - 添加跨源验证标签：🔗 双源验证 / 🔗 N源验证
4. **格式化 (Format)** — 构建带热度标签 + 来源标注的结构化内容供 LLM 使用
5. **摘要生成 (Summarize)** — 通过 LLM 生成日报摘要（失败自动重试）
6. **推送 (Push)** — 并发发送至所有配置的推送渠道

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

> **注意：** `timezone` 配置控制 cron 调度时区。`deploy/docker-compose.yml` 中的 `TZ` 环境变量控制日志输出时区。请确保两者保持一致。

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

- 📊 **概览**: 运行时间、任务数量、成功率、执行次数与耗时图表（ECharts）
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
