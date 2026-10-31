# EKO｜获客Agent｜验证记录｜1101

验证日期：2026-11-01（Asia/Shanghai）；公开页面实际抓取时间约为 2026-10-31 17:39–17:40 UTC，JSON 中保留原始带时区时间。

## 已执行验证

- Python 3.12.14：`python -m unittest discover -s tests -v`，43 项通过；无网络和 API Key 依赖。
- `python -m compileall -q eko_prospect` 通过。
- `pip install --no-deps --no-build-isolation --target <隔离目录> .` 构建 wheel、安装成功；在项目目录外运行安装后的 `python -m eko_prospect --help` 成功。
- `git diff --check` 通过。
- 本地 HTTP 复核测试：真实 GET、带 CSRF 的 POST 写入与重定向、缺失 CSRF 拒绝、错误 Host 拒绝、HTML 注入转义、审批版本令牌存在。
- Playwright 渲染烟测未能运行：环境缺少 Chromium 可执行文件。没有据此宣称完成浏览器视觉验收；HTTP 功能测试已执行。
- 原 Vue/Tauri 图片程序没有业务代码修改，也未运行其平台编译。根 README 仅增加独立模块入口。

## 真实公开来源

| 企业 | 本轮重新读取的官方来源 | 保守结论 |
|---|---|---|
| KaTom Restaurant Supply | [帮助/供应商联系](https://www.katom.com/help.html)、[垃圾桶品类](https://www.katom.com/cat/trash-cans-accessories.html) | 公开供应商入口与相关品类；不声称有 EKO 订单、近期特定采购需求或已确认现有合作关系 |
| Global Industrial | [供应商申请](https://www.globalindustrial.com/suppliers)、[不锈钢开放式垃圾桶](https://www.globalindustrial.com/p/global-industrial-153-stainless-steel-round-open-top-receptacle-16-gallon) | 有供应商申请和相关在售产品；未找到可验证的采购买手邮箱则保持空白 |
| Tractor Supply | [Nampa 配送中心公告](https://corporate.tractorsupply.com/newsroom/news-releases/news-releases-details/2026/Tractor-Supply-Celebrates-Grand-Opening-of-11th-Distribution-Center-in-Nampa-Idaho/default.aspx)、[供应商资源中心](https://www.tractorsupply.com/tsc/cms/vendor-resource-center) | 2026-09-23 渠道设施扩张，不能据此认定垃圾桶采购意向；本样本尚缺直接垃圾桶品类证据 |

六个页面均经过受控代理、遵循 robots 成功抓取。默认直连在本环境遇到 DNS 不可用，失败被正常记录，没有伪装为无客户信号。

`examples/real_public_sample.json` 只保留每个来源不超过25词的公开短摘录。`method=manual_excerpt` 表明留存的是经过选取的片段，SHA256 对应片段，不冒充原始网页哈希。网页全文和 HTTP 实测数据库保存在被忽略的工作目录，不进入公开仓库。

历史摘录 E2E 测试在临时 DB 中模拟人工动作：导入 → 每项证据复核 → 保守评分 → 暂缓客户 → CSV。模拟分数为 KaTom 6、Global Industrial 4、Tractor Supply 5；Tractor 扩张只得1个近期信号分，`verified_procurement=false`。这些是**测试中的规则计算结果**，不代表 Alon 已审核，交付样本的实际状态全部为 pending，合格客户数0/20。

首次在线 `run`（3家公司，每家最多2页）成功采集5页，另有1个供应商 PDF 返回404；两条带句号的邮箱摘录因标点空格问题未被接纳。这些失败完整落库，命令退出2；已修复标点保真，并让自动跟随跳过已知二进制文件。显式请求不支持的文件仍记录失败，供人工摘录。

修正后的最终在线 `run` 使用3家公司、每家1个公开入口页：3页成功、0失败、12条待审核候选证据，CSV成功生成，命令退出0。合格客户仍为0/20；人工审批没有被自动跳过。

## 独立代码审查与回归

独立只读审查检查了初版提交 `268401f`，先独立运行31项测试，再复现以下问题。所有修正均添加了先失败、后通过的回归测试：

| 问题 | 修正 |
|---|---|
| 代理模式受 NO_PROXY 影响而直连 | 强制对应协议代理，禁止绕过，缺失协议代理时拒绝 |
| 旧网页表单批准新数据版本 | expected_revision 绑定，事务内比较并拒绝过期提交 |
| 公司子域名被算成独立采购来源 | 排除第一方来源；外部来源还需人工明确核实出版者独立性 |
| supplier_access 的任意 value 可关联客服邮箱 | 完整邮箱必须实际出现在该供应商用途的原文摘录中 |
| LinkedIn 前缀匹配生成错误联系人 | 提取完整 URL，规范化后精确匹配 |
| 恶意长词导致邮箱正则长时间回溯 | 线性分词，对长度<=254的候选执行完整匹配 |
| 未知网页编码中断整批 | 编码异常记录为来源失败，后续页面继续 |
| 句号附近自动插入空格破坏原文 | 按原文字符边界截取，保留标点；真实采集已暴露该问题，按影响升级修复 |

增加了集团相同但邮箱不同的重复草稿拦截、实际 HTML 转义及正常网页审核 POST、schema v1→v2 数据保留验证。没有合并到 main，没有发送邮件。

## 实现决策及实际限制

- 使用独立目录交付，因为可用 GitHub 工具有分支/PR 权限而没有新建仓库入口；当前独立包可直接迁出。保留 Python 是为降低运行与模型依赖，未引入原图片工具的 Rust/Tauri 构建成本。
- 基于规则的有限研究流水线，不是可以自行证明事实的模型。搜索/提取结果都只是候选；事件与人物角色的语义、价格条件、EKO既有合作关系必须人工核实。
- 代理模式明确依赖所配置出口网关的私网目标阻断能力；无法从客户端证明网关内部策略。默认直连采用公共 IP 校验与地址固定。
- Brave 适配器按官方接口实现，但未配置真实付费密钥，未宣称完成生产 API 验证。可用 CSV 和公开官网完成整条流水线。
- 没有邮箱投递探测、付费联系人库、验证码绕过或 LinkedIn 自动爬取。没有虚构决策人。共享集团关系须由人录入；未识别的集团不会被自动猜测。
- 43项本地测试在 Linux/Python3.12执行；已提供 GitHub Actions 的 Python3.11/3.12 × Ubuntu/Windows 矩阵。远端 CI 结果以 PR 中实际运行状态为准。
- 未取得用户电脑上未推送项目的访问权限，不能排除本地已有相关实现。
