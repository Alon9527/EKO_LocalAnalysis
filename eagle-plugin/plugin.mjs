import {
  BRIDGE_BASE,
  buildAnalyzePayload,
  escapeHtml,
  normalizeError,
  promptForItem,
  selectedItemKey,
} from "./src/eagle-utils.mjs";

const fs = window.require ? window.require("fs") : null;
const http = window.require ? window.require("http") : null;

const state = {
  selectedItem: null,
  resultItem: null,
  target: "gpt",
  lang: "zh",
  busy: false,
  status: "等待选择图片",
  error: "",
  selectionKey: "",
  selectionTimer: null,
};

const $ = (selector) => document.querySelector(selector);

function render() {
  const item = state.selectedItem;
  const prompt = promptForItem(state.resultItem, state.target, state.lang);
  const thumb = item?.thumbnailURL || item?.fileURL || "";
  $("#app").innerHTML = `
    <section class="panel">
      <header class="header">
        <div>
          <h1>EKO 图片反推</h1>
          <p>${item ? escapeHtml(item.name || item.filePath || "已选择图片") : "在 Eagle 中选择一张图片"}</p>
        </div>
        <button class="icon-button" id="refreshButton" title="刷新选择">↻</button>
      </header>

      ${thumb ? `<img class="preview" src="${escapeHtml(thumb)}" alt="" />` : `<div class="empty-preview">未读取到预览图</div>`}

      <div class="actions">
        <button class="primary" id="analyzeButton" ${state.busy || !item ? "disabled" : ""}>${state.busy ? "分析中..." : "反推这张图"}</button>
        <button id="healthButton" ${state.busy ? "disabled" : ""}>检查 EKO</button>
      </div>

      <p class="status ${state.error ? "is-error" : ""}">${escapeHtml(state.error || state.status)}</p>

      ${state.resultItem ? `
        <div class="tabs model-tabs">
          <button data-target="gpt" class="${state.target === "gpt" ? "active" : ""}">GPT Image</button>
          <button data-target="nano" class="${state.target === "nano" ? "active" : ""}">Nano Banana</button>
        </div>
        <div class="tabs lang-tabs">
          <button data-lang="zh" class="${state.lang === "zh" ? "active" : ""}">中文</button>
          <button data-lang="en" class="${state.lang === "en" ? "active" : ""}">English</button>
        </div>
        <label class="prompt-label">${state.target === "nano" ? "Nano Banana" : "GPT Image"} · ${state.lang === "zh" ? "中文 Prompt" : "English Prompt"}</label>
        <textarea readonly id="promptText">${escapeHtml(prompt)}</textarea>
        <button class="copy" id="copyButton">复制当前 Prompt</button>
      ` : ""}
    </section>
  `;
  bindEvents();
}

function bindEvents() {
  $("#refreshButton")?.addEventListener("click", refreshSelection);
  $("#healthButton")?.addEventListener("click", checkHealthWithUi);
  $("#analyzeButton")?.addEventListener("click", analyzeSelected);
  document.querySelectorAll("[data-target]").forEach((button) => {
    button.addEventListener("click", () => { state.target = button.dataset.target; render(); });
  });
  document.querySelectorAll("[data-lang]").forEach((button) => {
    button.addEventListener("click", () => { state.lang = button.dataset.lang; render(); });
  });
  $("#copyButton")?.addEventListener("click", copyPrompt);
}

async function refreshSelection(options = {}) {
  const silent = Boolean(options.silent);
  if (!silent) state.error = "";
  try {
    const selected = await eagle.item.getSelected();
    const nextItem = Array.isArray(selected) ? selected[0] : selected;
    const nextKey = selectedItemKey(nextItem);
    const changed = nextKey !== state.selectionKey;
    if (!changed && silent) return;

    state.selectedItem = nextItem || null;
    state.selectionKey = nextKey;
    if (changed) {
      state.resultItem = null;
      state.error = "";
    }
    state.status = state.selectedItem ? "已选择图片，可开始反推" : "请选择一张图片";
    render();
  } catch (error) {
    if (!silent) {
      state.error = normalizeError(error);
      render();
    }
  }
}

function startSelectionWatcher() {
  if (state.selectionTimer) clearInterval(state.selectionTimer);
  state.selectionTimer = setInterval(() => {
    if (!state.busy && window.eagle?.item?.getSelected) {
      refreshSelection({ silent: true });
    }
  }, 700);

  window.addEventListener("focus", () => refreshSelection({ silent: true }));
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) refreshSelection({ silent: true });
  });
}
function requestJson(method, path, body) {
  return new Promise((resolve, reject) => {
    if (!http) return reject(new Error("Eagle 插件无法访问 Node http 模块"));
    const payload = body ? Buffer.from(JSON.stringify(body), "utf8") : null;
    const req = http.request({
      hostname: "127.0.0.1",
      port: 17621,
      path,
      method,
      headers: payload ? { "Content-Type": "application/json", "Content-Length": payload.length } : {},
      timeout: method === "GET" ? 3000 : 120000,
    }, (res) => {
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => {
        const text = Buffer.concat(chunks).toString("utf8");
        let data = {};
        try { data = text ? JSON.parse(text) : {}; } catch { data = { error: text }; }
        if (res.statusCode < 200 || res.statusCode >= 300 || data.ok === false) {
          reject(new Error(data.error || `EKO 请求失败 (${res.statusCode})`));
        } else {
          resolve(data);
        }
      });
    });
    req.on("timeout", () => req.destroy(new Error("Request timeout")));
    req.on("error", reject);
    if (payload) req.write(payload);
    req.end();
  });
}

async function checkHealthWithUi() {
  state.busy = true;
  state.error = "";
  state.status = "正在连接 EKO...";
  render();
  try {
    const health = await requestJson("GET", "/health");
    state.status = `EKO 已连接 · ${health.version || "unknown"}`;
  } catch (error) {
    state.error = normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function analyzeSelected() {
  if (!state.selectedItem) return;
  state.busy = true;
  state.error = "";
  state.status = "正在读取 Eagle 图片...";
  render();
  try {
    if (!fs) throw new Error("Eagle 插件无法访问 Node fs 模块");
    await requestJson("GET", "/health");
    const bytes = fs.readFileSync(state.selectedItem.filePath);
    state.status = "正在发送到 EKO 分析...";
    render();
    const payload = buildAnalyzePayload(state.selectedItem, bytes);
    const response = await requestJson("POST", "/analyze", payload);
    state.resultItem = response.item;
    state.target = response.item?.promptNanoBananaZh || response.item?.promptNanoBananaEn ? "nano" : "gpt";
    state.status = "已保存到 EKO 历史";
  } catch (error) {
    state.error = normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function copyPrompt() {
  const prompt = promptForItem(state.resultItem, state.target, state.lang);
  if (!prompt) return;
  await navigator.clipboard.writeText(prompt);
  state.status = "Prompt 已复制";
  render();
}

window.addEventListener("DOMContentLoaded", async () => {
  render();
  if (!window.eagle) {
    state.error = "请在 Eagle 插件环境中打开";
    render();
    return;
  }
  try {
    const theme = await eagle.app.theme;
    document.body.setAttribute("theme", theme || "dark");
    eagle.onThemeChanged((nextTheme) => document.body.setAttribute("theme", nextTheme || "dark"));
  } catch {}
  await refreshSelection();
  startSelectionWatcher();
});
