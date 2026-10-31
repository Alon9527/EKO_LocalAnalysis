# EKO｜获客Agent｜第一版｜1101

可实际运行的本地客户开发工具：发现候选 → 采集公开网页 → 保存来源与短引文 → 人工核验 → 可解释评分 → CSV。默认不需要模型、不需要付费 API；可选接入 Brave 搜索。**20 家是经审核的目标，不是自动补齐的名单。**

本目录与原图片分析应用独立，可以直接复制到新仓库。Python 3.11+；Windows、macOS、Linux；运行与测试均无第三方必装依赖。

## 3 分钟开始

先从 PR 分支取得代码（已有仓库可切换到此分支）。Windows 可使用 `py` 替代 `python`：

```bash
git clone --branch codex/eko-prospect-agent-v1-1101 https://github.com/Alon9527/EKO_LocalAnalysis.git
cd EKO_LocalAnalysis/prospect-agent
python -m eko_prospect --db workspace/demo.sqlite import examples/real_public_sample.json
python -m eko_prospect --db workspace/demo.sqlite serve
```

打开 <http://127.0.0.1:8765>。你会看到三家真实企业及来源摘录，全部为 `pending`，没有任何客户已被代为批准。点击企业，核对原文、审核人和理由后逐条保存。退出用 Ctrl+C。

不需要安装也能运行；希望使用短命令可执行 `python -m pip install -e .`，之后使用 `eko-prospect ...`。

## 一次执行采集与导出

```bash
python -m eko_prospect --db workspace/us.sqlite run --seeds examples/seeds_us.csv --market US --limit 20 --max-pages 5 --out workspace/us.csv
python -m eko_prospect --db workspace/us.sqlite serve
```

示例 CSV 只有三家公司，因此最多发现三家。增加 `name,website,market,customer_type` 行可扩展候选；仅有 name / website 为必填。`customer_type` 为待核实标签。每家公司最多抓取 5 页，自动跟随同主机的供应商、产品、联系与公司页面。会保留每次失败，部分失败退出码为 2，成功部分照常落库。不绕过 robots、登录或验证码。

运行环境若必须经**受控出口代理**联网，使用 `--trusted-proxy`。仅在已有 HTTP_PROXY/HTTPS_PROXY 且代理负责阻止私网目标时使用；此模式由代理解析 DNS。默认模式直接校验公共 IP 并固定连接地址，防止 DNS 重绑定。不会自动启用代理或降低默认规则。代理模式强制所有请求经过对应协议代理，忽略 NO_PROXY；未配置该协议代理时拒绝请求。

```bash
python -m eko_prospect run --seeds examples/seeds_us.csv --trusted-proxy --max-pages 3
```

需要从公开搜索发现更多企业时，设置 `BRAVE_SEARCH_API_KEY` 环境变量（不要提交到 Git），改为：

```bash
python -m eko_prospect run --brave --market US --limit 20 --max-pages 5
python -m eko_prospect queries --market DE
```

