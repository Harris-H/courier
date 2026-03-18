# Courier 部署指南

## 方式一：CI/CD 自动部署（推荐）

项目已配置 GitHub Actions，`git push` 到 `master` 分支后会自动构建 Docker 镜像并推送到阿里云容器镜像服务。

### 前置配置

在 GitHub 仓库的 **Settings → Secrets and variables → Actions** 中添加：

| Secret 名称 | 说明 |
|-------------|------|
| `ACR_USERNAME` | 阿里云 CR 登录用户名 |
| `ACR_PASSWORD` | 阿里云 CR 登录密码 |

### 服务器首次部署

```bash
# 1. 创建部署目录
mkdir -p /opt/courier && cd /opt/courier

# 2. 上传 config.toml 和 docker-compose.yml
#    （或从仓库获取 docker-compose.yml）

# 3. 登录阿里云镜像仓库
docker login registry.cn-hangzhou.aliyuncs.com

# 4. 拉取并启动
docker compose pull
docker compose up -d
```

### 后续更新

每次 `git push` 后，GitHub Actions 会自动构建新镜像。服务器上只需：

```bash
cd /opt/courier
docker compose pull
docker compose up -d
```

> 💡 可配合 Watchtower 实现全自动更新：镜像推送后自动拉取重启。

### 查看日志

```bash
docker compose logs -f
```

---

## 方式二：手动构建部署

### 1. 本地构建

```bash
# 构建前端
cd web && npm install && npm run build && cd ..

# 构建 Docker 镜像
docker build -t courier:latest .

# 导出镜像
docker save courier:latest -o courier.tar
```

### 2. 传输到服务器

```bash
scp courier.tar config.toml docker-compose.yml user@your-server:/opt/courier/
```

### 3. 服务器端部署

```bash
cd /opt/courier
docker load -i courier.tar
docker compose up -d
```

---

## 常用管理命令

```bash
# 查看状态
docker compose ps

# 查看日志
docker compose logs -f

# 重启
docker compose restart

# 停止
docker compose down

# 更新（CI/CD 方式）
docker compose pull && docker compose up -d
```

## 注意事项

- `config.toml` 中包含 API Key，请确保文件权限安全（`chmod 600 config.toml`）
- 数据（SQLite + 日志）存储在 Docker volume `courier_data` 中，容器重建后保留
- 默认端口 9090，可在 `docker-compose.yml` 中修改映射
- 时区设置：`docker-compose.yml` 中 `TZ=Asia/Shanghai` 控制日志时区；`config.toml` 中 `timezone = "Asia/Shanghai"` 控制 cron 调度时区，两者需保持一致
- 如果服务器在火山引擎 VPC 内，可将 docker-compose.yml 中的 registry 改为 `registry-vpc.cn-hangzhou.aliyuncs.com` 加速拉取

---

## RSSHub 集成（可选）

`docker-compose.yml` 已包含 [RSSHub](https://github.com/DIYgod/RSSHub) 服务，可作为 Courier 的通用 RSS 数据源。

### 启用 RSSHub

RSSHub 会随 `docker compose up -d` 自动启动，监听 `1200` 端口。

### 在 Courier 中使用

在 `config.toml` 的 `[sources.rss]` 中添加 RSSHub 路由：

```toml
[sources.rss]
enabled = true
feeds = [
    { name = "GitHub Trending", url = "http://rsshub:1200/github/trending/daily/any/en" },
    { name = "Product Hunt", url = "http://rsshub:1200/producthunt/today" },
    { name = "V2EX Hot", url = "http://rsshub:1200/v2ex/topics/hot" },
]
```

> 💡 Docker 内网通信使用 `http://rsshub:1200`（服务名），无需暴露端口。
> 📖 完整路由列表：https://docs.rsshub.app/zh/routes/

### 验证 RSSHub

```bash
# 在服务器上测试 RSSHub 是否正常
curl http://localhost:1200/github/trending/daily/any/en
```
