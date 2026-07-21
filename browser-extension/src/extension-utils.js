(function (root) {
  function filenameFromUrl(url) {
    try {
      const parsed = new URL(url);
      const name = parsed.pathname.split('/').filter(Boolean).pop();
      return decodeURIComponent(name || 'browser-image.jpg');
    } catch {
      return 'browser-image.jpg';
    }
  }

  function mimeFromDataUrl(dataUrl) {
    const match = String(dataUrl || '').match(/^data:([^;]+);base64,/);
    return match?.[1] || 'image/png';
  }

  function promptForItem(item, target, lang) {
    if (!item) return '';
    const first = (...values) => values.find((value) => typeof value === 'string' && value.trim()) || '';
    if (target === 'nano') {
      return lang === 'zh'
        ? first(item.promptNanoBananaZh, item.promptNanoBananaEn, item.prompt_zh, item.prompt_en)
        : first(item.promptNanoBananaEn, item.promptNanoBananaZh, item.prompt_en, item.prompt_zh);
    }
    return lang === 'zh'
      ? first(item.promptGptImageZh, item.prompt_zh, item.promptGptImageEn, item.prompt_en)
      : first(item.promptGptImageEn, item.prompt_en, item.promptGptImageZh, item.prompt_zh);
  }

  function normalizeError(error) {
    const message = error?.message || String(error || '_unknown');
    if (/Failed to fetch|NetworkError|Load failed|fetch/i.test(message)) {
      return '\u65e0\u6cd5\u8fde\u63a5 EKO \u672c\u5730\u8f6f\u4ef6\u3002\u8bf7\u5148\u6253\u5f00 EKO\uff0c\u786e\u8ba4\u8bbe\u7f6e\u4e2d\u5fc3\u7684 API \u53ef\u7528\uff0c\u7136\u540e\u91cd\u8bd5\u3002';
    }
    if (/Request timed out|timeout|aborted/i.test(message)) {
      return '\u8fde\u63a5 EKO \u672c\u5730\u8f6f\u4ef6\u8d85\u65f6\u3002\u8bf7\u786e\u8ba4\u8f6f\u4ef6\u5df2\u6253\u5f00\uff0c\u5e76\u7a0d\u540e\u91cd\u8bd5\u3002';
    }
    return message === '_unknown' ? '\u672a\u77e5\u9519\u8bef' : message;
  }

  function escapeHtml(value) {
    return String(value ?? '')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;');
  }

  root.EKOExtensionUtils = { filenameFromUrl, mimeFromDataUrl, promptForItem, normalizeError, escapeHtml };
})(globalThis);
