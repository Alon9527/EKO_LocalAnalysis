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

  it('prefers model-specific Nano Banana prompts with legacy fallback', async () => {
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
    expect(utils.promptForItem({ prompt_en: 'legacy en' }, 'nano', 'en')).toBe('legacy en');
  });

  it('normalizes low-level fetch errors into actionable connection guidance', async () => {
    const utils = await loadUtils();
    const message = utils.normalizeError(new Error('Failed to fetch'));
    expect(message).toContain('EKO');
    expect(message).toContain('API');
    expect(message).not.toContain('Failed to fetch');
  });
});
