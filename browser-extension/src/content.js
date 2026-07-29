let hoverImage = null;
let analyzeButton = null;
let analyzeButtonGroup = null;
let analyzeCloseButton = null;
let panel = null;

document.addEventListener("mouseover", (event) => {
  if (event.target instanceof Element && event.target.closest(".eko-analyze-actions, .eko-result-panel")) return;
  const image = findImage(event.target);
  if (!image || (!image.currentSrc && !image.src)) return;
  hoverImage = image;
  showAnalyzeButton(image);
}, true);

document.addEventListener("scroll", () => {
  if (hoverImage && analyzeButtonGroup) positionButton(hoverImage);
}, true);

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  closePanel();
  hideAnalyzeButton();
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
  if (!analyzeButtonGroup) {
    analyzeButtonGroup = document.createElement("div");
    analyzeButtonGroup.className = "eko-analyze-actions";

    analyzeButton = document.createElement("button");
    analyzeButton.className = "eko-analyze-button";
    analyzeButton.type = "button";
    analyzeButton.textContent = "\u53cd\u63a8";
    analyzeButton.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (hoverImage) await analyzeImageElement(hoverImage);
    });

    analyzeCloseButton = document.createElement("button");
    analyzeCloseButton.className = "eko-analyze-close";
    analyzeCloseButton.type = "button";
    analyzeCloseButton.title = "\u5173\u95ed";
    analyzeCloseButton.setAttribute("aria-label", "\u5173\u95ed EKO \u60ac\u6d6e\u6309\u94ae");
    analyzeCloseButton.textContent = "x";
    analyzeCloseButton.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      hideAnalyzeButton();
    });

    analyzeButtonGroup.append(analyzeButton, analyzeCloseButton);
    document.documentElement.appendChild(analyzeButtonGroup);
  }
  positionButton(image);
  analyzeButtonGroup.style.display = "inline-flex";
}

function positionButton(image) {
  const rect = image.getBoundingClientRect();
  if (!analyzeButtonGroup) return;
  analyzeButtonGroup.style.left = `${Math.max(12, rect.right - 104)}px`;
  analyzeButtonGroup.style.top = `${Math.max(12, rect.top + 12)}px`;
}

function hideAnalyzeButton() {
  hoverImage = null;
  if (analyzeButtonGroup) analyzeButtonGroup.style.display = "none";
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
      renderPanel({ state: "error", error: response?.error || "\u5206\u6790\u5931\u8d25" });
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
        <strong>EKO \u6b63\u5728\u5206\u6790</strong>
        <button type="button" data-close aria-label="\u5173\u95ed">x</button>
      </div>
      <div class="eko-panel-body">\u56fe\u7247\u5df2\u53d1\u9001\u5230\u672c\u5730\u8f6f\u4ef6\uff0c\u5206\u6790\u5b8c\u6210\u540e\u4f1a\u81ea\u52a8\u5199\u5165\u5386\u53f2\u8bb0\u5f55\u3002</div>
    `;
  } else if (payload.state === "error") {
    panel.innerHTML = `
      <div class="eko-panel-head">
        <strong>\u5206\u6790\u5931\u8d25</strong>
        <button type="button" data-close aria-label="\u5173\u95ed">x</button>
      </div>
      <div class="eko-panel-error">${escapeHtml(payload.error || "\u672a\u77e5\u9519\u8bef")}</div>
      <div class="eko-panel-hint">\u8bf7\u786e\u8ba4 EKO \u672c\u5730\u8f6f\u4ef6\u5df2\u6253\u5f00\uff0c\u5e76\u4e14 API \u8bbe\u7f6e\u53ef\u7528\u3002</div>
    `;
  } else {
    const item = payload.item || {};
    panel.innerHTML = `
      <div class="eko-panel-head">
        <strong>\u5df2\u4fdd\u5b58\u5230 EKO \u5386\u53f2</strong>
        <button type="button" data-close aria-label="\u5173\u95ed">x</button>
      </div>
      <div class="eko-score">\u8d28\u91cf\u8bc4\u5206 <b>${escapeHtml(String(item.qualityScore ?? "-"))}</b></div>
      <label>\u4e2d\u6587\u63d0\u793a\u8bcd</label>
      <textarea readonly>${escapeHtml(item.prompt_zh || "")}</textarea>
      <button type="button" data-copy="zh">\u590d\u5236\u4e2d\u6587</button>
      <label>English Prompt</label>
      <textarea readonly>${escapeHtml(item.prompt_en || "")}</textarea>
      <button type="button" data-copy="en">Copy English</button>
    `;
  }

  panel.querySelector("[data-close]")?.addEventListener("click", closePanel);
  panel.querySelector('[data-copy="zh"]')?.addEventListener("click", () => copyPanelText("zh"));
  panel.querySelector('[data-copy="en"]')?.addEventListener("click", () => copyPanelText("en"));
}

function closePanel() {
  if (!panel) return;
  panel.remove();
  panel = null;
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