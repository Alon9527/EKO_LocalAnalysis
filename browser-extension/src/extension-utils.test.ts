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

  function completePromptItem() {
    return {
      promptGptImageEn: 'A photorealistic commercial interior photograph of a calm modern kitchen intended for an architectural portfolio. The scene contains one central cooking island wrapped in veined white stone, one parallel sink island behind it, and a continuous wall of warm walnut cabinetry with two integrated steel ovens. Use wide landscape framing from an eye-level viewpoint, with the cooking island occupying the center-right foreground, the sink island offset behind it toward the left, and the window wall extending across the background. Show crisp stone veining, fine wood grain, brushed metal, clear glass, clean counter edges, and controlled natural reflections. Bright side daylight creates soft-edged shadows, cool white highlights, warm brown midtones, and a serene open atmosphere. Keep the exact two-island arrangement, open walkway, cabinet geometry, window boundaries, and uncluttered counters; do not add furniture, text, logos, or a watermark.',
      promptGptImageZh: '一幅用于建筑作品集的高真实感现代厨房商业室内摄影。场景中有一座包覆白色纹理石材的中央烹饪岛、一座位于其后并与其平行的水槽岛，以及沿墙连续排列的暖色胡桃木橱柜和两台嵌入式钢制烤箱。采用横向宽幅构图和平视视点，烹饪岛占据前景中央偏右，水槽岛向左错位位于其后，窗墙横跨背景。清晰呈现石材纹理、细密木纹、拉丝金属、透明玻璃、整洁台面边缘和受控自然反射。明亮侧向日光形成柔边阴影、冷白高光、暖棕中间调和宁静开放的氛围。保持两座岛台的准确排列、开放通道、橱柜几何、窗墙边界与简洁台面，不添加家具、文字、标志或水印。',
      promptNanoBananaEn: 'Create a photorealistic 16:9 architectural portfolio image of a calm high-end modern kitchen with a broad eye-level field of view, straight vertical lines, and deep focus. Show exactly one rectangular cooking island in the center-right foreground, wrapped on the top and both visible ends in white stone with long gray veins; integrate one brushed-steel gas range into its rear half. Place exactly one parallel sink island behind it toward the left, leaving a clearly visible walkway between the two islands. Across the right background, build one continuous wall of warm walnut cabinetry with two vertically stacked steel ovens and a matching stone backsplash. Extend a floor-to-ceiling window wall across the left and rear background, keeping its dark frames evenly spaced. Render crisp stone veining, directional wood grain, brushed metal, clear glazing, and subtle surface reflections. Use bright natural side light with soft shadow edges, balanced warm brown and cool white tones, and a quiet airy mood. Keep the scene limited to these two islands and the fixed cabinetry, maintain the exact offsets and open floor area, keep the counters clean, and preserve continuous architectural boundaries.',
      promptNanoBananaZh: '生成一幅用于建筑作品集的高真实感16:9高端现代厨房画面，采用宽广的平视取景、端正垂直线和深景深。前景中央偏右准确放置一座长方形烹饪岛，台面和两个可见端面包覆带长灰色纹理的白色石材，在岛台后半部嵌入一台拉丝钢燃气灶。其后偏左准确放置一座与之平行的水槽岛，两座岛台之间保留清晰可见的通道。右侧背景沿墙建立一组连续的暖色胡桃木橱柜，包含两台上下排列的钢制烤箱和同材质石材背板。落地窗墙横跨左侧与后方背景，深色窗框等距排列。清晰渲染石材纹理、定向木纹、拉丝金属、透明玻璃和细微表面反射。使用明亮自然侧光、柔和阴影边缘、平衡的暖棕与冷白色调以及安静通透的氛围。画面保持两座岛台和固定橱柜这一单一连贯布局，维持准确错位、开放地面、简洁台面和连续建筑边界。',
    };
  }

  it('rejects short Chinese summaries', async () => {
    const utils = await loadUtils();
    const item = completePromptItem();
    item.promptGptImageZh = '一个明亮的厨房场景。';
    item.promptNanoBananaZh = '一个明亮的厨房场景。';

    const error = utils.validatePromptContract(item);
    expect(error).toContain('GPT 中文仍是摘要');
    expect(error).toContain('Nano 中文仍是摘要');
  });

  it('rejects visible legacy template headings', async () => {
    const utils = await loadUtils();
    const item = completePromptItem();
    item.promptGptImageEn = 'OUTPUT FRAME: 16:9. ' + item.promptGptImageEn;

    const error = utils.validatePromptContract(item);
    expect(error).toContain('旧模板标题');
    expect(error).toContain('OUTPUT FRAME');
  });

  it('accepts detailed natural desktop prompts', async () => {
    const utils = await loadUtils();
    expect(utils.validatePromptContract(completePromptItem())).toBe('');
  });
  it('normalizes low-level fetch errors into actionable connection guidance', async () => {
    const utils = await loadUtils();
    const message = utils.normalizeError(new Error('Failed to fetch'));
    expect(message).toContain('EKO');
    expect(message).toContain('API');
    expect(message).not.toContain('Failed to fetch');
  });
});