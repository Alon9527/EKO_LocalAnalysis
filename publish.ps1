# EKO 一键发布脚本
# 用法：.\publish.ps1 [-NewVersion 1.0.2]  (不传则自动 patch+1)

param(
    [string]$NewVersion = "",
    [string]$Notes = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ProjectRoot

# 工具路径
$GH = "C:\Users\Administrator\gh-cli\bin\gh.exe"
$CARGO_PATH = "C:\Users\Administrator\.cargo\bin"
$env:PATH = "$CARGO_PATH;$env:PATH"

# 读取当前版本
$conf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$CurrentVersion = $conf.version

# 自动 +patch 如未指定
if (-not $NewVersion) {
    $parts = $CurrentVersion.Split('.')
    $parts[2] = [int]$parts[2] + 1
    $NewVersion = $parts -join '.'
}

Write-Host "==> 当前版本: $CurrentVersion" -ForegroundColor Cyan
Write-Host "==> 发布版本: $NewVersion" -ForegroundColor Green
Write-Host ""

# 检查 gh 登录
& $GH auth status 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ GitHub CLI 未登录" -ForegroundColor Red
    Write-Host "请先运行: & '$GH' auth login" -ForegroundColor Yellow
    exit 1
}

# 询问确认
$confirm = Read-Host "继续发布 v$NewVersion ? (y/N)"
if ($confirm -ne "y") {
    Write-Host "已取消" -ForegroundColor Yellow
    exit 0
}

# 1. 更新版本号
Write-Host ""
Write-Host "==> 步骤 1/5: 更新版本号..." -ForegroundColor Cyan
$conf.version = $NewVersion
$conf | ConvertTo-Json -Depth 100 | Set-Content "src-tauri\tauri.conf.json" -Encoding utf8

$cargoToml = Get-Content "src-tauri\Cargo.toml" -Raw
$cargoToml = $cargoToml -replace "version = `"$CurrentVersion`"", "version = `"$NewVersion`""
Set-Content "src-tauri\Cargo.toml" -Value $cargoToml -Encoding utf8

# 2. 签名构建
Write-Host ""
Write-Host "==> 步骤 2/5: 签名构建（约 1 分钟）..." -ForegroundColor Cyan
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    if (Test-Path "eko.key") {
        $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "eko.key" -Raw).Trim()
    } else {
        Write-Host "❌ TAURI_SIGNING_PRIVATE_KEY 未设置，且未找到 eko.key" -ForegroundColor Red
        exit 1
    }
}

if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
    Write-Host "❌ 请先设置 TAURI_SIGNING_PRIVATE_KEY_PASSWORD 环境变量" -ForegroundColor Red
    exit 1
}
npx tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败" -ForegroundColor Red
    exit 1
}

# 3. 生成 latest.json
Write-Host ""
Write-Host "==> 步骤 3/5: 生成 latest.json..." -ForegroundColor Cyan
$ExePath = "src-tauri\target\release\bundle\nsis\EKO_${NewVersion}_x64-setup.exe"
$SigPath = "${ExePath}.sig"

if (-not (Test-Path $SigPath)) {
    Write-Host "❌ 签名文件未生成: $SigPath" -ForegroundColor Red
    exit 1
}

$Signature = Get-Content $SigPath -Raw
$PubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

if (-not $Notes) {
    $Notes = "EKO v$NewVersion 发布"
}

$LatestJson = @{
    version = $NewVersion
    notes = $Notes
    pub_date = $PubDate
    platforms = @{
        "windows-x86_64" = @{
            signature = $Signature.Trim()
            url = "https://github.com/Alon9527/EKO_LocalAnalysis/releases/download/v$NewVersion/EKO_${NewVersion}_x64-setup.exe"
        }
    }
} | ConvertTo-Json -Depth 10

$LatestJson | Set-Content "latest.json" -Encoding utf8

# 4. 创建 GitHub Release
Write-Host ""
Write-Host "==> 步骤 4/5: 创建 GitHub Release..." -ForegroundColor Cyan
$Tag = "v$NewVersion"

# 检查 tag 是否已存在
$existingRelease = & $GH release view $Tag --repo "Alon9527/EKO_LocalAnalysis" 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "⚠️  Release $Tag 已存在，先删除..." -ForegroundColor Yellow
    & $GH release delete $Tag --repo "Alon9527/EKO_LocalAnalysis" --yes --cleanup-tag
}

& $GH release create $Tag `
    --repo "Alon9527/EKO_LocalAnalysis" `
    --title "$Tag" `
    --notes "$Notes" `
    --latest `
    $ExePath $SigPath "latest.json"

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Release 创建失败" -ForegroundColor Red
    exit 1
}

# 5. 完成
Write-Host ""
Write-Host "==> 步骤 5/5: 完成 ✅" -ForegroundColor Green
Write-Host ""
Write-Host "Release URL: https://github.com/Alon9527/EKO_LocalAnalysis/releases/tag/$Tag" -ForegroundColor Cyan
Write-Host ""
