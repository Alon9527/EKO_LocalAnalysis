import { describe, expect, it } from "vitest";
import type { MaterialAsset, MaterialCategory } from "@/lib/api";
import {
  buildMaterialPacks,
  categoryCountsForPack,
  filterPackRowsByCategory,
} from "@/lib/materialPacks";

function asset(
  id: string,
  category: MaterialCategory,
  historyId: string,
  promptZh: string,
  options: { favorite?: boolean; fieldPath?: string; sourceId?: string } = {},
): MaterialAsset {
  return {
    id,
    category,
    generatedName: promptZh,
    generatedExplanation: "",
    generatedPromptZh: promptZh,
    generatedPromptEn: `${promptZh} EN`,
    generatedAliases: [],
    userOverride: {
      displayName: null,
      promptZh: null,
      promptEn: null,
      aliases: [],
      favorite: options.favorite || false,
      manuallyEdited: false,
      mergedInto: null,
      splitFrom: null,
      splitSourceIds: [],
    },
    sources: [
      {
        id: options.sourceId || `${id}-${historyId}`,
        historyId,
        thumbnailId: historyId,
        fieldPath: options.fieldPath || "entities[0].label",
        promptZh,
        promptEn: `${promptZh} EN`,
        createdAt: historyId === "history-new" ? 20 : 10,
      },
    ],
    createdAt: 1,
    updatedAt: 1,
  };
}

describe("material packs", () => {
  it("groups material fragments by source history image", () => {
    const packs = buildMaterialPacks([
      asset("chair", "element", "history-new", "休闲椅"),
      asset("wood", "material", "history-new", "胡桃木纹理"),
      asset("light", "lighting", "history-old", "柔和自然光"),
    ]);

    expect(packs).toHaveLength(2);
    expect(packs[0].id).toBe("history-new");
    expect(packs[0].rows.map((row) => row.asset.id)).toEqual(["chair", "wood"]);
    expect(packs[0].thumbnailId).toBe("history-new");
  });

  it("deduplicates rows from the same asset and source", () => {
    const repeated = asset("chair", "element", "history-new", "休闲椅", {
      sourceId: "same-source",
    });
    repeated.sources.push({ ...repeated.sources[0] });

    const packs = buildMaterialPacks([repeated]);

    expect(packs).toHaveLength(1);
    expect(packs[0].rows).toHaveLength(1);
  });

  it("sorts packs by newest source and counts categories", () => {
    const packs = buildMaterialPacks([
      asset("old", "element", "history-old", "旧椅子"),
      asset("new", "camera", "history-new", "平视广角"),
      asset("new-light", "lighting", "history-new", "窗边漫射光"),
    ]);

    expect(packs.map((pack) => pack.id)).toEqual(["history-new", "history-old"]);
    expect(categoryCountsForPack(packs[0])).toEqual({
      camera: 1,
      lighting: 1,
    });
  });

  it("filters rows by category while keeping all rows for all", () => {
    const [pack] = buildMaterialPacks([
      asset("chair", "element", "history-new", "休闲椅"),
      asset("wood", "material", "history-new", "胡桃木纹理"),
    ]);

    expect(filterPackRowsByCategory(pack, "all")).toHaveLength(2);
    expect(filterPackRowsByCategory(pack, "material").map((row) => row.asset.id)).toEqual([
      "wood",
    ]);
  });
});