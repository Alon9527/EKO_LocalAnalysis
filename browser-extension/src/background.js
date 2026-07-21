importScripts('extension-utils.js');

const BRIDGE_BASE = 'http://127.0.0.1:17621';
const REQUEST_TIMEOUT_MS = 120000;
const HEALTH_TIMEOUT_MS = 3000;

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: 'eko-analyze-image', title: '\u7528 EKO \u53cd\u63a8\u8fd9\u5f20\u56fe\u7247', contexts: ['image'] });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== 'eko-analyze-image' || !tab?.id || !info.srcUrl) return;
  chrome.tabs.sendMessage(tab.id, { type: 'EKO_CONTEXT_IMAGE', imageUrl: info.srcUrl });
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === 'EKO_HEALTH') {
    checkHealth().then(sendResponse).catch((error) => sendResponse({ ok: false, error: EKOExtensionUtils.normalizeError(error) }));
    return true;
  }
  if (message?.type === 'EKO_ANALYZE_IMAGE') {
    analyzeImage(message.payload).then(sendResponse).catch((error) => sendResponse({ ok: false, error: EKOExtensionUtils.normalizeError(error) }));
    return true;
  }
  return false;
});

async function checkHealth() {
  const response = await fetchWithTimeout(BRIDGE_BASE + '/health', { method: 'GET' }, HEALTH_TIMEOUT_MS);
  const data = await response.json().catch(() => ({}));
  if (!response.ok || data?.ok === false) throw new Error(data?.error || '\u672c\u5730\u8f6f\u4ef6\u8fde\u63a5\u5931\u8d25 (' + response.status + ')');
  return data;
}

async function analyzeImage(payload) {
  const health = await checkHealth();
  if (!health?.ok) throw new Error('\u672c\u5730\u8f6f\u4ef6\u672a\u5c31\u7eea');
  const response = await fetchWithTimeout(BRIDGE_BASE + '/analyze', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) }, REQUEST_TIMEOUT_MS);
  const data = await response.json().catch(() => ({}));
  if (!response.ok || data?.ok === false) throw new Error(data?.error || '\u5206\u6790\u5931\u8d25 (' + response.status + ')');
  return data;
}

async function fetchWithTimeout(url, options, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(new Error('Request timed out')), timeoutMs);
  try { return await fetch(url, { ...options, signal: controller.signal }); }
  finally { clearTimeout(timer); }
}
