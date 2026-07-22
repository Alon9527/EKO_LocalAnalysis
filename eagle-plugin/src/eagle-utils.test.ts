import { describe, expect, it } from "vitest";
import { buildAnalyzePayload, mimeFromPath, promptForItem, normalizeError } from "./eagle-utils.mjs";

describe("Eagle plugin utils", () => {
  it("builds a base64 analyze payload from a selected Eagle item", () => {
    const payload = buildAnalyzePayload(
      { id: "eagle-item-1", name: "Kitchen", ext: "jpg", filePath: "C:/images/kitchen.jpg" },
      Buffer.from("image-bytes"),
    );

    expect(payload.fileName).toBe("Kitchen.jpg");
    expect(payload.mimeType).toBe("image/jpeg");
    expect(payload.base64Data).toBe(Buffer.from("image-bytes").toString("base64"));
    expect(payload.id).toContain("eagle-item-1");
  });

  it("prefers model-specific Nano Banana prompts with legacy fallback", () => {
    const item = {
      prompt_en: "legacy en",
      prompt_zh: "legacy zh",
      promptGptImageEn: "gpt en",
      promptGptImageZh: "gpt zh",
      promptNanoBananaEn: "nano en",
      promptNanoBananaZh: "nano zh",
    };

    expect(promptForItem(item, "nano", "zh")).toBe("nano zh");
    expect(promptForItem(item, "gpt", "en")).toBe("gpt en");
    expect(promptForItem({ prompt_zh: "legacy zh" }, "nano", "zh")).toBe("legacy zh");
  });

  it("normalizes file mime types and bridge connection errors", () => {
    expect(mimeFromPath("demo.webp")).toBe("image/webp");
    expect(mimeFromPath("demo.jpeg")).toBe("image/jpeg");
    expect(normalizeError(new Error("ECONNREFUSED"))).toContain("EKO");
  });
});