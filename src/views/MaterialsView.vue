<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  Check,
  CopyDocument,
  MoreFilled,
  Picture,
  Refresh,
  Search,
  Star,
  StarFilled,
} from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import { api, type MaterialCategory } from "@/lib/api";
import { MATERIAL_CATEGORY_LABELS, materialDisplayName } from "@/lib/materials";
import {
  buildMaterialPacks,
  categoryCountsForPack,
  filterPackRowsByCategory,
  type MaterialPack,
  type MaterialPackRow,
} from "@/lib/materialPacks";
import { useMaterialsStore } from "@/stores/materials";

const store = useMaterialsStore();
const route = useRoute();
const router = useRouter();
const menuOpen = ref(false);
const selectedCategory = ref<MaterialCategory | "all">("all");
const thumbnails = reactive<Record<string, string>>({});
const thumbnailUnavailable = reactive<Record<string, boolean>>({});
const copiedRowId = ref("");
let searchTimer: ReturnType<typeof setTimeout> | undefined;
let thumbnailRequest = 0;

const categoryTabs: Array<{ value: MaterialCategory | "all"; label: string }> = [
  { value: "all", label: "全部" },
  ...Object.entries(MATERIAL_CATEGORY_LABELS).map(([value, label]) => ({
    value: value as MaterialCategory,
    label,
  })),
];

const packs = computed(() => buildMaterialPacks(store.items));
const selectedPackId = computed(() => {
  const value = route.query.pack;
  return typeof value === "string" ? value : "";
});
const selectedPack = computed(() => (
  packs.value.find((pack) => pack.id === selectedPackId.value) || packs.value[0] || null
));
const selectedRows = computed(() => (
  selectedPack.value ? filterPackRowsByCategory(selectedPack.value, selectedCategory.value) : []
));
const totalFragments = computed(() => packs.value.reduce((sum, pack) => sum + pack.rows.length, 0));

function syncSelectionFromRoute() {
  const id = selectedPackId.value;
  if (id && packs.value.some((pack) => pack.id === id)) return;
  if (!packs.value.length) return;
  router.replace({ query: { ...route.query, pack: packs.value[0].id } });
}

onMounted(async () => {
  await store.load();
  syncSelectionFromRoute();
});

watch(() => route.query.pack, syncSelectionFromRoute);
watch(packs, syncSelectionFromRoute);

watch(
  packs,
  (currentPacks) => {
    const requestId = ++thumbnailRequest;
    for (const pack of currentPacks) {
      loadThumbnail(pack, requestId);
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (searchTimer) window.clearTimeout(searchTimer);
  thumbnailRequest += 1;
});

function scheduleSearch() {
  if (searchTimer) window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => store.load(), 300);
}

function setCategory(category: MaterialCategory | "all") {
  store.category = category;
  selectedCategory.value = category;
  store.load();
}

function setFavorite(event: Event) {
  store.favoriteOnly = (event.target as HTMLInputElement).checked;
  store.load();
}

function openPack(id: string) {
  router.replace({ query: { ...route.query, pack: id } });
}

async function toggleFavorite(id: string, favorite: boolean) {
  await store.setAssetFavorite(id, favorite);
}

async function rebuildIndex() {
  menuOpen.value = false;
  const rebuilt = await store.rebuild();
  if (rebuilt && store.error) {
    ElMessage.warning(`素材索引已重建，但列表刷新失败：${store.error}`);
  } else if (rebuilt) ElMessage.success("素材索引已重建");
  else ElMessage.error(store.error || "重建素材索引失败");
}

async function loadThumbnail(pack: MaterialPack, requestId: number) {
  if (thumbnails[pack.thumbnailId] || thumbnailUnavailable[pack.thumbnailId]) return;
  try {
    const dataUrl = await api.readThumbnailAsDataUrl(pack.thumbnailId);
    if (requestId === thumbnailRequest) thumbnails[pack.thumbnailId] = dataUrl;
  } catch {
    if (requestId === thumbnailRequest) thumbnailUnavailable[pack.thumbnailId] = true;
  }
}

function countsLabel(pack: MaterialPack) {
  const counts = categoryCountsForPack(pack);
  return pack.categories
    .slice(0, 4)
    .map((category) => `${MATERIAL_CATEGORY_LABELS[category]} ${counts[category] || 0}`)
    .join(" / ");
}

