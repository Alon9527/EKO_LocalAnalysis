import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { api, type MaterialAsset, type MaterialListResponse } from "@/lib/api";
import { useMaterialsStore } from "@/stores/materials";

function asset(id = "asset-1", name = "Chair"): MaterialAsset {
  return {
    id,
    category: "element",
    generatedName: name,
    generatedExplanation: `${name} explanation`,
    generatedPromptZh: `${name} zh`,
    generatedPromptEn: `${name} en`,
    generatedAliases: [],
    userOverride: {
      displayName: null,
      promptZh: null,
      promptEn: null,
      aliases: [],
      favorite: false,
      manuallyEdited: false,
      mergedInto: null,
      splitFrom: null,
      splitSourceIds: [],
    },
    sources: [],
    createdAt: 1,
    updatedAt: 1,
  };
}

function response(items: MaterialAsset[]): MaterialListResponse {
  return { items, total: items.length, stale: false, warnings: [] };
}

describe("materials store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("forwards active filters when loading materials", async () => {
    const item = asset();
    const list = vi.spyOn(api, "listMaterials").mockResolvedValue(response([item]));
    const store = useMaterialsStore();
    store.keyword = "chair";
    store.category = "element";
    store.favoriteOnly = true;
    store.minSources = 2;

    await store.load();

    expect(list).toHaveBeenCalledWith({
      keyword: "chair",
      category: "element",
      favorite: true,
      minSources: 2,
    });
    expect(store.items).toEqual([item]);
    expect(store.loading).toBe(false);
  });

  it("keeps the newest result when an older load finishes last", async () => {
    let resolveFirst!: (value: MaterialListResponse) => void;
    let resolveSecond!: (value: MaterialListResponse) => void;
    vi.spyOn(api, "listMaterials")
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));
    const store = useMaterialsStore();

    store.keyword = "old";
    const firstLoad = store.load();
    store.keyword = "new";
    const secondLoad = store.load();
    resolveSecond(response([asset("new", "New result")]));
    await secondLoad;
    expect(store.items[0].id).toBe("new");
    expect(store.loading).toBe(false);

    resolveFirst(response([asset("old", "Old result")]));
    await firstLoad;
    expect(store.items[0].id).toBe("new");
    expect(store.loading).toBe(false);
  });

  it("loads the breakdown for one history record", async () => {
    const item = asset();
    const getHistory = vi
      .spyOn(api, "getHistoryMaterials")
      .mockResolvedValue([item]);
    const store = useMaterialsStore();

    await store.loadForHistory("history-1");

    expect(getHistory).toHaveBeenCalledWith("history-1");
    expect(store.historyItems["history-1"]).toEqual([item]);
  });

  it("refreshes the selected asset and list after saving", async () => {
    const original = asset();
    const updated = {
      ...original,
      userOverride: { ...original.userOverride, favorite: true },
      updatedAt: 2,
    };
    vi.spyOn(api, "updateMaterial").mockResolvedValue(updated);
    const list = vi.spyOn(api, "listMaterials").mockResolvedValue(response([updated]));
    const store = useMaterialsStore();
    store.items = [original];
    store.openAsset(original.id);

    await store.saveAsset(original.id, { favorite: true });

    expect(api.updateMaterial).toHaveBeenCalledWith(original.id, { favorite: true });
    expect(list).toHaveBeenCalled();
    expect(store.selectedAsset).toEqual(updated);
    expect(store.items).toEqual([updated]);
  });

  it("refreshes the selected asset and list after merging", async () => {
    const merged = asset("asset-1", "Merged chair");
    vi.spyOn(api, "mergeMaterials").mockResolvedValue(merged);
    vi.spyOn(api, "listMaterials").mockResolvedValue(response([merged]));
    const store = useMaterialsStore();

    await store.mergeAssets(["asset-1", "asset-2"], "Merged chair");

    expect(api.mergeMaterials).toHaveBeenCalledWith(
      ["asset-1", "asset-2"],
      "Merged chair",
    );
    expect(store.selectedAsset).toEqual(merged);
    expect(store.items).toEqual([merged]);
  });

  it("refreshes the selected asset and list after splitting", async () => {
    const original = asset("asset-1", "Chair");
    const split = asset("asset-2", "Chair detail");
    vi.spyOn(api, "splitMaterial").mockResolvedValue([original, split]);
    vi.spyOn(api, "listMaterials").mockResolvedValue(response([original, split]));
    const store = useMaterialsStore();

    await store.splitAsset("asset-1", ["source-2"], "Chair detail");

    expect(api.splitMaterial).toHaveBeenCalledWith(
      "asset-1",
      ["source-2"],
      "Chair detail",
    );
    expect(store.selectedAsset).toEqual(original);
    expect(store.items).toEqual([original, split]);
  });

  it("reapplies active filters after a successful rebuild", async () => {
    const selected = asset("selected", "Selected");
    const filtered = asset("filtered", "Filtered");
    vi.spyOn(api, "rebuildMaterialIndex").mockResolvedValue(response([
      selected,
      filtered,
      asset("unfiltered", "Unfiltered"),
    ]));
    const list = vi.spyOn(api, "listMaterials").mockResolvedValue(response([filtered]));
    const store = useMaterialsStore();
    store.items = [selected];
    store.openAsset(selected.id);
    store.keyword = "filtered";

    await store.rebuild();

    expect(list).toHaveBeenCalledWith({ keyword: "filtered" });
    expect(store.items).toEqual([filtered]);
    expect(store.selectedAsset).toBeNull();
  });

  it("keeps the previous list and exposes an error when rebuild fails", async () => {
    const original = asset();
    vi.spyOn(api, "rebuildMaterialIndex").mockRejectedValue(new Error("disk full"));
    const store = useMaterialsStore();
    store.items = [original];

    await store.rebuild();

    expect(store.items).toEqual([original]);
    expect(store.error).toContain("disk full");
    expect(store.rebuilding).toBe(false);
  });

  it("rejects manual rebuild in browser preview", async () => {
    await expect(api.rebuildMaterialIndex()).rejects.toThrow();
  });
});
