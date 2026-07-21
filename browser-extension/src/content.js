let hoverImage = null;
let analyzeButton = null;
let panel = null;
let lastPayload = null;
let lastItem = null;
let selectedTarget = 'gpt';
let selectedLang = 'zh';

const utils = globalThis.EKOExtensionUtils;
const TEXT = {
  button: 'EKO \u53cd\u63a8',
  busy: '\u5206\u6790\u4e2d...',
  blobError: '\u8fd9\u5f20\u56fe\u7247\u662f\u9875\u9762\u4e34\u65f6 blob \u56fe\u7247\uff0c\u6d4f\u89c8\u5668\u7981\u6b62\u63d2\u4ef6\u8bfb\u53d6\u3002\u8bf7\u5c1d\u8bd5\u53f3\u952e\u590d\u5236\u56fe\u7247\u540e\u5728 EKO \u8f6f\u4ef6\u4e2d\u7c98\u8d34\u5206\u6790\u3002',
  failed: '\u5206\u6790\u5931\u8d25',
  unknown: '\u672a\u77e5\u9519\u8bef',
  loadingTitle: 'EKO \u6b63\u5728\u5206\u6790',
  close: '\u5173\u95ed',
  loadingBody: '\u56fe\u7247\u5df2\u53d1\u9001\u5230\u672c\u5730\u8f6f\u4ef6\uff0c\u5206\u6790\u5b8c\u6210\u540e\u4f1a\u81ea\u52a8\u5199\u5165\u5386\u53f2\u8bb0\u5f55\u3002',
  hint: '\u8bf7\u786e\u8ba4 EKO \u672c\u5730\u8f6f\u4ef6\u5df2\u6253\u5f00\uff0c\u5e76\u4e14\u8bbe\u7f6e\u4e2d\u5fc3\u7684 API \u53ef\u7528\u3002',
  retry: '\u91cd\u8bd5',
  health: '\u68c0\u67e5\u8fde\u63a5',
  saved: '\u5df2\u4fdd\u5b58\u5230 EKO \u5386\u53f2',
  score: '\u8d28\u91cf\u8bc4\u5206',
  zh: '\u4e2d\u6587',
  copy: '\u590d\u5236\u5f53\u524d Prompt',
  reanalyze: '\u91cd\u65b0\u5206\u6790',
  healthOk: '\u8fde\u63a5\u6b63\u5e38',
  connected: '\u5df2\u8fde\u63a5 EKO \u672c\u5730\u8f6f\u4ef6 v',
};

document.addEventListener('mouseover', (event) => {
  const image = findImage(event.target);
  if (!image || (!image.currentSrc && !image.src)) return;
  hoverImage = image;
  showAnalyzeButton(image);
}, true);

document.addEventListener('scroll', () => {
  if (hoverImage && analyzeButton) positionButton(hoverImage);
}, true);

window.addEventListener('resize', () => {
  if (hoverImage && analyzeButton) positionButton(hoverImage);
});

chrome.runtime.onMessage.addListener((message) => {
  if (message?.type === 'EKO_CONTEXT_IMAGE' && message.imageUrl) {
    void analyzePayload({ imageUrl: message.imageUrl, fileName: utils.filenameFromUrl(message.imageUrl) });
  }
});

function findImage(target) {
  if (!(target instanceof Element)) return null;
  const image = target.closest('img');
  if (!image) return null;
  const rect = image.getBoundingClientRect();
  if (rect.width < 80 || rect.height < 80) return null;
  return image;
}

function showAnalyzeButton(image) {
  if (!analyzeButton) {
    analyzeButton = document.createElement('button');
    analyzeButton.className = 'eko-analyze-button';
    analyzeButton.type = 'button';
    analyzeButton.textContent = TEXT.button;
    analyzeButton.addEventListener('click', async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (hoverImage) await analyzeImageElement(hoverImage);
    });
    document.documentElement.appendChild(analyzeButton);
  }
  positionButton(image);
  analyzeButton.style.display = 'inline-flex';
}

function positionButton(image) {
  const rect = image.getBoundingClientRect();
  analyzeButton.style.left = Math.max(12, rect.right - 92) + 'px';
  analyzeButton.style.top = Math.max(12, rect.top + 12) + 'px';
}

async function analyzeImageElement(image) {
  const sourceUrl = image.currentSrc || image.src;
  const payload = { imageUrl: sourceUrl, fileName: utils.filenameFromUrl(sourceUrl) };
  if (sourceUrl.startsWith('data:image/')) {
    payload.base64Data = sourceUrl;
    payload.mimeType = utils.mimeFromDataUrl(sourceUrl);
    delete payload.imageUrl;
  } else {
    const dataUrl = tryImageToDataUrl(image);
    if (dataUrl && dataUrl.length < 28 * 1024 * 1024) {
      payload.base64Data = dataUrl;
      payload.mimeType = utils.mimeFromDataUrl(dataUrl);
      delete payload.imageUrl;
    } else if (sourceUrl.startsWith('blob:')) {
      renderPanel({ state: 'error', error: TEXT.blobError });
      return;
    }
  }
  await analyzePayload(payload);
}

async function analyzePayload(payload) {
  lastPayload = payload;
  setButtonBusy(true);
  renderPanel({ state: 'loading' });
  chrome.runtime.sendMessage({ type: 'EKO_ANALYZE_IMAGE', payload }, (response) => {
    setButtonBusy(false);
    if (chrome.runtime.lastError) {
      renderPanel({ state: 'error', error: utils.normalizeError(chrome.runtime.lastError) });
      return;
    }
    if (!response?.ok) {
      renderPanel({ state: 'error', error: response?.error || TEXT.failed });
      return;
    }
    lastItem = response.item;
    selectedTarget = response.item?.promptNanoBananaEn || response.item?.promptNanoBananaZh ? 'nano' : 'gpt';
    renderPanel({ state: 'done', item: response.item });
  });
}