async function copyRow(row: MaterialPackRow) {
  const text = row.source.promptEn
    ? `${row.source.promptZh}\n\n${row.source.promptEn}`
    : row.source.promptZh;
  try {
    await navigator.clipboard.writeText(text);
    copiedRowId.value = row.source.id;
    ElMessage.success("片段已复制");
    window.setTimeout(() => {
      if (copiedRowId.value === row.source.id) copiedRowId.value = "";
    }, 1200);
  } catch {
    ElMessage.error("复制失败");
  }
}
</script>

<template>
  <div class="materials-view">
    <header class="materials-toolbar">
      <div class="materials-toolbar__title">
        <div>
          <h1>素材库</h1>
          <p>按图片整理可复用 Prompt 片段，再从元素、材质、镜头、光影里查找来源。</p>
        </div>
        <div class="library-stats" aria-label="素材库统计">
          <strong>{{ packs.length }}</strong>
          <span>图片素材包</span>
          <strong>{{ totalFragments }}</strong>
          <span>Prompt 片段</span>
        </div>
      </div>

      <div class="materials-toolbar__controls">
        <label class="search-field">
          <el-icon :size="16"><Search /></el-icon>
          <input
            v-model="store.keyword"
            data-testid="materials-search"
            type="search"
            placeholder="搜索物体、材质、角度、光影 Prompt"
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

      <div v-else-if="packs.length" class="materials-workspace">
        <section class="pack-list" aria-label="图片素材包">
          <button
            v-for="pack in packs"
            :key="pack.id"
            data-testid="material-pack"
            type="button"
            class="pack-card"
            :class="{ 'is-active': selectedPack?.id === pack.id }"
            @click="openPack(pack.id)"
          >
            <div class="pack-card__thumb">
              <img v-if="thumbnails[pack.thumbnailId]" :src="thumbnails[pack.thumbnailId]" alt="" />
              <div v-else class="pack-card__missing">
                <el-icon><Picture /></el-icon>
              </div>
            </div>
            <div class="pack-card__body">
              <div class="pack-card__meta">
                <span>{{ pack.sourceCount }} 个片段</span>
                <span v-if="pack.favorite">已收藏</span>
              </div>
              <h2>{{ pack.title }}</h2>
              <p>{{ countsLabel(pack) }}</p>
            </div>
          </button>
        </section>

        <section v-if="selectedPack" class="pack-detail" aria-label="素材包详情">
          <header class="pack-detail__header">
            <div>
              <span class="eyebrow">图片素材包</span>
              <h2>{{ selectedPack.title }}</h2>
              <p>{{ selectedPack.sourceCount }} 个可复用片段，来自同一张历史图片。</p>
            </div>
            <RouterLink
              :to="{ path: '/gallery', query: { history: selectedPack.id } }"
              class="history-link"
            >
              查看原图
            </RouterLink>
          </header>

          <div class="detail-tabs" role="tablist" aria-label="详情分类">
            <button
              v-for="tab in categoryTabs"
              :key="tab.value"
              type="button"
              :class="{ 'is-active': selectedCategory === tab.value }"
              @click="selectedCategory = tab.value"
            >
              {{ tab.label }}
            </button>
          </div>

          <div class="fragment-list">
            <article
              v-for="row in selectedRows"
              :key="`${row.asset.id}-${row.source.id}`"
              class="fragment-row"
            >
              <div class="fragment-row__body">
                <div class="fragment-row__title">
                  <span>{{ MATERIAL_CATEGORY_LABELS[row.asset.category] }}</span>
                  <strong>{{ materialDisplayName(row.asset) }}</strong>
                </div>
                <p>{{ row.source.promptZh }}</p>
                <p v-if="row.source.promptEn" class="fragment-row__en">{{ row.source.promptEn }}</p>
                <code>{{ row.source.fieldPath }}</code>
              </div>
              <div class="fragment-row__actions">
                <button
                  :data-testid="`copy-${row.source.id}`"
                  type="button"
                  class="icon-tool"
                  :title="copiedRowId === row.source.id ? '已复制' : '复制片段'"
                  @click="copyRow(row)"
                >
                  <el-icon>
                    <Check v-if="copiedRowId === row.source.id" />
                    <CopyDocument v-else />
                  </el-icon>
                </button>
                <button
                  :data-testid="`favorite-${row.asset.id}`"
                  type="button"
                  class="icon-tool"
                  :class="{ 'is-favorite': row.asset.userOverride.favorite }"
                  :title="row.asset.userOverride.favorite ? '取消收藏' : '收藏片段'"
                  @click="toggleFavorite(row.asset.id, !row.asset.userOverride.favorite)"
                >
                  <el-icon>
                    <StarFilled v-if="row.asset.userOverride.favorite" />
                    <Star v-else />
                  </el-icon>
                </button>
              </div>
            </article>
          </div>
        </section>
      </div>

      <section v-else class="materials-empty">
        <div class="materials-empty__mark">EKO</div>
        <h2>素材库还没有内容</h2>
        <p>新的结构化分析结果会自动整理成图片素材包。</p>
        <RouterLink to="/single">开始分析图片</RouterLink>
      </section>
    </main>
  </div>
