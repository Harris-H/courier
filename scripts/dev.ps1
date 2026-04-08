# Courier 本地开发一键启动脚本 (Windows PowerShell)
# 启动 RSSHub (Docker) + Rust 后端 + Vue 前端 dev server

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

# Fix console encoding for emoji display
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Prevent PowerShell from treating native command stderr as errors
$ErrorActionPreference = "Continue"
if ($null -ne (Get-Variable PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue)) {
    $PSNativeCommandUseErrorActionPreference = $false
}

# Helper: check if a TCP port is open (avoids proxy/HTTP issues)
function Test-Port {
    param([int]$Port, [int]$Timeout = 2000)
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        # Use 127.0.0.1 (IPv4) since servers typically bind to 0.0.0.0
        $result = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        $success = $result.AsyncWaitHandle.WaitOne($Timeout)
        if ($success) { $client.EndConnect($result) }
        $client.Close()
        return $success
    } catch {
        return $false
    }
}

Write-Host "`n📬 Courier Local Dev Startup" -ForegroundColor Cyan
Write-Host "================================"

# 1. Start RSSHub
Write-Host "`n1/3 🐳 Starting RSSHub..." -ForegroundColor Green
cmd.exe /c "docker compose -f deploy\docker-compose.dev.yml up -d 2>&1"
Write-Host "  RSSHub: http://localhost:1200"

Write-Host -NoNewline "  Waiting for RSSHub"
$rsshubReady = $false
for ($i = 0; $i -lt 30; $i++) {
    if (Test-Port -Port 1200) {
        Write-Host " ready!" -ForegroundColor Green
        $rsshubReady = $true
        break
    }
    Write-Host -NoNewline "."
    Start-Sleep -Seconds 1
}
if (-not $rsshubReady) {
    Write-Host " (timeout, continuing anyway)" -ForegroundColor Yellow
}

# 2. Start backend (auto-generate dev config with localhost URLs)
Write-Host "`n2/3 🦀 Starting backend..." -ForegroundColor Green
$configPath = Join-Path $ProjectRoot "config.toml"
$devConfigPath = Join-Path $ProjectRoot "config.dev.toml"
$configContent = [System.IO.File]::ReadAllText($configPath, [System.Text.Encoding]::UTF8)
$devConfig = $configContent -replace 'rsshub:1200', 'localhost:1200'
[System.IO.File]::WriteAllText($devConfigPath, $devConfig, [System.Text.UTF8Encoding]::new($false))
Write-Host "  Generated config.dev.toml (rsshub:1200 → localhost:1200)"
$backendJob = Start-Process -FilePath "cargo" -ArgumentList "run", "--", "config.dev.toml" -PassThru -NoNewWindow
Write-Host "  Backend PID: $($backendJob.Id)"
Write-Host "  Dashboard: http://localhost:9090"

Write-Host -NoNewline "  Waiting for backend (compiling may take a while)"
$backendReady = $false
for ($i = 0; $i -lt 120; $i++) {
    if ($backendJob.HasExited) {
        Write-Host " process exited unexpectedly!" -ForegroundColor Red
        break
    }
    if (Test-Port -Port 9090) {
        Write-Host " ready!" -ForegroundColor Green
        $backendReady = $true
        break
    }
    Write-Host -NoNewline "."
    Start-Sleep -Seconds 2
}
if (-not $backendReady -and -not $backendJob.HasExited) {
    Write-Host " (timeout, continuing anyway)" -ForegroundColor Yellow
}

# 3. Start frontend dev server
Write-Host "`n3/3 🖥️  Starting frontend dev server..." -ForegroundColor Green
Push-Location web
npm install --silent 2>$null
$frontendJob = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "npm run dev" -PassThru -NoNewWindow
Pop-Location
Write-Host "  Frontend PID: $($frontendJob.Id)"
Write-Host "  Dev server: http://localhost:5173"

Write-Host "`n================================" -ForegroundColor Cyan
Write-Host "🚀 All services running!" -ForegroundColor Green
Write-Host "  RSSHub:    http://localhost:1200"
Write-Host "  Backend:   http://localhost:9090"
Write-Host "  Frontend:  http://localhost:5173"
Write-Host "Press Ctrl+C to stop all services.`n" -ForegroundColor Yellow

try {
    # Wait for backend process to exit
    $backendJob.WaitForExit()
} finally {
    Write-Host "`n🛑 Shutting down..." -ForegroundColor Yellow
    # Kill backend process tree
    if (!$backendJob.HasExited) {
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $($backendJob.Id)" -ErrorAction SilentlyContinue |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
        Stop-Process -Id $backendJob.Id -Force -ErrorAction SilentlyContinue
        Write-Host "  Stopped backend"
    }
    # Kill frontend process tree
    if (!$frontendJob.HasExited) {
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $($frontendJob.Id)" -ErrorAction SilentlyContinue |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
        Stop-Process -Id $frontendJob.Id -Force -ErrorAction SilentlyContinue
        Write-Host "  Stopped frontend"
    }
    cmd.exe /c "docker compose -f deploy\docker-compose.dev.yml down 2>&1" | Out-Null
    Write-Host "  Stopped RSSHub"
    Remove-Item "config.dev.toml" -Force -ErrorAction SilentlyContinue
    Write-Host "✅ All services stopped." -ForegroundColor Green
}
