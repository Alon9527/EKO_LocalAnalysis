let hoverImage = null;
let analyzeButton = null;
let panel = null;

document.addEventListener("mouseover", (event) => {
  const image = findImage(event.target);
  if (!image || !image.currentSrc && !image.src) return;
  hoverImage = image;
  showAnalyzeButton(image);
}, true);

document.addEventListener("scroll", () => {
  if (hoverImage && analyzeButton) positionButton(hoverImage);
}, true);

chrome.runtime.onMessage.addListener((message) => {
  if (message?.type === "EKO_CONTEXT_IMAGE" && message.imageUrl) {
    analyzePayload({
      imageUrl: message.imageUrl,
      fileName: filenameFromUrl(message.imageUrl),
    });
  }
});

function findImage(target) {
  if (!(target instanceof Element)) return null;
  const image = target.closest("img");
  if (!image) return null;
  const rect = image.getBoundingClientRect();
  if (rect.width < 80 || rect.height < 80) return null;
  return image;
}

function showAnalyzeButton(image) {
  if (!analyzeButton) {
    analyzeButton = document.createElement("button");
    analyzeButton.className = "eko-analyze-button";
    analyzeButton.type = "button";
    analyzeButton.textContent = "反推";
    analyzeButton.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (hoverImage) await analyzeImageElement(hoverImage);
    });
    document.documentElement.appendChild(analyzeButton);
  }
  positionButton(image);
  analyzeButton.style.display = "inline-flex";
}

function positionButton(image) {
  const rect = image.getBoundingClientRect();
  analyzeButton.style.left = `${Math.max(12, rect.right - 68)}px`;
  analyzeButton.style.top = `${Math.max(12, rect.top + 12)}px`;
}

async function analyzeImageElement(image) {
  const sourceUrl = image.currentSrc || image.src;
  const payload = {
    imageUrl: sourceUrl,
    fileName: filenameFromUrl(sourceUrl),
  };

  const dataUrl = tryImageToDataUrl(image);
  if (dataUrl && dataUrl.length < 28 * 1024 * 1024) {
    payload.base64Data = dataUrl;
    payload.mimeType = mimeFromDataUrl(dataUrl);
    delete payload.imageUrl;
  }

  await analyzePayload(payload);
}

async function analyzePayload(payload) {
  renderPanel({ state: "loading" });
  chrome.runtime.sendMessage({ type: "EKO_ANALYZE_IMAGE", payload }, (response) => {
    if (chrome.runtime.lastError) {
      renderPanel({ state: "error", error: chrome.runtime.lastError.message });
      return;
    }
    if (!response?.ok) {
      renderPanel({ state: "error", error: response?.error || "分析失败" });
      return;
    }
    renderPanel({ state: "done", item: response.item });
  });
}

function tryImageToDataUrl(image) {
  try {
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) return "";
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d");
    if (!context) return "";
    context.drawImage(image, 0, 0);
    return canvas.toDataURL("image/png");
  } catch {
    return "";
  }
}

function renderPanel(payload) {
  if (!panel) {
    panel = document.createElement("section");
    panel.className = "eko-result-panel";
    document.documentElement.appendChild(panel);
  }

  if (payload.state === "loading") {
    panel.innerHTML = `
      <div class="eko-panel-head">
        <strong>EKO 正在分析</strong>
        <button type="button" data-close>×</button>
      </div>
      <div class="eko-panel-body">图片已发送到本地软件，分析完成后会自动写入历史记录。</div>
    `;
  } else if (payload.state === "error") {
    panel.innerHTML = `
      <div class="eko-panel-head">
        <strong>分析失败</strong>
        <button type="button" data-close>×</button>
      </div>
      <div class="eko-panel-error">${escapeHtml(payload.error || "未知错误")}</div>
      <div class="eko-panel-hint">请确认 EKO 本地软件已打开，并且 API 设置可用。</div>
    `;
  } else {
    const item = payload.item || {};
    panel.innerHTML = `
      <div class="eko-panel-head">
        <strong>已保存到 EKO 历史</strong>
        <button type="button" data-close>×</button>
      </div>
      <div class="eko-score">质量评分 <b>${escapeHtml(String(item.qualityScore ?? "-"))}</b></div>
      <label>中文提示词</label>
      <textarea readonly>${escapeHtml(item.prompt_zh || "")}</textarea>
      <button type="button" data-copy="zh">复制中文</button>
      <label>English Prompt</label>
      <textarea readonly>${escapeHtml(item.prompt_en || "")}</textarea>
      <button type="button" data-copy="en">Copy English</button>
    `;
  }

  panel.querySelector("[data-close]")?.addEventListener("click", () => {
    panel.remove();
    panel = null;
  });
  panel.querySelector('[data-copy="zh"]')?.addEventListener("click", () => copyPanelText("zh"));
  panel.querySelector('[data-copy="en"]')?.addEventListener("click", () => copyPanelText("en"));
}

function copyPanelText(lang) {
  const fields = panel?.querySelectorAll("textarea") || [];
  const text = lang === "zh" ? fields[0]?.value : fields[1]?.value;
  if (text) navigator.clipboard.writeText(text);
}

function filenameFromUrl(url) {
  try {
    const parsed = new URL(url);
    const name = parsed.pathname.split("/").filter(Boolean).pop();
    return decodeURIComponent(name || "browser-image.jpg");
  } catch {
    return "browser-image.jpg";
  }
}

function mimeFromDataUrl(dataUrl) {
  const match = dataUrl.match(/^data:([^;]+);base64,/);
  return match?.[1] || "image/png";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