搜索入口参考 [Brave 官方文档](https://api-dashboard.search.brave.com/app/documentation/web-search/get-started)。未配置密钥时不会调用，也不假装搜索成功。本次未做真实付费 Brave 调用。搜索标题只作候选名称，需要人审；结果页日期、排名、摘要均不会被写为已核实事实。

## 人工复核流程

1. 核对公司身份、官网归属与目标市场。种子中的 US 只是研究目标，仍需 `market` 来源证据。
2. 打开每条来源，检查摘录是否支持 `category`、`channel`、`supplier_access` 等断言。`email` 仅表示公开展示，不证明可投递，更不证明其为采购人。岗位、个人姓名不自动生成，未核实则留空。
3. 对 `expansion` / `procurement`，使用“补充证据”填写原文短句、事件日和原文日期。支持 `2026-09-23`、`September 23, 2026` 等明确日期。还需人工确认日期确实属于该事件；网页发布日期不自动等于事件日。采购线索应填写统一的事件标识 `value`，用于交叉核验。第二来源须核对出版者确实独立，再勾选“独立外部采购来源”或使用 `review-claim --independent-source`；同公司子域名不算独立来源。
4. 核对既有 CRM/销售记录，将 relationship 设置为 new / existing / do_not_contact；同集团使用相同 group_key。默认 unknown 会阻止批准。
5. 写明 EKO 合作切入点，它被单独标为建议。评分、来源与阻碍满足条件后才能批准，审核人与理由进入审计记录。网页审批绑定你看到的 revision；数据更新后旧表单会被拒绝。CLI 批准须传入 `show` 中的 `--expected-revision`。
6. 导出 CSV。缺失值为空或在 missing 列明确列出。CSV 含所有候选及其审核状态，**不是可直接群发名单**。

网页只开放本机，具备 Host/Origin 与 CSRF 校验。不应暴露到公网；多用户权限体系属于下一版本。

## 评分依据

| 维度 | 分数 | 证据要求 |
|---|---:|---|
| 品类 | 0 / 2 | 官方来源中的相关垃圾桶品类，人工核实 |
| 渠道 | 0 / 2 | 官方来源证明零售、分销、批发等适用渠道 |
| 近期事件 | 0 / 1 / 2 | 六个日历月内的明确事件；扩张或单源采购最多 1，采购须独立来源同事件交叉核验才 2 |
| 供应商入口 | 0 / 2 | 官方招商、供应商申请或相关联系入口 |
| 合作可行性 | 0 / 2 | 人工结合可引用来源评估价位/商业条件；这是判断，不是采购事实 |

仅人工核验且来源有效的证据计分。来源观测时间也必须在六个月内；新抓取版本使旧证据失效，需要重新审核。总分下限显示 1，`raw_total=0` 表示没有有效计分依据。`rubric_version=eko-v1.0`，每个维度和支持的证据 ID 都可追溯。

计入 20 家：评分至少 8、市场与品类核实、有效供应商入口、至少两个不同来源 URL、查过既有合作关系、未抑制、当前 revision 获人工批准。按集团去重。新增/撤回证据或更换来源会撤销旧批准。近期信号缺失会显式提示；没有信号也不得编造来凑分。评分代表开发优先级，**不代表采购概率**。

## 邮件与重复联系

代码没有 SMTP、Gmail 发送或群发通道。只有人工确认后的本地 `.eml` 草稿，以及手动发送记录。

```bash
python -m eko_prospect draft --company 1 --email vendors@katom.com --subject "EKO commercial waste solutions" --body-file workspace/reviewed-email.txt --actor Alon --confirm-recipient vendors@katom.com
```

这条命令在示例尚未批准时会被拒绝。需要：当前客户批准、公开邮箱已核验，且该邮箱明确对应已核验的供应商入口（`supplier_access` 原文必须精确出现完整邮箱，单独填写 value 不构成归属证据）。客服/新闻邮箱不能直接当买手。

草稿准备时事务性预留集团和邮箱；即使尚未发送也会阻止再次生成同公司/集团/邮箱的开发草稿。`--confirm-recipient` 必须逐封精确填写。正文与标题 hash、操作者、时间留档。请在邮箱中再次核对后手动发送，再记账：

```bash
python -m eko_prospect sent --draft 1 --actor Alon --note "已在工作邮箱手动发送"
python -m eko_prospect suppress --key example.com --actor Alon --reason "已有同事跟进"
python -m eko_prospect suppress --key buyer@example.com --actor Alon --reason "不要联系"
```

抑制键可以是域名、group_key 或完整邮箱。暂无自动跟进、自动释放预留或定时群发。若草稿文件写入失败，保留数据库预留以避免重复。邮件操作不可使用历史评估日期绕过新鲜度检查。

## 常用命令

```bash
python -m eko_prospect --help
python -m eko_prospect add-company --name "Company" --website https://example.com --market CA --type distributor
python -m eko_prospect collect --company 1 --url https://www.katom.com/help.html
python -m eko_prospect show --company 1
python -m eko_prospect review-claim --claim 1 --status verified --actor Alon --note "已逐项核对原文和角色"
python -m eko_prospect review-company --company 1 --status hold --actor Alon --note "待查既有关系与价格条件"
python -m eko_prospect export --out workspace/prospects.csv
python -m eko_prospect audit
python -m eko_prospect --as-of 2026-11-01 list
```

全局参数 `--db`、`--as-of` 放在子命令前；默认 DB 为当前目录下 `workspace/prospects.sqlite`。`--as-of` 用于历史评估和可重现测试，不是网站事件时间。

## 来源与数据模型

- `companies`：以精确域名（仅去 www）+ 市场去重；不会根据相似名称误合并。同集团人工关联。
- `sources`：最终 URL、抓取时间、正文 SHA256、HTTP/人工摘录、失败原因、版本有效性。短摘录 hash 对应所留存摘录，不冒充完整页面 hash。
- `claims`：证据类型、原文（<=25词）、事件日期及日期锚点、pending / verified / rejected、审核记录。
- `audit`：追加式操作流水；`suppressions` 与 `outreach`：防重复及人工联系记录。
- 联系人从当前有效证据派生，按完整邮箱/规范 LinkedIn URL 去重，不猜测姓名邮箱模式、不合并点号或加号别名。

`examples/real_public_sample.json` 演示人工摘录导入格式。`import` 不接受 status、reviewer 或其他绕过复核字段，导入失败整体回滚。网站不能直接修改审核状态。PDF/JS 页面可人工核对后按此格式录入短摘录；不绕过登录，不自动抓取 LinkedIn。

## 模块和扩展

| 模块 | 职责 |
|---|---|
| agent.py | 有界流水线编排 |
| providers.py | SearchProvider / CSV / 可选 Brave；US CA GB DE 配置 |
| fetch.py | robots / 请求限制 / 公共目标校验 / 受控代理 |
| extract.py | Extractor 协议 / 保守规则候选；模型可替换此处 |
| pipeline.py | 采集、候选写入、原子导入 |
| store.py | SQLite、来源版本、审核修订、审计 |
| policy.py | 六个月窗口、评分、达标门槛 |
| web.py / cli.py | 本地复核台与脚本入口 |
| export.py | CSV与集团去重进度 |
| outreach.py | 人工草稿和防重复；无发信实现 |

可以接入本地模型或其他模型 API 产生相同的 `kind/quote/value/event_date/date_quote` 候选；不得让模型直接返回 verified 或修改评分。现有核心无需模型仍能运行。加拿大、英国、德国已有市场配置；德语语义提取仍有限，需补充词表与人工本地化核验。

## 测试、验证与后续

```bash
python -m unittest discover -s tests -v
python -m compileall -q eko_prospect
```

测试全部离线，无 Key；真实样本使用历史短摘录，自动化只在临时数据库模拟人工审核，不写入交付数据库。完整范围和真实抓取结果见 [验证记录](docs/VALIDATION.md)。GitHub Actions 仅在本目录变更时测试 Python 3.11/3.12，不运行抓取或联系客户。

建议下一轮按顺序：① 导入销售 CRM 的已有客户/集团/抑制记录；② 逐批补足美国市场证据，审核后再追求20家；③ 增加更准确的公司实体、角色与发布日期提取，并保留人工证据关口；④ 新增官方采购公告/供应商门户适配器；⑤ 扩展 CA/GB/DE 本地语言与商业条件。平台认证、报价、MOQ、物流能力需要 EKO 的实际资料，不由 Agent 编造。

数据库 schema v2 会自动迁移 v1，保留原数据并为独立来源核验添加默认关闭的标记。迁移前建议备份数据库。

运行数据默认存于被 Git 忽略的 workspace/；请定期备份此目录。公开仓库只含代码、测试和短公开样本。不要提交真实客户工作表、API Key、抓取全文、邮件正文或数据库。
