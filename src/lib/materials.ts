import type { MaterialAsset, MaterialCategory } from "@/lib/api";

export const MATERIAL_CATEGORY_LABELS: Record<MaterialCategory, string> = {
  element: "\u5143\u7d20",
  material: "\u6750\u8d28",
  color: "\u8272\u5f69",
  lighting: "\u5149\u5f71",
  camera: "\u955c\u5934",
  composition: "\u6784\u56fe",
  style: "\u98ce\u683c",
  environment: "\u73af\u5883",
};

export function groupMaterialsByCategory<T extends { category: string }>(assets: T[]) {
  return assets.reduce<Record<string, T[]>>((groups, asset) => {
    (groups[asset.category] ||= []).push(asset);
    return groups;
  }, {});
}

export function materialDisplayName(asset: MaterialAsset) {
  return asset.userOverride.displayName || asset.generatedName;
}

export function materialPromptZh(asset: MaterialAsset) {
  return asset.userOverride.promptZh || asset.generatedPromptZh;
}

export function materialPromptEn(asset: MaterialAsset) {
  return asset.userOverride.promptEn || asset.generatedPromptEn || "";
}
