import { describe, expect, it } from "vitest";
import type { MaterialAsset } from "@/lib/api";
import {
  MATERIAL_CATEGORY_LABELS,
  groupMaterialsByCategory,
  materialDisplayName,
  materialPromptEn,
  materialPromptZh,
} from "@/lib/materials";

describe("groupMaterialsByCategory", () => {
  it("groups assets without changing their order", () => {
    const assets = [
      { id: "light-1", category: "lighting" },
      { id: "element-1", category: "element" },
      { id: "light-2", category: "lighting" },
    ] as any[];

    expect(groupMaterialsByCategory(assets)).toEqual({
      lighting: [assets[0], assets[2]],
      element: [assets[1]],
    });
  });
});

describe("material resolved values", () => {
  const asset = {
    id: "material-1",
    category: "material",
    generatedName: "Leather",
    generatedExplanation: "Fine leather grain",
    generatedPromptZh: "\u7ec6\u817b\u76ae\u9769\u7eb9\u7406",
    generatedPromptEn: "fine leather grain",
    generatedAliases: ["leather"],
    userOverride: {
      displayName: "Caramel leather",
      promptZh: "\u7126\u7cd6\u8272\u76ae\u9769",
      promptEn: "caramel leather",
      aliases: [],
      favorite: true,
      manuallyEdited: true,
    },
    sources: [],
    createdAt: 1,
    updatedAt: 2,
  } satisfies MaterialAsset;

  it("provides labels for every material category", () => {
    expect(Object.keys(MATERIAL_CATEGORY_LABELS)).toEqual([
      "element", "material", "color", "lighting", "camera",
      "composition", "style", "environment",
    ]);
    expect(MATERIAL_CATEGORY_LABELS).toMatchObject({
      element: "\u5143\u7d20",
      material: "\u6750\u8d28",
      color: "\u8272\u5f69",
      lighting: "\u5149\u5f71",
      camera: "\u955c\u5934",
      composition: "\u6784\u56fe",
      style: "\u98ce\u683c",
      environment: "\u73af\u5883",
    });
  });

  it("prefers user overrides and falls back to generated prompts", () => {
    expect(materialDisplayName(asset)).toBe("Caramel leather");
    expect(materialPromptZh(asset)).toBe("\u7126\u7cd6\u8272\u76ae\u9769");
    expect(materialPromptEn(asset)).toBe("caramel leather");
    expect(materialPromptEn({
      ...asset,
      userOverride: { ...asset.userOverride, promptEn: undefined },
    })).toBe("fine leather grain");
  });
});
