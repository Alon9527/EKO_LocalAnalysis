# EKO 图片反推工具

EKO 是一款本地桌面应用，用于把参考图片反推成可复用的 AI 绘图提示词。它会分析画面主体、场景、光影、镜头与构图，并生成中文和英文 Prompt，适合设计参考、视觉提案、图像生成工作流和素材整理。

## 功能亮点

- 支持单张图片分析和批量图片分析
- 输出完整中文 Prompt 与 English Prompt
- 拆解结构化提示词字段，便于继续编辑和复用
- 保存历史记录、评分、模型信息和缩略图
- 支持收藏、搜索、筛选与导出分析结果
- 支持 Gemini 原生接口和 OpenAI 兼容接口
- 支持 Chrome 浏览器插件，将网页图片发送到本地软件分析并自动保存到历史记录

## 浏览器插件

插件源码位于 [`browser-extension`](browser-extension) 目录，可在 Chrome 中以“加载已解压的扩展程序”的方式安装。

1. 启动 EKO 桌面软件。
2. 打开 Chrome 的 `chrome://extensions/`，开启“开发者模式”。
3. 点击“加载已解压的扩展程序”，选择仓库中的 `browser-extension` 文件夹。
4. 在网页图片上悬停并点击“反推”，或右键图片选择“用 EKO 反推这张图片”。

插件只通过本机地址 `http://127.0.0.1:17621` 与桌面软件通信，不保存或暴露 API Key。分析结果会写入 EKO 的历史记录。

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
.\publish.ps1 -NewVersion 1.1.1 -Notes "更新说明"
```

不指定 `-NewVersion` 时，发布脚本会自动递增版本号。patch 小于 9 时递增 patch；patch 大于等于 9 时进入下一个 minor，例如 `1.1.9` 的下一版是 `1.2.0`。

## License

MIT
