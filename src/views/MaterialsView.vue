<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  MoreFilled,
  Refresh,
  Search,
  Star,
} from "@element-plus/icons-vue";
import type { MaterialCategory, MaterialPatch } from "@/lib/api";
import { MATERIAL_CATEGORY_LABELS } from "@/lib/materials";
import { useMaterialsStore } from "@/stores/materials";
import MaterialAssetCard from "@/components/materials/MaterialAssetCard.vue";
import MaterialDetailDrawer from "@/components/materials/MaterialDetailDrawer.vue";

const store = useMaterialsStore();
const route = useRoute();
const router = useRouter();
const menuOpen = ref(false);
let searchTimer: ReturnType<typeof setTimeout> | undefined;

const categoryTabs: Array<{ value: MaterialCategory | "all"; label: string }> = [
  { value: "all", label: "全部" },
  ...Object.entries(MATERIAL_CATEGORY_LABELS).map(([value, label]) => ({
    value: value as MaterialCategory,
    label,
  })),
];

function syncSelectionFromRoute() {
  const id = typeof route.query.asset === "string" ? route.query.asset : "";
  if (!id) {
    store.closeAsset();
    return;
  }
  if (store.selectedAsset?.id === id) return;
  store.openAsset(id);
  if (store.selectedAsset) store.loadMergeCandidates(store.selectedAsset.category);
}

onMounted(async () => {
  await store.load();
  syncSelectionFromRoute();
});

watch(() => route.query.asset, syncSelectionFromRoute);

onBeforeUnmount(() => {
  if (searchTimer) window.clearTimeout(searchTimer);
});

function scheduleSearch() {
  if (searchTimer) window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => store.load(), 300);
}

function setCategory(category: MaterialCategory | "all") {
  store.category = category;
  store.load();
}

function setFavorite(event: Event) {
  store.favoriteOnly = (event.target as HTMLInputElement).checked;
  store.load();
}

function setMinSources(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  store.minSources = value ? Number(value) : undefined;
  store.load();
}

function openAsset(id: string) {
  store.openAsset(id);
  if (store.selectedAsset) store.loadMergeCandidates(store.selectedAsset.category);
  router.replace({ query: { ...route.query, asset: id } });
}

function closeAsset() {
  store.closeAsset();
  const query = { ...route.query };
  delete query.asset;
  router.replace({ query });
}

async function saveAsset(id: string, patch: MaterialPatch) {
  await store.saveAsset(id, patch);
}

async function mergeAssets(ids: string[], displayName?: string) {
  const merged = await store.mergeAssets(ids, displayName);
  if (!merged) return;
  await store.loadMergeCandidates(merged.category);
  router.replace({ query: { ...route.query, asset: merged.id } });
}

async function splitAsset(id: string, sourceIds: string[], displayName: string) {
  await store.splitAsset(id, sourceIds, displayName);
}

async function toggleFavorite(id: string, favorite: boolean) {
  await store.setAssetFavorite(id, favorite);
}

async function rebuildIndex() {
  menuOpen.value = false;
  await store.rebuild();
}
</script>

