# 图片反推工具

AI 驱动的本地图片反推 Prompt 桌面工具，基于 Vue 和 Tauri 构建。

## 安全说明

- 本项目不会在源码中内置 API Key。用户在应用设置里填写的 API Key 会保存在本机应用数据目录中。
- 分析历史、图片路径、图片 URL、生成的提示词和缩略图会保存在本机，用于历史记录和收藏功能。
- 发布签名私钥不应提交到仓库。发布时请通过环境变量提供 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，私钥可通过 `TAURI_SIGNING_PRIVATE_KEY` 提供，或仅在本地放置被 `.gitignore` 忽略的 `eko.key`。
- 不要提交 `.env`、`eko.key`、构建产物或用户数据目录。

## 发布

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-signing-key-password"
.\publish.ps1 -NewVersion 1.0.14 -Notes "更新说明"
```
