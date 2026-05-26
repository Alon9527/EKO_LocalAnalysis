const ASPECT_RATIOS = new Set(["1:1", "3:4", "4:3", "9:16", "16:9"]);
const NEGATIVE_PHRASES = [
  /\bno\s+[a-z]/i,
  /\bwithout\s+[a-z]/i,
  /\bexclude(?:d|s|ing)?\b/i,
  /\bexcluding\b/i,
  /\babsence of\b/i,
  /\bmissing\b/i,
];

export function buildInferenceInstruction(): string {
  return [
    "You are a senior vision prompt reverse-engineer for Google Gemini and Imagen workflows.",
    "Analyze the provided reference image and return ONLY valid JSON.",
    "Your job is single-pass reverse prompting for high-fidelity recreation readiness.",
    "",
    "Mandatory quality rules:",
    "1. Use positive framing only. Do not use negative constraints such as 'no', 'without', 'excluding', or 'absence of' in any prompt-bearing field.",
    "2. Optimize for Imagen-style prompting with dense visual detail, professional photography language, and explicit spatial relationships.",
    "3. Keep prompt_en in English. Keep prompt_zh as a faithful Chinese translation of prompt_en.",
    '4. If the image contains visible text, embedded_text_syntax must use this exact pattern: with the text "..." in a typography',
    "5. Embedded text must be 25 characters or fewer. If the source text is longer, keep only the most salient visible text fragment within 25 characters.",
    "6. aspect_ratio must be exactly one of: 1:1, 3:4, 4:3, 9:16, 16:9",
    "7. contains_people must be a boolean.",
    "8. Do not add markdown, comments, code fences, or conversational filler.",
    "9. Prefer concrete visual facts. If something is uncertain, place the uncertainty in quality_notes instead of weakening the main prompt.",
    "",
    "Output JSON schema:",
    "{",
    '  "aspect_ratio": "1:1|3:4|4:3|9:16|16:9",',
    '  "contains_people": true,',
    '  "reconstructed_prompt": {',
    '    "style_prefix": "string",',
    '    "subject": "string",',
    '    "context_and_background": "string",',
    '    "lighting": "string",',
    '    "camera_and_composition": "string",',
    '    "embedded_text_syntax": "string"',
    "  },",
    '  "quality_notes": ["string"],',
    '  "prompt_en": "string",',
    '  "prompt_zh": "string"',
    "}",
    "",
    "Composition rules:",
    "- style_prefix should establish medium/style first.",
    "- subject must describe the core subject precisely, including materials, colors, texture, shape, pose.",
    "- context_and_background must describe foreground, midground, background, environment.",
    "- lighting must describe direction, softness, intensity, color temperature.",
    "- camera_and_composition must describe lens feel, angle, framing, perspective, depth of field.",
    "- prompt_en should read like a production-ready single prompt assembled from the reconstructed_prompt fields.",
    "- prompt_en must stay under 480 tokens.",
    "Return JSON only.",
  ].join("\n");
}

export function buildRepairInstruction(opts: { previousText: string; failure: string }): string {
  return [
    "Repair the previous response and return ONLY valid JSON.",
    "Keep the same schema and all mandatory quality rules from the original task.",
    `Validation failure: ${opts.failure || "unknown failure"}`,
    "Do not explain the fix.",
    "Previous response to repair:",
    opts.previousText,
  ].join("\n");
}

export function getInferenceResponseSchema() {
  return {
    type: "OBJECT",
    required: ["aspect_ratio", "contains_people", "reconstructed_prompt", "quality_notes", "prompt_en", "prompt_zh"],
    properties: {
      aspect_ratio: { type: "STRING", enum: ["1:1", "3:4", "4:3", "9:16", "16:9"] },
      contains_people: { type: "BOOLEAN" },
      reconstructed_prompt: {
        type: "OBJECT",
        required: ["style_prefix", "subject", "context_and_background", "lighting", "camera_and_composition", "embedded_text_syntax"],
        properties: {
          style_prefix: { type: "STRING" },
          subject: { type: "STRING" },
          context_and_background: { type: "STRING" },
          lighting: { type: "STRING" },
          camera_and_composition: { type: "STRING" },
          embedded_text_syntax: { type: "STRING" },
        },
      },
      quality_notes: { type: "ARRAY", items: { type: "STRING" } },
      prompt_en: { type: "STRING" },
      prompt_zh: { type: "STRING" },
    },
  };
}

export interface ReconstructedPrompt {
  style_prefix: string;
  subject: string;
  context_and_background: string;
  lighting: string;
  camera_and_composition: string;
  embedded_text_syntax: string;
}

export interface PromptResult {
  aspect_ratio: string;
  contains_people: boolean;
  reconstructed_prompt: ReconstructedPrompt;
  quality_notes: string[];
  prompt_en: string;
  prompt_zh: string;
  _raw?: string;
}