<template>
  <div class="materials-view">
    <header class="materials-toolbar">
      <div class="materials-toolbar__title">
        <div>
          <h1>素材库</h1>
          <p>从结构化分析中整理可复用的元素、材质、镜头与光影片段。</p>
        </div>
      </div>

      <div class="materials-toolbar__controls">
        <label class="search-field">
          <el-icon :size="16"><Search /></el-icon>
          <input
            v-model="store.keyword"
            data-testid="materials-search"
            type="search"
            placeholder="搜索名称、别名或 Prompt"
            @input="scheduleSearch"
          />
        </label>

        <label class="compact-check">
          <input
            data-testid="favorite-only"
            type="checkbox"
            :checked="store.favoriteOnly"
            @change="setFavorite"
          />
          <el-icon :size="15"><Star /></el-icon>
          <span>收藏</span>
        </label>

        <label class="select-field">
          <span>来源数</span>
          <select
            data-testid="min-sources"
            :value="store.minSources ?? ''"
            @change="setMinSources"
          >
            <option value="">不限</option>
            <option value="2">至少 2</option>
            <option value="3">至少 3</option>
            <option value="5">至少 5</option>
          </select>
        </label>

        <div class="library-menu">
          <button
            data-testid="open-library-menu"
            type="button"
            class="icon-tool"
            title="更多操作"
            @click="menuOpen = !menuOpen"
          >
            <el-icon :size="18"><MoreFilled /></el-icon>
          </button>
          <div v-if="menuOpen" class="library-menu__popup">
            <button
              data-testid="rebuild-index"
              type="button"
              :disabled="store.rebuilding"
              @click="rebuildIndex"
            >
              <el-icon :size="15"><Refresh /></el-icon>
              {{ store.rebuilding ? "正在重建" : "重建素材索引" }}
            </button>
          </div>
        </div>
      </div>

      <nav class="category-tabs" aria-label="素材分类">
        <button
          v-for="tab in categoryTabs"
          :key="tab.value"
          :data-testid="`category-${tab.value}`"
          type="button"
          :class="{ 'is-active': store.category === tab.value }"
          @click="setCategory(tab.value)"
        >
          {{ tab.label }}
        </button>
      </nav>
    </header>

    <main class="materials-content">
      <div v-if="store.error" class="materials-message materials-message--error">
        <span>{{ store.error }}</span>
        <button type="button" @click="store.load()">重试</button>
      </div>
      <div v-if="store.warnings.length" class="materials-message materials-message--warning">
        {{ store.warnings.length }} 条历史记录暂未建立素材索引，其他素材仍可正常使用。
      </div>

      <div v-if="store.loading && !store.items.length" class="materials-skeleton" aria-label="正在加载素材">
        <span v-for="index in 8" :key="index" />
      </div>

      <div v-else-if="store.items.length" class="materials-grid">
        <MaterialAssetCard
          v-for="asset in store.items"
          :key="asset.id"
          :asset="asset"
          @open="openAsset"
          @toggle-favorite="toggleFavorite"
        />
      </div>

      <section v-else class="materials-empty">
        <div class="materials-empty__mark">EKO</div>
        <h2>素材库还没有内容</h2>
        <p>新的结构化分析结果会自动出现在这里。</p>
        <RouterLink to="/single">开始分析图片</RouterLink>
      </section>
    </main>

    <MaterialDetailDrawer
      :model-value="!!store.selectedAsset"
      :asset="store.selectedAsset"
      :candidates="store.mergeCandidates"
      @update:model-value="$event ? undefined : closeAsset()"
      @save="saveAsset"
      @merge="mergeAssets"
      @split="splitAsset"
      @toggle-favorite="toggleFavorite"
    />
  </div>
</template>

<style scoped>
.materials-view {
  min-height: 100%;
  color: rgba(255, 255, 255, 0.9);
}

.materials-toolbar {
  position: sticky;
  z-index: 5;
  top: 0;
  display: grid;
  grid-template-columns: minmax(220px, 1fr) auto;
  gap: 14px 22px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(10, 12, 17, 0.92);
  padding: 18px 24px 12px;
  backdrop-filter: blur(16px);
}

.materials-toolbar__title h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 680;
  letter-spacing: 0;
}

.materials-toolbar__title p {
  margin: 5px 0 0;
  color: rgba(255, 255, 255, 0.42);
  font-size: 12px;
}

.materials-toolbar__controls {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.search-field,
.compact-check,
.select-field,
.icon-tool {
  height: 36px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.035);
}

.search-field {
  display: flex;
  width: min(330px, 30vw);
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  color: rgba(255, 255, 255, 0.38);
}

.search-field:focus-within {
  border-color: rgba(45, 212, 191, 0.55);
}

