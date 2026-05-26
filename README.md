# EKO 图片反推工具

EKO 是一款本地桌面应用，用于把参考图片反推成可复用的 AI 绘图提示词。它会分析画面主体、场景、光影、镜头与构图，并生成中文和英文 Prompt，适合设计参考、视觉提案、图像生成工作流和素材整理。

## 功能亮点

- 支持单张图片分析和批量图片分析
- 输出完整中文 Prompt 与 English Prompt
- 拆解结构化提示词字段，便于继续编辑和复用
- 保存历史记录、评分、模型信息和缩略图
- 支持收藏、搜索、筛选与导出分析结果
- 支持 Gemini 原生接口和 OpenAI 兼容接口

## 数据与隐私

EKO 是本地应用，用户的 API Key、分析历史、图片路径、图片 URL、生成提示词和缩略图会保存在本机应用数据目录中。项目源码不内置任何 API Key。

如果你准备自行构建或二次开发，请不要提交 `.env`、`eko.key`、构建产物或用户数据目录。

## 开发

```powershell
npm install
npm run tauri -- dev
```

## 发布

发布需要 Tauri updater 签名密钥。私钥不要提交到仓库。

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-signing-key-password"
.\publish.ps1 -NewVersion 1.0.15 -Notes "更新说明"
```

## License

MIT
