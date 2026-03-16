# Courier 部署指南（Docker 方式）

## 一、本地构建

### 1. 构建前端
```bash
cd web && npm run build
```

### 2. 构建 Docker 镜像
```bash
cd E:\Github Project\courier
docker build -t courier:latest .
```

### 3. 导出镜像为压缩包
```bash
docker save courier:latest | gzip > courier-image.tar.gz
```

## 二、传输到服务器

```bash
scp courier-image.tar.gz user@your-server:/home/user/
scp config.toml user@your-server:/home/user/courier/
```

## 三、服务器端部署

### 1. 加载镜像
```bash
gunzip -c courier-image.tar.gz | docker load
```

### 2. 创建配置目录
```bash
mkdir -p /home/user/courier
# 将 config.toml 放入此目录（注意修改其中的 API key 等敏感信息）
```

### 3. 使用 docker-compose 启动（推荐）

将 `docker-compose.yml` 也复制到服务器的 `/home/user/courier/` 目录，然后：

```bash
cd /home/user/courier
docker-compose up -d
```

### 或直接 docker run

```bash
docker run -d \
  --name courier \
  --restart unless-stopped \
  -p 9090:9090 \
  -v /home/user/courier/config.toml:/app/config.toml:ro \
  -v courier_data:/app/data \
  -e TZ=Asia/Shanghai \
  courier:latest
```

## 四、验证

```bash
# 检查容器状态
docker ps | grep courier

# 查看日志
docker exec courier cat /app/data/logs/courier.log.*

# 访问面板
curl http://localhost:9090/api/status
```

浏览器访问 `http://your-server-ip:9090` 即可看到前端面板。

## 五、常用管理命令

```bash
# 查看日志
docker logs courier

# 重启
docker restart courier

# 停止
docker stop courier

# 更新部署（重复步骤一~三）
docker stop courier && docker rm courier
# 然后重新 docker load 和 docker run
```

## 注意事项

- `config.toml` 中包含 API Key，请确保文件权限安全（`chmod 600 config.toml`）
- 数据（SQLite 数据库 + 日志）存储在 Docker volume `courier_data` 中，会在容器重建后保留
- 默认端口 9090，可通过 `-p` 参数修改映射
- 时区设置为 `Asia/Shanghai`，cron 将按北京时间执行
