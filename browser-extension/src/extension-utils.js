(function (root) {
  const GPT_SECTIONS = ['OUTPUT FRAME', 'CAMERA', 'FIXED LAYOUT', 'APPEARANCE', 'LIGHTING', 'INVARIANTS'];
  const NANO_SECTIONS = ['FRAME AND CAMERA', 'EXACT SPATIAL LAYOUT', 'MATERIALS AND COLOR', 'LIGHT AND ATMOSPHERE', 'LOCKED CONDITIONS'];

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

  function nonEmpty(value) {
    return typeof value === 'string' && value.trim() ? value : '';
  }

  function promptForItem(item, target, lang) {
    if (!item) return '';
    if (target === 'nano') {
      return lang === 'zh'
        ? nonEmpty(item.promptNanoBananaZh)
        : nonEmpty(item.promptNanoBananaEn);
    }
    return lang === 'zh'
      ? nonEmpty(item.promptGptImageZh)
      : nonEmpty(item.promptGptImageEn);
  }

  function validatePromptContract(item) {
    const required = [
      ['promptGptImageEn', item?.promptGptImageEn],
      ['promptGptImageZh', item?.promptGptImageZh],
      ['promptNanoBananaEn', item?.promptNanoBananaEn],
      ['promptNanoBananaZh', item?.promptNanoBananaZh],
    ];
    const issues = required
      .filter(([, value]) => !nonEmpty(value))
      .map(([field]) => field);

    const gptEn = nonEmpty(item?.promptGptImageEn);
    const nanoEn = nonEmpty(item?.promptNanoBananaEn);
    const missingGptSections = gptEn ? GPT_SECTIONS.filter((section) => !gptEn.includes(section)) : [];
    const missingNanoSections = nanoEn ? NANO_SECTIONS.filter((section) => !nanoEn.includes(section)) : [];

    if (missingGptSections.length) issues.push('GPT sections: ' + missingGptSections.join(', '));
    if (missingNanoSections.length) issues.push('Nano sections: ' + missingNanoSections.join(', '));
    if (!issues.length) return '';

    return '插件收到旧版或不完整的 Prompt 格式。请确认 EKO 桌面端已更新到 v1.4.2 或更高版本，然后重新分析。缺少：' + issues.join('; ');
  }

  function normalizeError(error) {
    const message = error?.message || String(error || 'unknown');
    if (/Failed to fetch|NetworkError|Load failed|fetch/i.test(message)) {
      return '无法连接 EKO 本地软件。请先打开 EKO，确认设置中心的 API 可用，然后重试。';
    }
    if (/Request timed out|timeout|aborted/i.test(message)) {
      return '连接 EKO 本地软件超时。请确认软件已打开，并稍后重试。';
    }
    return message === 'unknown' ? '未知错误' : message;
  }

  function escapeHtml(value) {
    return String(value ?? '')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;');
  }

  root.EKOExtensionUtils = {
    filenameFromUrl,
    mimeFromDataUrl,
    promptForItem,
    validatePromptContract,
    normalizeError,
    escapeHtml,
  };
})(globalThis);