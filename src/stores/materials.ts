import { defineStore } from "pinia";
import { ref } from "vue";
import {
  api,
  type MaterialAsset,
  type MaterialCategory,
  type MaterialIndexWarning,
  type MaterialPatch,
  type MaterialQuery,
} from "@/lib/api";

function errorText(error: unknown) {
  if (error instanceof Error) return error.message;
  return String(error || "Material operation failed");
}

export const useMaterialsStore = defineStore("materials", () => {
  const items = ref<MaterialAsset[]>([]);
  const historyItems = ref<Record<string, MaterialAsset[]>>({});
  const selectedAsset = ref<MaterialAsset | null>(null);
  const mergeCandidates = ref<MaterialAsset[]>([]);
  const keyword = ref("");
  const category = ref<MaterialCategory | "all">("all");
  const favoriteOnly = ref(false);
  const minSources = ref<number | undefined>();
  const loading = ref(false);
  const rebuilding = ref(false);
  const error = ref("");
  const warnings = ref<MaterialIndexWarning[]>([]);
  let latestLoad = 0;
  let latestHistoryLoad = 0;
  let latestCandidateLoad = 0;

  function currentQuery(): MaterialQuery {
    const query: MaterialQuery = {};
    const normalizedKeyword = keyword.value.trim();
    if (normalizedKeyword) query.keyword = normalizedKeyword;
    if (category.value !== "all") query.category = category.value;
    if (favoriteOnly.value) query.favorite = true;
    if (minSources.value !== undefined) query.minSources = minSources.value;
    return query;
  }

  async function load() {
    const requestId = ++latestLoad;
    loading.value = true;
    error.value = "";
    try {
      const response = await api.listMaterials(currentQuery());
      if (requestId !== latestLoad) return false;
      items.value = response.items;
      warnings.value = response.warnings;
      return true;
    } catch (caught) {
      if (requestId === latestLoad) error.value = errorText(caught);
      return false;
    } finally {
      if (requestId === latestLoad) loading.value = false;
    }
  }

  async function loadForHistory(historyId: string) {
    const requestId = ++latestHistoryLoad;
    error.value = "";
    try {
      const assets = await api.getHistoryMaterials(historyId);
      if (requestId !== latestHistoryLoad) return false;
      historyItems.value[historyId] = assets;
      return true;
    } catch (caught) {
      if (requestId === latestHistoryLoad) error.value = errorText(caught);
      return false;
    }
  }
  async function loadMergeCandidates(materialCategory: MaterialCategory) {
    const requestId = ++latestCandidateLoad;
    try {
      const response = await api.listMaterials({ category: materialCategory });
      if (requestId !== latestCandidateLoad) return;
      mergeCandidates.value = response.items;
    } catch {
      if (requestId !== latestCandidateLoad) return;
      mergeCandidates.value = items.value.filter(
        (item) => item.category === materialCategory,
      );
    }
  }


  function openAsset(id: string) {
    selectedAsset.value = items.value.find((item) => item.id === id) || null;
  }

  function closeAsset() {
    selectedAsset.value = null;
  }

  async function saveAsset(id: string, patch: MaterialPatch) {
    error.value = "";
    try {
      const updated = await api.updateMaterial(id, patch);
      await load();
      selectedAsset.value = updated;
      return updated;
    } catch (caught) {
      error.value = errorText(caught);
      return null;
    }
  }

  async function setAssetFavorite(id: string, favorite: boolean) {
    error.value = "";
    try {
      const updated = await api.updateMaterial(id, { favorite });
      await load();
      const currentSelection = selectedAsset.value;
      if (currentSelection?.id === id) {
        selectedAsset.value = updated;
      } else if (currentSelection) {
        selectedAsset.value = items.value.find(
          (item) => item.id === currentSelection.id,
        ) || currentSelection;
      }
      return updated;
    } catch (caught) {
      error.value = errorText(caught);
      return null;
    }
  }

  async function mergeAssets(ids: string[], displayName?: string) {
    error.value = "";
    try {
      const merged = await api.mergeMaterials(ids, displayName);
      await load();
      selectedAsset.value = merged;
      return merged;
    } catch (caught) {
      error.value = errorText(caught);
      return null;
    }
  }

  async function splitAsset(
    id: string,
    sourceIds: string[],
    displayName: string,
  ) {
    error.value = "";
    try {
      const changed = await api.splitMaterial(id, sourceIds, displayName);
      await load();
      selectedAsset.value = changed.find((item) => item.id === id) || changed[0] || null;
      return changed;
    } catch (caught) {
      error.value = errorText(caught);
      return [];
    }
  }

  async function rebuild() {
    rebuilding.value = true;
    error.value = "";
    try {
      await api.rebuildMaterialIndex();
      const loaded = await load();
      if (loaded && selectedAsset.value) {
        selectedAsset.value = items.value.find(
          (item) => item.id === selectedAsset.value?.id,
        ) || null;
      }
      return true;
    } catch (caught) {
      error.value = errorText(caught);
      return false;
    } finally {
      rebuilding.value = false;
    }
  }

  return {
    items,
    historyItems,
    selectedAsset,
    keyword,
    category,
    favoriteOnly,
    minSources,
    loading,
    mergeCandidates,
    rebuilding,
    error,
    warnings,
    load,
    loadForHistory,
    openAsset,
    closeAsset,
    saveAsset,
    setAssetFavorite,
    mergeAssets,
    splitAsset,
    loadMergeCandidates,
    rebuild,
  };
});