export function parsePromptJson(rawText: string): PromptResult {
  if (!rawText) throw new Error("模型返回为空");
  const text = rawText.trim();
  try {
    return normalizePromptResult(JSON.parse(text), text);
  } catch {
    const start = text.indexOf("{");
    const end = text.lastIndexOf("}");
    if (start >= 0 && end > start) {
      return normalizePromptResult(JSON.parse(text.slice(start, end + 1)), text);
    }
    throw new Error("模型返回内容不是有效的 JSON");
  }
}

function normalizePromptResult(obj: any, raw: string): PromptResult {
  if (!obj || typeof obj !== "object") throw new Error("JSON 对象无效");

  const rp = obj.reconstructed_prompt && typeof obj.reconstructed_prompt === "object" ? obj.reconstructed_prompt : {};
  const result: PromptResult = {
    aspect_ratio: normalizeAspectRatio(obj.aspect_ratio),
    contains_people: normalizeBoolean(obj.contains_people),
    reconstructed_prompt: {
      style_prefix: cleanText(rp.style_prefix),
      subject: cleanText(rp.subject),
      context_and_background: cleanText(rp.context_and_background),
      lighting: cleanText(rp.lighting),
      camera_and_composition: cleanText(rp.camera_and_composition),
      embedded_text_syntax: normalizeEmbeddedTextSyntax(rp.embedded_text_syntax),
    },
    quality_notes: normalizeNotes(obj.quality_notes),
    prompt_en: cleanText(obj.prompt_en),
    prompt_zh: cleanText(obj.prompt_zh),
    _raw: raw,
  };

  if (!result.aspect_ratio) throw new Error("aspect_ratio 缺失或格式无效");

  const requiredFields: [string, string][] = [
    ["style_prefix", result.reconstructed_prompt.style_prefix],
    ["subject", result.reconstructed_prompt.subject],
    ["context_and_background", result.reconstructed_prompt.context_and_background],
    ["lighting", result.reconstructed_prompt.lighting],
    ["camera_and_composition", result.reconstructed_prompt.camera_and_composition],
  ];
  for (const [name, value] of requiredFields) {
    if (!value) throw new Error(`缺少必填字段：${name}`);
  }

  if (!result.prompt_en) result.prompt_en = composePrompt(result.reconstructed_prompt);
  if (!result.prompt_zh) result.prompt_zh = result.prompt_en;

  validatePromptResult(result);
  return result;
}

function validatePromptResult(result: PromptResult) {
  const fieldsToCheck = [
    result.reconstructed_prompt.style_prefix,
    result.reconstructed_prompt.subject,
    result.reconstructed_prompt.context_and_background,
    result.reconstructed_prompt.lighting,
    result.reconstructed_prompt.camera_and_composition,
    result.reconstructed_prompt.embedded_text_syntax,
    result.prompt_en,
  ].filter(Boolean);

  for (const value of fieldsToCheck) {
    if (containsNegativePrompting(value)) throw new Error("结果违反了正向描述约束");
  }

  const embeddedText = extractEmbeddedText(result.reconstructed_prompt.embedded_text_syntax);
  if (embeddedText == null && result.reconstructed_prompt.embedded_text_syntax) {
    throw new Error("embedded_text_syntax 不符合 Imagen 兼容格式");
  }
  if (embeddedText && embeddedText.length > 25) throw new Error("内嵌文字超过 25 个字符限制");
}

function composePrompt(rp: ReconstructedPrompt): string {
  return [rp.style_prefix, rp.subject, rp.context_and_background, rp.lighting, rp.camera_and_composition, rp.embedded_text_syntax]
    .map(cleanText)
    .filter(Boolean)
    .join(", ");
}

function normalizeAspectRatio(value: any): string {
  const clean = cleanText(value);
  if (ASPECT_RATIOS.has(clean)) return clean;
  const normalized = clean.replace(/\s+/g, "").replace(/[xX]/g, ":");
  if (ASPECT_RATIOS.has(normalized)) return normalized;
  return "";
}

function normalizeBoolean(value: any): boolean {
  if (typeof value === "boolean") return value;
  const text = cleanText(value).toLowerCase();
  if (["true", "yes", "1"].includes(text)) return true;
  if (["false", "no", "0"].includes(text)) return false;
  throw new Error("contains_people 必须是布尔值");
}

function normalizeNotes(value: any): string[] {
  if (!Array.isArray(value)) return [];
  return value.map((item: any) => cleanText(item)).filter(Boolean);
}

function cleanText(value: any): string {
  return String(value || "")
    .replace(/\s+/g, " ")
    .replace(/[“”]/g, '"')
    .replace(/[‘’]/g, "'")
    .trim();
}

function containsNegativePrompting(value: string): boolean {
  return NEGATIVE_PHRASES.some((pattern) => pattern.test(value));
}

function extractEmbeddedText(value: string): string | null {
  const text = cleanText(value);
  if (!text) return "";
  const match = text.match(/^with the text "([^"]*)" in a typography$/i);
  return match ? match[1] : null;
}