</template>

<style scoped>
.materials-view {
  min-height: 100%;
  padding: 34px clamp(24px, 4vw, 56px);
  color: rgba(255, 255, 255, 0.9);
}

.materials-toolbar {
  display: grid;
  gap: 22px;
  margin-bottom: 26px;
}

.materials-toolbar__title {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
}

.materials-toolbar__title h1 {
  margin: 0 0 7px;
  color: rgba(255, 255, 255, 0.95);
  font-size: 30px;
  line-height: 1.15;
}

.materials-toolbar__title p {
  margin: 0;
  color: rgba(255, 255, 255, 0.55);
  font-size: 14px;
  line-height: 1.6;
}

.library-stats {
  display: grid;
  grid-template-columns: auto auto;
  gap: 2px 10px;
  min-width: 170px;
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  padding-left: 18px;
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
}

.library-stats strong {
  color: #5eead4;
  font-size: 18px;
  line-height: 1;
}

.library-stats span {
  align-self: center;
  white-space: nowrap;
}

.materials-toolbar__controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}

.search-field {
  display: flex;
  min-width: 260px;
  flex: 1 1 360px;
  align-items: center;
  gap: 9px;
  height: 42px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  padding: 0 13px;
  color: rgba(255, 255, 255, 0.45);
}

.search-field input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: 0;
  background: transparent;
  color: rgba(255, 255, 255, 0.9);
  font: inherit;
}

.search-field input::placeholder {
  color: rgba(255, 255, 255, 0.32);
}

.compact-check {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 7px;
  height: 42px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.035);
  padding: 0 12px;
  color: rgba(255, 255, 255, 0.62);
  font-size: 13px;
  white-space: nowrap;
}

.compact-check input {
  accent-color: #2dd4bf;
}

.library-menu {
  position: relative;
  flex: 0 0 auto;
}

.icon-tool {
  display: inline-flex;
  width: 38px;
  height: 38px;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
}

.icon-tool:hover {
  border-color: rgba(45, 212, 191, 0.38);
  color: #5eead4;
}

.icon-tool.is-favorite {
  color: #fbbf24;
}

.library-menu__popup {
  position: absolute;
  z-index: 10;
  top: 46px;
  right: 0;
  min-width: 180px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(17, 20, 28, 0.98);
  padding: 6px;
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.32);
}

.library-menu__popup button {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  padding: 9px 10px;
  color: rgba(255, 255, 255, 0.78);
  cursor: pointer;
  text-align: left;
}

.library-menu__popup button:hover {
  background: rgba(45, 212, 191, 0.1);
  color: #5eead4;
}

.category-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.category-tabs button {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.04);
  padding: 8px 13px;
  color: rgba(255, 255, 255, 0.56);
  font-size: 13px;
  cursor: pointer;
}

.category-tabs button.is-active,
.category-tabs button:hover {
  border-color: rgba(45, 212, 191, 0.32);
  background: rgba(20, 184, 166, 0.14);
  color: #5eead4;
}

.materials-content {
  min-width: 0;
}

.materials-message {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 14px;
  border-radius: 8px;
  padding: 11px 13px;
  font-size: 13px;
}

.materials-message--error {
  border: 1px solid rgba(248, 113, 113, 0.28);
  background: rgba(127, 29, 29, 0.24);
  color: #fecaca;
}

.materials-message--warning {
  border: 1px solid rgba(251, 191, 36, 0.22);
  background: rgba(120, 53, 15, 0.2);
  color: #fde68a;
}

.materials-message button {
  border: 0;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.1);
  padding: 6px 10px;
  color: inherit;
  cursor: pointer;
}

.materials-skeleton {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 14px;
}

.materials-skeleton span {
  min-height: 178px;
  border-radius: 8px;
  background: linear-gradient(90deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.04));
}

.materials-workspace {
  display: grid;
  grid-template-columns: minmax(300px, 0.38fr) minmax(0, 1fr);
  gap: 18px;
  align-items: start;
}

.pack-list {
  display: grid;
  max-height: calc(100vh - 250px);
  gap: 10px;
  overflow: auto;
  padding-right: 4px;
}

