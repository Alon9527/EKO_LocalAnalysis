let hoverImage = null;
let analyzeButton = null;
let analyzeButtonGroup = null;
let analyzeCloseButton = null;
let panel = null;
let lastPayload = null;
let lastItem = null;
let selectedTarget = 'gpt';
let selectedLang = 'zh';

const utils = globalThis.EKOExtensionUtils;
const TEXT = {
  button: 'EKO 反推',
  busy: '分析中...',
  blobError: '这张图片是页面临时 blob 图片，浏览器禁止插件读取。请尝试右键复制图片后在 EKO 软件中粘贴分析。',
  failed: '分析失败',
  unknown: '未知错误',
  loadingTitle: 'EKO 正在分析',
  close: '关闭',
  loadingBody: '图片已发送到本地软件，分析完成后会自动写入历史记录。',
  hint: '请确认 EKO 本地软件已打开，并且设置中心的 API 可用。',
  retry: '重试',
  health: '检查连接',
  saved: '已保存到 EKO 历史',
  score: '质量评分',
  zh: '中文',
  copy: '复制当前 Prompt',
  reanalyze: '重新分析',
  healthOk: '连接正常',
  connected: '已连接 EKO 本地软件 v',
};

document.addEventListener('mouseover', (event) => {
  if (event.target instanceof Element && event.target.closest('.eko-analyze-actions, .eko-result-panel')) return;
  const image = findImage(event.target);
  if (!image || (!image.currentSrc && !image.src)) return;
  hoverImage = image;
  showAnalyzeButton(image);
}, true);

document.addEventListener('scroll', () => {
  if (hoverImage && analyzeButtonGroup) positionButton(hoverImage);
}, true);

window.addEventListener('resize', () => {
  if (hoverImage && analyzeButtonGroup) positionButton(hoverImage);
});

document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape') return;
  closePanel();
  hideAnalyzeButton();
}, true);

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
  if (!analyzeButtonGroup) {
    analyzeButtonGroup = document.createElement('div');
    analyzeButtonGroup.className = 'eko-analyze-actions';

    analyzeButton = document.createElement('button');
    analyzeButton.className = 'eko-analyze-button';
    analyzeButton.type = 'button';
    analyzeButton.textContent = TEXT.button;
    analyzeButton.addEventListener('click', async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (hoverImage) await analyzeImageElement(hoverImage);
    });

    analyzeCloseButton = document.createElement('button');
    analyzeCloseButton.className = 'eko-analyze-close';
    analyzeCloseButton.type = 'button';
    analyzeCloseButton.title = TEXT.close;
    analyzeCloseButton.setAttribute('aria-label', TEXT.close);
    analyzeCloseButton.innerHTML = '&times;';
    analyzeCloseButton.addEventListener('click', (event) => {
      event.preventDefault();
      event.stopPropagation();
      hideAnalyzeButton();
    });

    analyzeButtonGroup.append(analyzeButton, analyzeCloseButton);
    document.documentElement.appendChild(analyzeButtonGroup);
  }
  positionButton(image);
  analyzeButtonGroup.style.display = 'inline-flex';
}

function positionButton(image) {
  const rect = image.getBoundingClientRect();
  if (!analyzeButtonGroup) return;
  analyzeButtonGroup.style.left = Math.max(12, rect.right - 132) + 'px';
  analyzeButtonGroup.style.top = Math.max(12, rect.top + 12) + 'px';
}

function hideAnalyzeButton() {
  hoverImage = null;
  if (analyzeButtonGroup) analyzeButtonGroup.style.display = 'none';
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
    const contractError = utils.validatePromptContract(response.item);
    if (contractError) {
      renderPanel({ state: 'error', error: contractError });
      return;
    }
    lastItem = response.item;
    selectedTarget = 'gpt';
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
  panel.querySelector('[data-close]')?.addEventListener('click', closePanel);
  panel.querySelector('[data-retry]')?.addEventListener('click', () => { if (lastPayload) void analyzePayload(lastPayload); });
  panel.querySelector('[data-health]')?.addEventListener('click', checkBridgeHealth);
  panel.querySelector('[data-copy]')?.addEventListener('click', copyCurrentPrompt);
  panel.querySelectorAll('[data-target]').forEach((button) => button.addEventListener('click', () => {
    selectedTarget = button.getAttribute('data-target') || 'gpt';
    renderPanel({ state: 'done', item: lastItem });
  }));
  panel.querySelectorAll('[data-lang]').forEach((button) => button.addEventListener('click', () => {
    selectedLang = button.getAttribute('data-lang') || 'zh';
    renderPanel({ state: 'done', item: lastItem });
  }));
}

function closePanel() {
  if (!panel) return;
  panel.remove();
  panel = null;
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