(function (root) {
  const FORBIDDEN_HEADINGS = [
    'OUTPUT FRAME', 'FIXED LAYOUT', 'LOCKED CONDITIONS',
    'SCENE & PURPOSE', 'RENDERING INTENT',
    '输出画幅：', '固定布局：', '锁定条件：', '场景与用途：', '渲染意图：',
  ];
  const REFERENCE_DEPENDENCIES = [
    'reference image', 'based on the reference', 'refer to the image',
    '参考图', '保持原图', '与原图一致', '如图',
  ];
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

    if (gptEn && gptEn.trim().split(/\s+/).length < 100) issues.push('GPT English 仍是摘要');
    if (nanoEn && nanoEn.trim().split(/\s+/).length < 120) issues.push('Nano English 仍是摘要');
    if (gptZh && (gptZh.match(/\S/g) || []).length < 180) issues.push('GPT 中文仍是摘要');
    if (nanoZh && (nanoZh.match(/\S/g) || []).length < 220) issues.push('Nano 中文仍是摘要');

    if (nanoEn && !/^(create|generate|produce)\s/i.test(nanoEn.trim())) {
      issues.push('Nano Prompt 必须以 Create、Generate 或 Produce 开头');
    }

    for (const prompt of [gptEn, gptZh, nanoEn, nanoZh].filter(Boolean)) {
      const upper = prompt.toUpperCase();
      const lower = prompt.toLowerCase();
      const headings = FORBIDDEN_HEADINGS.filter((heading) => upper.includes(heading.toUpperCase()));
      const dependencies = REFERENCE_DEPENDENCIES.filter((phrase) => lower.includes(phrase.toLowerCase()));
      if (headings.length) issues.push('包含旧模板标题: ' + headings.join(', '));
      if (dependencies.length) issues.push('依赖不可用参考图: ' + dependencies.join(', '));
    }

    if (gptEn && gptEn === nanoEn) issues.push('GPT 与 Nano English 内容相同');
    if (gptZh && gptZh === nanoZh) issues.push('GPT 与 Nano 中文内容相同');
    if (!issues.length) return '';

    return '插件收到摘要级或旧版 Prompt。请确认 EKO 桌面端已更新，然后使用较强视觉模型重新分析。问题：' + issues.join('; ');
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