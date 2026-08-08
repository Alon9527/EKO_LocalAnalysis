import { beforeEach, describe, expect, it } from 'vitest';

async function loadUtils() {
  delete (globalThis as any).EKOExtensionUtils;
  await import('./extension-utils.js?test=' + Date.now());
  return (globalThis as any).EKOExtensionUtils;
}

describe('browser extension utils', () => {
  beforeEach(() => {
    delete (globalThis as any).EKOExtensionUtils;
  });

  it('returns only model-specific prompts without legacy fallback', async () => {
    const utils = await loadUtils();
    const item = {
      prompt_en: 'legacy en',
      prompt_zh: 'legacy zh',
      promptGptImageEn: 'gpt en',
      promptGptImageZh: 'gpt zh',
      promptNanoBananaEn: 'nano en',
      promptNanoBananaZh: 'nano zh',
    };

    expect(utils.promptForItem(item, 'nano', 'en')).toBe('nano en');
    expect(utils.promptForItem(item, 'nano', 'zh')).toBe('nano zh');
    expect(utils.promptForItem({ prompt_en: 'legacy en' }, 'nano', 'en')).toBe('');
    expect(utils.promptForItem({ prompt_zh: 'legacy zh' }, 'gpt', 'zh')).toBe('');
  });

  it('rejects incomplete or legacy prompt contracts', async () => {
    const utils = await loadUtils();
    const error = utils.validatePromptContract({
      prompt_en: 'legacy en',
      prompt_zh: 'legacy zh',
    });

    expect(error).toContain('promptGptImageEn');
    expect(error).toContain('promptNanoBananaEn');
  });

  it('accepts the desktop dual-model prompt contract', async () => {
    const utils = await loadUtils();
    const error = utils.validatePromptContract({
      promptGptImageEn: 'OUTPUT FRAME: 16:9. CAMERA: eye level. FIXED LAYOUT: foreground. APPEARANCE: realistic. LIGHTING: daylight. INVARIANTS: one subject.',
      promptGptImageZh: '完整 GPT 中文提示词',
      promptNanoBananaEn: 'FRAME AND CAMERA: 16:9. EXACT SPATIAL LAYOUT: foreground. MATERIALS AND COLOR: realistic. LIGHT AND ATMOSPHERE: daylight. LOCKED CONDITIONS: one subject.',
      promptNanoBananaZh: '完整 Nano 中文提示词',
    });

    expect(error).toBe('');
  });

  it('normalizes low-level fetch errors into actionable connection guidance', async () => {
    const utils = await loadUtils();
    const message = utils.normalizeError(new Error('Failed to fetch'));
    expect(message).toContain('EKO');
    expect(message).toContain('API');
    expect(message).not.toContain('Failed to fetch');
  });
});