.pack-card {
  display: grid;
  grid-template-columns: 104px minmax(0, 1fr);
  gap: 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(18, 21, 28, 0.78);
  padding: 10px;
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 160ms ease, background-color 160ms ease;
}

.pack-card:hover,
.pack-card.is-active {
  border-color: rgba(45, 212, 191, 0.46);
  background: rgba(22, 31, 36, 0.92);
}

.pack-card__thumb {
  aspect-ratio: 4 / 3;
  overflow: hidden;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
}

.pack-card__thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.pack-card__missing {
  display: grid;
  width: 100%;
  height: 100%;
  place-items: center;
  color: rgba(255, 255, 255, 0.32);
}

.pack-card__body {
  min-width: 0;
}

.pack-card__meta {
  display: flex;
  gap: 8px;
  margin-bottom: 6px;
  color: rgba(94, 234, 212, 0.72);
  font-size: 11px;
}

.pack-card h2 {
  overflow: hidden;
  margin: 0 0 7px;
  color: rgba(255, 255, 255, 0.9);
  font-size: 14px;
  line-height: 1.45;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pack-card p {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: rgba(255, 255, 255, 0.48);
  font-size: 12px;
  line-height: 1.55;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.pack-detail {
  min-width: 0;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(14, 17, 24, 0.72);
}

.pack-detail__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  padding: 18px;
}

.eyebrow {
  color: #5eead4;
  font-size: 12px;
  font-weight: 650;
}

.pack-detail__header h2 {
  margin: 5px 0 7px;
  color: rgba(255, 255, 255, 0.94);
  font-size: 20px;
  line-height: 1.35;
}

.pack-detail__header p {
  margin: 0;
  color: rgba(255, 255, 255, 0.5);
  font-size: 13px;
}

.history-link {
  flex: 0 0 auto;
  border: 1px solid rgba(45, 212, 191, 0.32);
  border-radius: 8px;
  padding: 9px 12px;
  color: #5eead4;
  font-size: 13px;
  text-decoration: none;
}

.detail-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 14px 18px;
}

.detail-tabs button {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.04);
  padding: 7px 11px;
  color: rgba(255, 255, 255, 0.58);
  cursor: pointer;
}

.detail-tabs button.is-active {
  border-color: rgba(45, 212, 191, 0.38);
  background: rgba(20, 184, 166, 0.16);
  color: #5eead4;
}

.fragment-list {
  display: grid;
  gap: 10px;
  padding: 0 18px 18px;
}

.fragment-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.035);
  padding: 13px;
}

.fragment-row__title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-bottom: 7px;
}

.fragment-row__title span {
  color: #5eead4;
  font-size: 12px;
  font-weight: 650;
}

.fragment-row__title strong {
  color: rgba(255, 255, 255, 0.9);
  font-size: 14px;
}

.fragment-row p {
  margin: 0;
  color: rgba(255, 255, 255, 0.76);
  font-size: 13px;
  line-height: 1.65;
}

.fragment-row__en {
  margin-top: 8px !important;
  color: rgba(255, 255, 255, 0.52) !important;
}

.fragment-row code {
  display: inline-block;
  margin-top: 8px;
  color: rgba(255, 255, 255, 0.34);
  font-size: 11px;
}

.fragment-row__actions {
  display: flex;
  gap: 6px;
}

.materials-empty {
  display: grid;
  min-height: 440px;
  place-items: center;
  border: 1px dashed rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  padding: 36px;
  text-align: center;
}

.materials-empty__mark {
  display: grid;
  width: 58px;
  height: 58px;
  place-items: center;
  border-radius: 8px;
  background: rgba(45, 212, 191, 0.12);
  color: #5eead4;
  font-weight: 800;
}

.materials-empty h2 {
  margin: 14px 0 8px;
  font-size: 20px;
}

.materials-empty p {
  margin: 0 0 16px;
  color: rgba(255, 255, 255, 0.5);
}

.materials-empty a {
  border-radius: 8px;
  background: #2dd4bf;
  padding: 10px 14px;
  color: #041311;
  font-weight: 700;
  text-decoration: none;
}

@media (max-width: 1180px) {
  .materials-toolbar__title {
    align-items: flex-start;
    flex-direction: column;
  }

  .materials-workspace {
    grid-template-columns: 1fr;
  }

  .pack-list {
    max-height: none;
  }
}

@media (max-width: 760px) {
  .materials-view {
    padding: 24px 16px;
  }

  .search-field {
    min-width: 100%;
  }

  .pack-card,
  .fragment-row {
    grid-template-columns: 1fr;
  }

  .pack-detail__header {
    display: grid;
  }
}
</style>
