#!/usr/bin/env bash
# Courier 本地开发一键启动脚本
# 启动 RSSHub (Docker) + Rust 后端 + Vue 前端 dev server
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Bypass proxy for localhost health checks
export no_proxy="localhost,127.0.0.1"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

cleanup() {
    echo -e "\n${YELLOW}🛑 Shutting down...${NC}"
    # Kill background processes
    [ -n "$BACKEND_PID" ] && kill "$BACKEND_PID" 2>/dev/null && echo "  Stopped backend (PID $BACKEND_PID)"
    [ -n "$FRONTEND_PID" ] && kill "$FRONTEND_PID" 2>/dev/null && echo "  Stopped frontend (PID $FRONTEND_PID)"
    # Stop RSSHub container
    docker compose -f deploy/docker-compose.dev.yml down 2>/dev/null && echo "  Stopped RSSHub"
    rm -f config.dev.toml
    echo -e "${GREEN}✅ All services stopped.${NC}"
    exit 0
}
trap cleanup SIGINT SIGTERM

echo -e "${BLUE}📬 Courier Local Dev Startup${NC}"
echo "================================"

# 1. Start RSSHub
echo -e "\n${GREEN}1/3 🐳 Starting RSSHub...${NC}"
docker compose -f deploy/docker-compose.dev.yml up -d
echo "  RSSHub: http://localhost:1200"

# Wait for RSSHub to be ready
echo -n "  Waiting for RSSHub"
RSSHUB_READY=false
for i in $(seq 1 30); do
    if curl -sf --noproxy localhost http://localhost:1200 > /dev/null 2>&1; then
        echo -e " ${GREEN}ready!${NC}"
        RSSHUB_READY=true
        break
    fi
    echo -n "."
    sleep 1
done
if [ "$RSSHUB_READY" = false ]; then
    echo -e " ${YELLOW}(timeout, continuing anyway)${NC}"
fi

# 2. Start backend (auto-generate dev config with localhost URLs)
echo -e "\n${GREEN}2/3 🦀 Starting backend...${NC}"
sed 's/rsshub:1200/localhost:1200/g' config.toml > config.dev.toml
echo "  Generated config.dev.toml (rsshub:1200 → localhost:1200)"
cargo run -- config.dev.toml &
BACKEND_PID=$!
echo "  Backend PID: $BACKEND_PID"
echo "  Dashboard: http://localhost:9090"

# Wait for backend to be ready (compile may take a while)
echo -n "  Waiting for backend (compiling may take a while)"
BACKEND_READY=false
for i in $(seq 1 120); do
    if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
        echo -e " ${YELLOW}process exited unexpectedly!${NC}"
        break
    fi
    if curl -sf --noproxy localhost http://localhost:9090/api/status > /dev/null 2>&1; then
        echo -e " ${GREEN}ready!${NC}"
        BACKEND_READY=true
        break
    fi
    echo -n "."
    sleep 2
done
if [ "$BACKEND_READY" = false ] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    echo -e " ${YELLOW}(timeout, continuing anyway)${NC}"
fi

# 3. Start frontend dev server
echo -e "\n${GREEN}3/3 🖥️  Starting frontend dev server...${NC}"
cd web
npm install --silent 2>/dev/null
npm run dev &
FRONTEND_PID=$!
cd ..
echo "  Frontend PID: $FRONTEND_PID"
echo "  Dev server: http://localhost:5173"

echo -e "\n${BLUE}================================${NC}"
echo -e "${GREEN}🚀 All services running!${NC}"
echo -e "  RSSHub:    http://localhost:1200"
echo -e "  Backend:   http://localhost:9090"
echo -e "  Frontend:  http://localhost:5173"
echo -e "${YELLOW}Press Ctrl+C to stop all services.${NC}"

# Wait for any background process to exit
wait
