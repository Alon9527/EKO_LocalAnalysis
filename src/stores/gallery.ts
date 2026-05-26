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

  async function load() {
    loading.value = true;
    try {
      const query: HistoryQuery = { pageSize: 200 };
      if (keyword.value) query.keyword = keyword.value;
      if (minScore.value) query.minScore = minScore.value;
      if (favOnly.value) query.favorite = true;
      const data = await api.getHistory(query);
      items.value = data.items || [];
      // load thumbnails for each item asynchronously
      for (const item of items.value) {
        if (item.thumbUrl) continue;
        if (item.sourceType === "url" && item.imageUrl) {
          item.thumbUrl = item.imageUrl;
        } else if (item.filePath) {
          api.readFileAsDataUrl(item.filePath)
            .then((url) => { item.thumbUrl = url; })
            .catch(() => {});
        }
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

  async function toggleFavorite(id: string) {
    const newState = await api.toggleFavorite(id);
    const item = items.value.find((i) => i.id === id);
    if (item) item.favorite = newState;
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

  function openDetail(id: string) {
    detailItem.value = items.value.find((i) => i.id === id) || null;
  }

  function closeDetail() {
    detailItem.value = null;
  }

  return {
    items, selected, keyword, minScore, favOnly, detailItem, loading,
    selectedCount, load, toggleSelect, toggleFavorite, deleteSelected,
    clearAll, openDetail, closeDetail,
  };
});
