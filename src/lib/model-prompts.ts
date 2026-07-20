import type { HistoryItem } from "@/lib/api";

export type PromptTarget = "gpt" | "nano";
export type PromptLang = "zh" | "en";

function nonEmpty(value: string | undefined | null) {
  return value?.trim() ? value : undefined;
}

export function getModelPrompt(item: HistoryItem | null | undefined, target: PromptTarget, lang: PromptLang) {
  if (!item) return "";
  if (target === "nano") {
    if (lang === "zh") return nonEmpty(item.promptNanoBananaZh) || nonEmpty(item.promptNanoBananaEn) || nonEmpty(item.prompt_zh) || nonEmpty(item.prompt_en) || "";
    return nonEmpty(item.promptNanoBananaEn) || nonEmpty(item.promptNanoBananaZh) || nonEmpty(item.prompt_en) || nonEmpty(item.prompt_zh) || "";
  }
  if (lang === "zh") return nonEmpty(item.promptGptImageZh) || nonEmpty(item.prompt_zh) || nonEmpty(item.promptGptImageEn) || nonEmpty(item.prompt_en) || "";
  return nonEmpty(item.promptGptImageEn) || nonEmpty(item.prompt_en) || nonEmpty(item.promptGptImageZh) || nonEmpty(item.prompt_zh) || "";
}

export function setModelPrompt(item: HistoryItem | null | undefined, target: PromptTarget, lang: PromptLang, value: string) {
  if (!item) return;
  if (target === "nano") {
    if (lang === "zh") item.promptNanoBananaZh = value;
    else item.promptNanoBananaEn = value;
    return;
  }
  if (lang === "zh") {
    item.promptGptImageZh = value;
    item.prompt_zh = value;
  } else {
    item.promptGptImageEn = value;
    item.prompt_en = value;
  }
}
