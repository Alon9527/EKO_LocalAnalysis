import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { api, type HistoryItem, type HistoryQuery } from "@/lib/api";

export const useGalleryStore = defineStore("gallery", () => {
  const items = ref<HistoryItem[]>([]);
  const selected = ref<Set<string>>(new Set());
  const keyword = ref("");
  const minScore = ref<number | undefined>(undefined);
  const favOnly = ref(false);
  const detailItem = ref<HistoryItem | null>(null);
  const loading = ref(false);

  const selectedCount = computed(() => selected.value.size);
  let detailRequestId = 0;

  async function load() {
    loading.value = true;
    try {
      const query: HistoryQuery = { pageSize: 200 };
      if (keyword.value) query.keyword = keyword.value;
      if (minScore.value) query.minScore = minScore.value;
      if (favOnly.value) query.favorite = true;
      const data = await api.getHistory(query);
      items.value = data.items || [];
      // Load thumbnails asynchronously. Dragged/pasted images may not have a
      // stable file path, so fall back to the cached thumbnail saved by Tauri.
      for (const item of items.value) {
        loadThumbnail(item);
      }
    } catch {
      items.value = [];
    }
    selected.value = new Set();
    loading.value = false;
  }

  function toggleSelect(id: string) {
    const next = new Set(selected.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected.value = next;
  }

  function selectAll() {
    selected.value = new Set(items.value.map((item) => item.id));
  }

  function clearSelection() {
    selected.value = new Set();
  }

  function loadCachedThumbnail(item: HistoryItem) {
    api.readThumbnailAsDataUrl(item.id)
      .then((url) => { item.thumbUrl = url; })
      .catch(() => {});
  }

  function loadThumbnail(item: HistoryItem) {
    if (item.thumbUrl) return;
    if (item.sourceType === "url" && item.imageUrl) {
      item.thumbUrl = item.imageUrl;
    } else if (item.filePath) {
      api.readFileAsDataUrl(item.filePath)
        .then((url) => { item.thumbUrl = url; })
        .catch(() => loadCachedThumbnail(item));
    } else {
      loadCachedThumbnail(item);
    }
  }

  async function toggleFavorite(id: string) {
    const newState = await api.toggleFavorite(id);
    const item = items.value.find((i) => i.id === id);
    if (item) item.favorite = newState;
    if (detailItem.value?.id === id) detailItem.value.favorite = newState;
  }

  async function deleteSelected() {
    const ids = Array.from(selected.value);
    if (!ids.length) return;
    await api.deleteHistory(ids);
    selected.value = new Set();
    await load();
  }

  async function clearAll() {
    await api.clearHistory();
    await load();
  }

  async function openDetail(id: string) {
    const requestId = ++detailRequestId;
    let found = items.value.find((item) => item.id === id) || null;
    if (!found) {
      try {
        const data = await api.getHistory({ id, page: 1, pageSize: 1 });
        found = data.items.find((item) => item.id === id) || null;
        if (found) loadThumbnail(found);
      } catch {
        found = null;
      }
    }
    if (requestId !== detailRequestId) return detailItem.value;
    detailItem.value = found;
    return found;
  }

  function closeDetail() {
    detailRequestId += 1;
    detailItem.value = null;
  }

  return {
    items, selected, keyword, minScore, favOnly, detailItem, loading,
    selectedCount, load, toggleSelect, toggleFavorite, deleteSelected,
    selectAll, clearSelection, clearAll, openDetail, closeDetail,
  };
});
