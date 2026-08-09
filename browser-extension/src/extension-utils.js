(function (root) {
  const GPT_SECTIONS = ['OUTPUT FRAME', 'CAMERA', 'FIXED LAYOUT', 'APPEARANCE', 'LIGHTING', 'INVARIANTS'];
  const NANO_SECTIONS = ['FRAME AND CAMERA', 'EXACT SPATIAL LAYOUT', 'MATERIALS AND COLOR', 'LIGHT AND ATMOSPHERE', 'LOCKED CONDITIONS'];
  const GPT_ZH_SECTIONS = ['输出画幅', '相机', '固定布局', '外观', '光线', '不变量'];
  const NANO_ZH_SECTIONS = ['画幅与相机', '精确空间布局', '材质与色彩', '光线与氛围', '锁定条件'];

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
    const gptZh = nonEmpty(item?.promptGptImageZh);
    const nanoEn = nonEmpty(item?.promptNanoBananaEn);
    const nanoZh = nonEmpty(item?.promptNanoBananaZh);
    const missingGptSections = gptEn ? GPT_SECTIONS.filter((section) => !gptEn.includes(section)) : [];
    const missingNanoSections = nanoEn ? NANO_SECTIONS.filter((section) => !nanoEn.includes(section)) : [];
    const missingGptZhSections = gptZh ? GPT_ZH_SECTIONS.filter((section) => !gptZh.includes(section)) : [];
    const missingNanoZhSections = nanoZh ? NANO_ZH_SECTIONS.filter((section) => !nanoZh.includes(section)) : [];

    if (missingGptSections.length) issues.push('GPT sections: ' + missingGptSections.join(', '));
    if (missingNanoSections.length) issues.push('Nano sections: ' + missingNanoSections.join(', '));
    if (gptEn && gptEn.trim().split(/\s+/).length < 70) issues.push('GPT English 过短');
    if (nanoEn && nanoEn.trim().split(/\s+/).length < 70) issues.push('Nano English 过短');
    if (missingGptZhSections.length) issues.push('GPT 中文段落: ' + missingGptZhSections.join('、'));
    if (missingNanoZhSections.length) issues.push('Nano 中文段落: ' + missingNanoZhSections.join('、'));
    if (gptZh && (gptZh.match(/\S/g) || []).length < 140) issues.push('GPT 中文过短');
    if (nanoZh && (nanoZh.match(/\S/g) || []).length < 140) issues.push('Nano 中文过短');
    if (!issues.length) return '';

    return '插件收到旧版或不完整的 Prompt 格式。请确认 EKO 桌面端已更新到 v1.4.3 或更高版本，然后重新分析。问题：' + issues.join('; ');
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