function normalizeEmbeddedTextSyntax(value: any): string {
  const text = cleanText(value);
  if (!text) return "";
  const lower = text.toLowerCase();
  if (["none", "n/a", "na", "null", "no text", "without text"].includes(lower)) return "";

  const exact = extractEmbeddedText(text);
  if (exact != null) return exact ? `with the text "${exact.slice(0, 25)}" in a typography` : "";

  const quoted = text.match(/"([^"]{1,80})"/);
  if (quoted?.[1]) return `with the text "${quoted[1].slice(0, 25)}" in a typography`;

  if (text.length <= 25 && text.split(/\s+/).length <= 6) {
    return `with the text "${text.slice(0, 25)}" in a typography`;
  }

  return "";
}

export function computeQualityScore(result: PromptResult) {
  const rp = result.reconstructed_prompt || ({} as ReconstructedPrompt);
  const warnings: string[] = [];

  const breakdown = {
    subject: scoreTextRichness(rp.subject, { minWords: 12, keywords: ["color", "texture", "material", "pose", "shape", "age", "face", "expression", "fabric", "surface"] }),
    context: scoreTextRichness(rp.context_and_background, { minWords: 14, keywords: ["foreground", "midground", "background", "behind", "around", "environment", "interior", "exterior", "surface", "spatial"] }),
    lighting: scoreTextRichness(rp.lighting, { minWords: 10, keywords: ["light", "shadow", "soft", "hard", "backlit", "rim", "diffused", "warm", "cool", "direction"] }),
    camera: scoreTextRichness(rp.camera_and_composition, { minWords: 10, keywords: ["lens", "depth", "field", "close-up", "wide", "framing", "angle", "composition", "perspective", "bokeh"] }),
    text: scoreEmbeddedText(rp.embedded_text_syntax),
    imagen: scoreImagenFit(result),
  };

  if (breakdown.subject < 70) warnings.push("主体细节偏弱");
  if (breakdown.context < 70) warnings.push("空间层次不足");
  if (breakdown.lighting < 68) warnings.push("光影描述偏弱");
  if (breakdown.camera < 68) warnings.push("镜头构图偏弱");
  if (breakdown.text < 80) warnings.push("文字字段待确认");
  if (breakdown.imagen < 75) warnings.push("Imagen 适配一般");

  const qualityNotes = Array.isArray(result.quality_notes) ? result.quality_notes.filter(Boolean) : [];
  if (qualityNotes.length) warnings.push(...qualityNotes.slice(0, 2));

  const weighted =
    breakdown.subject * 0.24 +
    breakdown.context * 0.18 +
    breakdown.lighting * 0.16 +
    breakdown.camera * 0.16 +
    breakdown.text * 0.08 +
    breakdown.imagen * 0.18;
  const total = Math.max(1, Math.min(100, Math.round(weighted)));

  const label = total >= 90 ? "很高" : total >= 78 ? "较强" : total >= 64 ? "可用" : total >= 45 ? "偏弱" : "较低";

  return { total, breakdown, warnings: total >= 86 ? [] : warnings.slice(0, 2), label };
}

function scoreTextRichness(text: string, opts: { minWords: number; keywords: string[] }): number {
  const clean = (text || "").trim();
  if (!clean) return 0;
  const words = clean.split(/\s+/).filter(Boolean);
  const lower = clean.toLowerCase();
  let score = 30;
  score += Math.min(35, Math.round((words.length / Math.max(1, opts.minWords)) * 35));
  const hits = opts.keywords.filter((kw) => lower.includes(kw)).length;
  score += Math.min(25, hits * 4);
  if (/[,:;]/.test(clean)) score += 5;
  if (/\b(and|with|featuring|showing|positioned|rendered)\b/i.test(clean)) score += 5;
  return Math.max(0, Math.min(100, score));
}

function scoreEmbeddedText(text: string): number {
  const clean = (text || "").trim();
  if (!clean) return 92;
  const match = clean.match(/^with the text "([^"]*)" in a typography$/i);
  if (!match) return 35;
  const content = match[1] || "";
  if (content.length > 25) return 40;
  if (!content.length) return 88;
  return 100;
}

function scoreImagenFit(result: PromptResult): number {
  let score = 0;
  if (["1:1", "3:4", "4:3", "9:16", "16:9"].includes(result.aspect_ratio)) score += 20;
  if (typeof result.contains_people === "boolean") score += 10;
  if (result.prompt_en?.trim()) score += 20;
  if (result.prompt_zh?.trim()) score += 5;
  const wordCount = (result.prompt_en || "").split(/\s+/).filter(Boolean).length;
  if (wordCount >= 30 && wordCount <= 220) score += 20;
  else if (wordCount >= 12 && wordCount <= 320) score += 12;
  else score += 4;
  const rp = result.reconstructed_prompt;
  const fields = [rp.style_prefix, rp.subject, rp.context_and_background, rp.lighting, rp.camera_and_composition];
  score += fields.every((v) => (v || "").trim()) ? 15 : 0;
  if ((rp.style_prefix || "").trim().split(/\s+/).length >= 2) score += 10;
  return Math.max(0, Math.min(100, score));
}
