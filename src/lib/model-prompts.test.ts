import { describe, expect, it } from "vitest";
import { getModelPrompt } from "@/lib/model-prompts";
import type { HistoryItem } from "@/lib/api";

function item(fields: Partial<HistoryItem>): HistoryItem {
  return fields as HistoryItem;
}

describe("getModelPrompt", () => {
  it("keeps GPT and Nano Banana prompts distinct", () => {
    const history = item({
      prompt_en: "legacy English",
      prompt_zh: "旧版中文",
      promptGptImageEn: "GPT English",
      promptGptImageZh: "GPT 中文",
      promptNanoBananaEn: "Nano English",
      promptNanoBananaZh: "Nano 中文",
    });

    expect(getModelPrompt(history, "gpt", "zh")).toBe("GPT 中文");
    expect(getModelPrompt(history, "nano", "zh")).toBe("Nano 中文");
    expect(getModelPrompt(history, "gpt", "en")).toBe("GPT English");
    expect(getModelPrompt(history, "nano", "en")).toBe("Nano English");
  });

  it("does not disguise a legacy prompt as a model-specific prompt", () => {
    const history = item({ prompt_en: "legacy English", prompt_zh: "旧版中文" });

    expect(getModelPrompt(history, "gpt", "zh")).toBe("");
    expect(getModelPrompt(history, "nano", "en")).toBe("");
  });

  it("does not fall back across languages", () => {
    const history = item({ promptGptImageEn: "GPT English", promptNanoBananaZh: "Nano 中文" });

    expect(getModelPrompt(history, "gpt", "zh")).toBe("");
    expect(getModelPrompt(history, "nano", "en")).toBe("");
  });
});