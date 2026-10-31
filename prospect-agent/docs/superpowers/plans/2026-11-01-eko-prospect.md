# EKO｜获客Agent｜实施计划｜1101

> For agentic workers: use superpowers:executing-plans task-by-task.

**Goal:** 可运行的有来源、有人审、可导出的客户开发流水线。
**Architecture:** Python 标准库模块、SQLite 持久化、规则提议与事实复核分离。
**Tech Stack:** Python >=3.11; unittest; sqlite3; http.server; 无强制模型依赖。
**Spec:** ../specs/2026-11-01-eko-prospect-design.md

## Global Constraints
不得推断邮箱/采购意向；不发送邮件；目标20家不是补足数量；6个日历月；保持原应用独立。

## Review Focus
- 抓取日/页脚年份被当作事件日期，过期来源不降级。
- 跨公司证据归属错误，网页关键词被认定采购事实。
- 重复导入重置人审，或新证据未使旧人审失效。
- 集团/邮箱重复联系，撤回证据后草稿仍放行。
- SSRF/重定向、HTML/CSV 注入、本地网页跨站写入。

### Task 1: Evidence store and policy
Files: eko_prospect/{identity,store,policy}.py; tests/test_policy.py
Interfaces: Store(path), upsert_company, add_source, add_claim, review_claim, review_company, score(company_id, as_of), export.
- [x] Write and run failing tests for date windows, evidence provenance, dedup, review invalidation and scoring.
- [x] Implement transactional SQLite with append-only audit and revisioned reviews.
- [x] Run python -m unittest discover -s tests -v; commit.

### Task 2: Acquisition and extraction
Files: eko_prospect/{fetch,extract,providers,pipeline}.py; tests/test_acquisition.py
Interfaces: Fetcher.fetch(url)->Document; Extractor.propose(document)->Claims; SearchProvider.search(query)->SearchHits.
- [x] Test blocked/private redirects, robots, failure persistence, quote validation and proposal-only extraction.
- [x] Implement crawl/import/search adapters with bounded budgets; configuration for US/CA/GB/DE.
- [x] Run complete unittest suite; commit.

### Task 3: Human workflow and export
Files: eko_prospect/{cli,web,export,outreach}.py; tests/test_workflow.py
Interfaces: python -m eko_prospect; localhost review UI; CSV; prepare_outreach with exact recipient human confirmation.
- [x] Test end-to-end ingestion→review→score→export and duplicate outreach/CSRF/CSV safety.
- [x] Implement CLI + UI and local-only .eml, no sending transport.
- [x] Run complete suite and CLI demo; commit.

### Task 4: Real-source validation and delivery
Files: examples/real_public_sample.json; README.md; docs/VALIDATION.md; AGENTS.md; root CI workflow scoped to prospect-agent.
- [x] Recheck three actual companies using official sources; preserve exact short excerpts, dates, capture provenance, leave missing values.
- [x] Run offline reproducible E2E, live acquisition attempts, fresh install and UI smoke test.
- [x] Independent whole-branch review; fix important findings using regression tests.
- [ ] Push branch and open PR; do not merge.