.search-field input {
  width: 100%;
  border: 0;
  outline: 0;
  background: transparent;
  color: rgba(255, 255, 255, 0.86);
  font-size: 13px;
}

.compact-check,
.select-field {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  color: rgba(255, 255, 255, 0.58);
  font-size: 12px;
}

.compact-check input {
  width: 14px;
  height: 14px;
  accent-color: #2dd4bf;
}

.select-field select {
  border: 0;
  outline: 0;
  background: #171a21;
  color: rgba(255, 255, 255, 0.76);
  font-size: 12px;
}

.library-menu {
  position: relative;
}

.icon-tool {
  display: inline-flex;
  width: 36px;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.58);
  cursor: pointer;
}

.library-menu__popup {
  position: absolute;
  z-index: 8;
  top: 42px;
  right: 0;
  width: 174px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 7px;
  background: #171a21;
  padding: 5px;
  box-shadow: 0 14px 38px rgba(0, 0, 0, 0.44);
}

.library-menu__popup button {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 8px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  padding: 9px;
  color: rgba(255, 255, 255, 0.72);
  font-size: 12px;
  cursor: pointer;
}

.library-menu__popup button:hover {
  background: rgba(255, 255, 255, 0.06);
}

.category-tabs {
  display: flex;
  grid-column: 1 / -1;
  gap: 4px;
  overflow-x: auto;
}

.category-tabs button {
  min-width: 52px;
  height: 30px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  padding: 0 11px;
  color: rgba(255, 255, 255, 0.48);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.category-tabs button:hover {
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.76);
}

.category-tabs button.is-active {
  background: rgba(45, 212, 191, 0.13);
  color: #5eead4;
}

.materials-content {
  padding: 18px 24px 34px;
}

.materials-grid,
.materials-skeleton {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
}

.materials-skeleton span {
  min-height: 184px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  background: linear-gradient(90deg, rgba(255,255,255,0.025), rgba(255,255,255,0.06), rgba(255,255,255,0.025));
  background-size: 220% 100%;
  animation: material-shimmer 1.4s linear infinite;
}

.materials-message {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  border: 1px solid;
  border-radius: 7px;
  padding: 9px 11px;
  font-size: 12px;
}

.materials-message button {
  border: 0;
  background: transparent;
  color: currentColor;
  font-weight: 650;
  cursor: pointer;
}

.materials-message--error {
  border-color: rgba(248, 113, 113, 0.28);
  background: rgba(127, 29, 29, 0.16);
  color: #fca5a5;
}

.materials-message--warning {
  border-color: rgba(251, 191, 36, 0.24);
  background: rgba(120, 53, 15, 0.12);
  color: #fcd34d;
}

.materials-empty {
  display: flex;
  min-height: 420px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

.materials-empty__mark {
  display: flex;
  width: 54px;
  height: 54px;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(45, 212, 191, 0.28);
  border-radius: 8px;
  background: rgba(45, 212, 191, 0.08);
  color: #5eead4;
  font-size: 13px;
  font-weight: 750;
}

.materials-empty h2 {
  margin: 16px 0 0;
  font-size: 17px;
  letter-spacing: 0;
}

.materials-empty p {
  margin: 7px 0 15px;
  color: rgba(255, 255, 255, 0.42);
  font-size: 13px;
}

.materials-empty a {
  color: #5eead4;
  font-size: 13px;
  text-decoration: none;
}

@keyframes material-shimmer {
  to { background-position: -120% 0; }
}

@media (max-width: 980px) {
  .materials-toolbar {
    grid-template-columns: 1fr;
  }

  .materials-toolbar__controls {
    justify-content: flex-start;
    flex-wrap: wrap;
  }

  .search-field {
    width: min(100%, 360px);
  }
}

@media (max-width: 620px) {
  .materials-toolbar,
  .materials-content {
    padding-right: 14px;
    padding-left: 14px;
  }

  .materials-grid,
  .materials-skeleton {
    grid-template-columns: 1fr;
  }
}
</style>
