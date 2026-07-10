const BRIDGE_BASE = "http://127.0.0.1:17621";

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "eko-analyze-image",
    title: "用 EKO 反推这张图片",
    contexts: ["image"],
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== "eko-analyze-image" || !tab?.id || !info.srcUrl) return;
  chrome.tabs.sendMessage(tab.id, {
    type: "EKO_CONTEXT_IMAGE",
    imageUrl: info.srcUrl,
  });
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === "EKO_HEALTH") {
    checkHealth().then(sendResponse).catch((error) => {
      sendResponse({ ok: false, error: normalizeError(error) });
    });
    return true;
  }

  if (message?.type === "EKO_ANALYZE_IMAGE") {
    analyzeImage(message.payload).then(sendResponse).catch((error) => {
      sendResponse({ ok: false, error: normalizeError(error) });
    });
    return true;
  }

  return false;
});

async function checkHealth() {
  const response = await fetch(`${BRIDGE_BASE}/health`, { method: "GET" });
  if (!response.ok) throw new Error(`本地软件连接失败 (${response.status})`);
  return response.json();
}

async function analyzeImage(payload) {
  const health = await checkHealth();
  if (!health?.ok) throw new Error("本地软件未就绪");

  const response = await fetch(`${BRIDGE_BASE}/analyze`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok || data?.ok === false) {
    throw new Error(data?.error || `分析失败 (${response.status})`);
  }
  return data;
}

function normalizeError(error) {
  if (error?.message) return error.message;
  return String(error || "未知错误");
}
