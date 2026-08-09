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

  it('rejects short Chinese summaries even when English labels exist', async () => {
    const utils = await loadUtils();
    const error = utils.validatePromptContract({
      promptGptImageEn: 'OUTPUT FRAME: 16:9. CAMERA: eye level. FIXED LAYOUT: foreground. APPEARANCE: realistic. LIGHTING: daylight. INVARIANTS: one subject.',
      promptGptImageZh: '一个明亮的厨房场景。',
      promptNanoBananaEn: 'FRAME AND CAMERA: 16:9. EXACT SPATIAL LAYOUT: foreground. MATERIALS AND COLOR: realistic. LIGHT AND ATMOSPHERE: daylight. LOCKED CONDITIONS: one subject.',
      promptNanoBananaZh: '一个明亮的厨房场景。',
    });

    expect(error).toContain('GPT 中文');
    expect(error).toContain('Nano 中文');
  });

  it('accepts the complete desktop four-prompt contract', async () => {
    const utils = await loadUtils();
    const error = utils.validatePromptContract({
      promptGptImageEn: 'OUTPUT FRAME: 16:9 landscape. CAMERA: eye-level wide-angle view. FIXED LAYOUT: foreground, midground, and background keep exact object count, position, scale, overlap, and spacing. APPEARANCE: photorealistic colors, shapes, materials, and texture. LIGHTING: soft directional daylight with balanced highlights and shadows. INVARIANTS: preserve subjects, boundaries, proportions, open areas, and uncluttered surfaces. ' + 'Detailed reconstruction wording. '.repeat(12),
      promptGptImageZh: '输出画幅：横向16:9画幅并锁定裁切。相机：平视广角机位，保留透视和地平线。固定布局：逐区描述前景中景背景的数量位置尺度遮挡间距。外观：还原颜色形状材质纹理。光线：柔和定向日光。 不变量：固定主体边界比例开放区域和简洁表面。' + '继续保留每个可见物体的准确空间关系和真实表面细节。'.repeat(6),
      promptNanoBananaEn: 'FRAME AND CAMERA: 16:9 landscape with an eye-level wide-angle camera. EXACT SPATIAL LAYOUT: reconstruct foreground, midground, and background counts, positions, scale, orientation, overlap, spacing, and boundaries. MATERIALS AND COLOR: reproduce visible colors, forms, materials, and texture. LIGHT AND ATMOSPHERE: soft directional daylight with balanced reflections and shadows. LOCKED CONDITIONS: keep subjects, boundaries, proportions, open areas, and uncluttered surfaces fixed. ' + 'Standalone reconstruction wording. '.repeat(12),
      promptNanoBananaZh: '画幅与相机：横向16:9画面，采用平视广角机位并锁定裁切透视。精确空间布局：逐区重建前景中景背景的数量位置尺度朝向遮挡间距边界。材质与色彩：还原可见颜色形状材质纹理。光线与氛围：柔和定向日光，平衡反射高光阴影。锁定条件：固定主体边界比例开放区域和简洁表面。' + '继续仅凭文字锁定每个可见物体的准确空间关系和真实表面细节。'.repeat(6),
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