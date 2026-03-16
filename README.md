# Courier 📬

> 轻量级个人新闻摘要推送 Bot — 抓取、总结、推送，一气呵成。

## 功能

- 📰 **多数据源抓取**: Hacker News / Reddit / RSS
- 🤖 **LLM 智能摘要**: 调用 OpenAI 兼容 API 生成日报
- 📮 **多通道推送**: Telegram / 飞书 / Email
- ⏰ **定时调度**: Cron 表达式灵活配置
- 💬 **聊天模式**: 交互式对话，支持指令系统

## 架构

```
Source(HN/Reddit/RSS) → LLM(摘要) → Channel(TG/飞书/Email)
         ↑                                    ↑
         └──────── Scheduler(Cron) ───────────┘
                       + Chat Mode
```

## 快速开始

### 1. 配置

```bash
cp config.example.toml config.toml
# 编辑 config.toml，填入你的 API keys
```

### 2. 运行

```bash
cargo run --release
```

### 3. Docker

```bash
docker build -t courier .
docker run -d -v ./config.toml:/app/config.toml courier
```

## 配置说明

参见 [config.example.toml](./config.example.toml)

## License

MIT