function setButtonBusy(isBusy) {
  if (!analyzeButton) return;
  analyzeButton.disabled = isBusy;
  analyzeButton.textContent = isBusy ? TEXT.busy : TEXT.button;
}

function tryImageToDataUrl(image) {
  try {
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) return '';
    const canvas = document.createElement('canvas');
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext('2d');
    if (!context) return '';
    context.drawImage(image, 0, 0);
    return canvas.toDataURL('image/png');
  } catch {
    return '';
  }
}

function renderPanel(payload) {
  if (!panel) {
    panel = document.createElement('section');
    panel.className = 'eko-result-panel';
    document.documentElement.appendChild(panel);
  }
  if (payload.state === 'loading') panel.innerHTML = renderLoading();
  else if (payload.state === 'error') panel.innerHTML = renderError(payload.error);
  else panel.innerHTML = renderDone(payload.item || {});
  bindPanelEvents();
}

function renderLoading() {
  return '<div class="eko-panel-head"><strong>' + TEXT.loadingTitle + '</strong><button type="button" data-close aria-label="' + TEXT.close + '">&times;</button></div><div class="eko-panel-body"><span class="eko-spinner"></span>' + TEXT.loadingBody + '</div>';
}

function renderError(error) {
  return '<div class="eko-panel-head"><strong>' + TEXT.failed + '</strong><button type="button" data-close aria-label="' + TEXT.close + '">&times;</button></div><div class="eko-panel-error">' + utils.escapeHtml(error || TEXT.unknown) + '</div><div class="eko-panel-hint">' + TEXT.hint + '</div><div class="eko-panel-actions"><button type="button" data-retry>' + TEXT.retry + '</button><button type="button" data-health>' + TEXT.health + '</button></div>';
}

function renderDone(item) {
  const prompt = utils.promptForItem(item, selectedTarget, selectedLang);
  const score = item.qualityScore ?? '-';
  const modelLabel = selectedTarget === 'nano' ? 'Nano Banana' : 'GPT Image';
  const langLabel = selectedLang === 'zh' ? TEXT.zh : 'English';
  return '<div class="eko-panel-head"><strong>' + TEXT.saved + '</strong><button type="button" data-close aria-label="' + TEXT.close + '">&times;</button></div>'
    + '<div class="eko-score">' + TEXT.score + ' <b>' + utils.escapeHtml(String(score)) + '</b></div>'
    + '<div class="eko-tabs" role="tablist"><button type="button" data-target="gpt" class="' + (selectedTarget === 'gpt' ? 'is-active' : '') + '">GPT Image</button><button type="button" data-target="nano" class="' + (selectedTarget === 'nano' ? 'is-active' : '') + '">Nano Banana</button></div>'
    + '<div class="eko-tabs eko-tabs--compact" role="tablist"><button type="button" data-lang="zh" class="' + (selectedLang === 'zh' ? 'is-active' : '') + '">' + TEXT.zh + '</button><button type="button" data-lang="en" class="' + (selectedLang === 'en' ? 'is-active' : '') + '">English</button></div>'
    + '<label>' + modelLabel + ' &middot; ' + langLabel + ' Prompt</label><textarea readonly>' + utils.escapeHtml(prompt) + '</textarea>'
    + '<div class="eko-panel-actions"><button type="button" data-copy>' + TEXT.copy + '</button><button type="button" data-retry>' + TEXT.reanalyze + '</button></div>';
}

function bindPanelEvents() {
  panel.querySelector('[data-close]')?.addEventListener('click', () => { panel.remove(); panel = null; });
  panel.querySelector('[data-retry]')?.addEventListener('click', () => { if (lastPayload) void analyzePayload(lastPayload); });
  panel.querySelector('[data-health]')?.addEventListener('click', () => checkBridgeHealth());
  panel.querySelector('[data-copy]')?.addEventListener('click', () => copyCurrentPrompt());
  panel.querySelectorAll('[data-target]').forEach((button) => button.addEventListener('click', () => { selectedTarget = button.getAttribute('data-target') || 'gpt'; renderPanel({ state: 'done', item: lastItem }); }));
  panel.querySelectorAll('[data-lang]').forEach((button) => button.addEventListener('click', () => { selectedLang = button.getAttribute('data-lang') || 'zh'; renderPanel({ state: 'done', item: lastItem }); }));
}

function checkBridgeHealth() {
  chrome.runtime.sendMessage({ type: 'EKO_HEALTH' }, (response) => {
    if (chrome.runtime.lastError || !response?.ok) {
      renderPanel({ state: 'error', error: response?.error || chrome.runtime.lastError?.message || TEXT.failed });
      return;
    }
    panel.innerHTML = '<div class="eko-panel-head"><strong>' + TEXT.healthOk + '</strong><button type="button" data-close aria-label="' + TEXT.close + '">&times;</button></div><div class="eko-panel-body">' + TEXT.connected + utils.escapeHtml(response.version || '-') + '&#12290;</div>';
    bindPanelEvents();
  });
}

function copyCurrentPrompt() {
  const text = panel?.querySelector('textarea')?.value || '';
  if (text) navigator.clipboard.writeText(text);
}
