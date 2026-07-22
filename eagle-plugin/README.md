# EKO Eagle 图片反推插件

这是 Eagle Inspector 插件。选中 Eagle 里的图片后，右侧检查器会显示 `EKO 图片反推` 面板，点击 `反推这张图` 即可调用本机 EKO 软件分析图片，并把结果保存到 EKO 历史记录。

## 使用前准备

1. 安装并打开 EKO 本地软件。
2. 在 EKO 设置中心配置可用的 API Key 和模型。
3. 确认 EKO 版本支持本地桥接服务：`http://127.0.0.1:17621`。

## 安装方式

1. 打开 Eagle。
2. 进入插件管理页面。
3. 选择导入本地插件文件夹。
4. 选择本目录：`eagle-plugin`。
5. 在 Eagle 中选中一张 JPG、PNG、WebP 等图片，查看右侧 Inspector 面板。

## 功能

- 读取 Eagle 当前选中的本地图片。
- 调用 EKO 本地桥接接口分析图片。
- 分析完成后保存到 EKO 历史记录。
- 支持 GPT Image / Nano Banana Prompt 切换。
- 支持中文 / English Prompt 切换和复制。

## 常见问题

### 提示无法连接 EKO

请先打开 EKO 本地软件，并确认设置中心 API 可用。

### 分析失败

通常是 EKO 设置里的 API Key、模型名或网络配置不可用。先在 EKO 软件里用单图分析测试一张图片。