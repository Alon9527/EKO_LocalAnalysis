<script setup lang="ts">
import { computed, ref, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useGalleryStore } from "@/stores/gallery";
import { api } from "@/lib/api";
import RadarChart from "@/components/RadarChart.vue";
import AnalysisBreakdown from "@/components/materials/AnalysisBreakdown.vue";
import { getModelPrompt, type PromptTarget } from "@/lib/model-prompts";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  Search, Star, StarFilled, Download, Delete, Close, CopyDocument, Check, Upload,
  Grid, List, RefreshLeft, Picture as PicIcon
} from "@element-plus/icons-vue";

const store = useGalleryStore();
const route = useRoute();
const router = useRouter();
const viewMode = ref<"grid" | "list">("grid");
const modelFilter = ref("");
const dateFilter = ref("");
const scoreFilter = ref<number | "">("");
const copiedField = ref("");
const promptTab = ref<"zh" | "en">("zh");
const promptTarget = ref<PromptTarget>("gpt");
const detailTab = ref<"prompt" | "breakdown">("prompt");
const currentPage = ref(1);
const pageSize = ref(24);
const importing = ref(false);
const isExportPage = computed(() => route.path === "/export");

async function syncHistoryFromRoute(value = route.query.history) {
  if (typeof value === "string" && value) {
    await store.openDetail(value);
  } else if (route.path === "/gallery") {
    store.closeDetail();
  }
}

async function loadGallery() {
  await store.load();
  await syncHistoryFromRoute();
}

async function refreshGallery() {
  await loadGallery();
  ElMessage.success("历史记录已刷新");
}

onMounted(loadGallery);

watch(() => route.query.history, (historyId) => {
  void syncHistoryFromRoute(historyId);
});

async function closeHistoryDetail() {
  store.closeDetail();
  if (route.query.history === undefined) return;
  const { history: _history, ...query } = route.query;
  await router.replace({ path: route.path, query });
}

function onDetailVisibilityChange(visible: boolean) {
  if (!visible) void closeHistoryDetail();
}

watch(() => store.detailItem?.id, () => {
  detailTab.value = "prompt";
  promptTab.value = "zh";
  copiedField.value = "";
});

let searchTimer: ReturnType<typeof setTimeout>;
function onSearch(val: string) {
  store.keyword = val;
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => store.load(), 300);
}

function onScoreFilter(val: number | string | null) {
  store.minScore = val ? Number(val) : undefined;
  store.load();
}

function toggleFav() {
  store.favOnly = !store.favOnly;
  store.load();
}

async function exportItems(ids: string[], defaultName = `autoprompt-export-${Date.now()}`) {
  if (!ids.length) return;
  const outputPath = await api.saveFile(defaultName, [
    { name: "ZIP", extensions: ["zip"] },
    { name: "CSV", extensions: ["csv"] },
    { name: "JSON", extensions: ["json"] },
    { name: "Markdown", extensions: ["md"] },
    { name: "TXT", extensions: ["txt"] },
  ]);
  if (!outputPath) return;
  const ext = outputPath.split(".").pop()?.toLowerCase() || "zip";
  const formatMap: Record<string, string> = { zip: "zip", csv: "csv", json: "json", md: "markdown", txt: "txt" };
  await api.exportItems(ids, formatMap[ext] || "zip", outputPath);
  ElMessage.success(`已导出 ${ids.length} 条结果`);
}

async function exportSelected() {
  await exportItems(Array.from(store.selected), `autoprompt-selected-${Date.now()}`);
}

async function exportAll() {
  const data = await api.getHistory({ pageSize: 9999, page: 1 });
  await exportItems(data.items.map((item) => item.id), `autoprompt-all-${Date.now()}`);
}

async function importResults() {
  const inputPath = await api.openImportFile();
  if (!inputPath) return;

  importing.value = true;
  try {
    const summary = await api.importItems(inputPath);
    await store.load();
    const renamedText = summary.renamed ? `，重命名 ${summary.renamed} 条重复记录` : "";
    const skippedText = summary.skipped ? `，跳过 ${summary.skipped} 条空记录` : "";
    ElMessage.success(`已导入 ${summary.imported} 条结果${renamedText}${skippedText}`);
  } finally {
    importing.value = false;
  }
}

function toggleSelectAll() {
  if (store.selectedCount === store.items.length) store.clearSelection();
  else store.selectAll();
}

async function deleteSelected() {
  if (!store.selectedCount) return;
  try {
    await ElMessageBox.confirm(`确定删除选中的 ${store.selectedCount} 条记录吗？`, "删除确认", { type: "warning" });
    await store.deleteSelected();
  } catch { /* cancelled */ }
}

async function deleteDetailItem() {
  if (!store.detailItem) return;
  const id = store.detailItem.id;
  try {
    await ElMessageBox.confirm("确定删除这条历史记录吗？此操作不可恢复。", "删除确认", {
      type: "warning",
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      confirmButtonClass: "el-button--danger",
    });
    await api.deleteHistory([id]);
    await closeHistoryDetail();
    await store.load();
    ElMessage.success("记录已删除");
  } catch {
    /* cancelled */
  }
}

async function copyText(text: string, field: string) {
  await navigator.clipboard.writeText(text);
  copiedField.value = field;
  setTimeout(() => (copiedField.value = ""), 1500);
}

function scoreType(score: number): "success" | "primary" | "warning" | "danger" {
  if (score >= 90) return "success";
  if (score >= 78) return "primary";
  if (score >= 64) return "warning";
  return "danger";
}

const detailDimensions = (item: any) => {
  if (!item?.qualityBreakdown) return [];
  const bd = item.qualityBreakdown;
  return [
    { label: "主体", value: bd.subject },
    { label: "场景", value: bd.context },
    { label: "光照", value: bd.lighting },
    { label: "构图", value: bd.camera },
    { label: "文本", value: bd.text },
    { label: "图像契合度", value: bd.imagen },
  ].filter((d) => d.value != null);
};
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Toolbar -->
    <div data-tauri-drag-region class="shrink-0 px-6 pt-5 pb-4 flex items-center gap-3 flex-wrap border-b border-white/[0.06]">
      <el-input
        :model-value="store.keyword"
        size="default"
        placeholder="搜索提示词关键词..."
        :prefix-icon="Search"
        clearable
        class="!w-[320px]"
        @input="onSearch"
      />

      <el-select v-model="modelFilter" size="default" placeholder="全部模型" clearable class="!w-[150px]">
        <el-option label="GPT-4o Vision" value="gpt-4o" />
        <el-option label="Gemini" value="gemini" />
      </el-select>

      <el-select v-model="scoreFilter" size="default" placeholder="评分范围" clearable class="!w-[150px]" @change="onScoreFilter">
        <el-option label="90+ 优秀" :value="90" />
        <el-option label="78+ 较强" :value="78" />
        <el-option label="64+ 可用" :value="64" />
      </el-select>

      <el-select v-model="dateFilter" size="default" placeholder="日期" clearable class="!w-[130px]">
        <el-option label="今天" value="today" />
        <el-option label="本周" value="week" />
        <el-option label="本月" value="month" />
      </el-select>

      <el-button
        size="default"
        :type="store.favOnly ? 'warning' : ''"
        :plain="!store.favOnly"
        @click="toggleFav"
      >
        <el-icon class="mr-1"><StarFilled v-if="store.favOnly" /><Star v-else /></el-icon>
        收藏
      </el-button>

      <div class="flex-1" />

      <el-button
        data-testid="refresh-history"
        size="default"
        plain
        :loading="store.loading"
        @click="refreshGallery"
      >
        <el-icon class="mr-1"><RefreshLeft /></el-icon>刷新
      </el-button>

      <template v-if="isExportPage">
        <el-button
          size="default"
          plain
          :loading="importing"
          @click="importResults"
        >
          <el-icon class="mr-1"><Upload /></el-icon>导入结果
        </el-button>
        <el-button
          size="default"
          type="primary"
          :disabled="!store.items.length"
          @click="exportAll"
        >
          <el-icon class="mr-1"><Download /></el-icon>导出全部结果
        </el-button>
        <el-button
          size="default"
          type="primary"
          plain
          :disabled="!store.selectedCount"
          @click="exportSelected"
        >
          <el-icon class="mr-1"><Download /></el-icon>导出选中 ({{ store.selectedCount }})
        </el-button>
        <el-button
          size="default"
          plain
          :disabled="!store.items.length"
          @click="toggleSelectAll"
        >
          <el-icon class="mr-1"><Check /></el-icon>{{ store.selectedCount === store.items.length ? '取消全选' : '全选' }}
        </el-button>
      </template>

      <el-button-group>
        <el-button size="default" :type="viewMode === 'grid' ? 'primary' : ''" @click="viewMode = 'grid'">
          <el-icon><Grid /></el-icon>
        </el-button>
        <el-button size="default" :type="viewMode === 'list' ? 'primary' : ''" @click="viewMode = 'list'">
          <el-icon><List /></el-icon>
        </el-button>
      </el-button-group>

      <el-button
        v-if="store.selectedCount && !isExportPage"
        size="default"
        type="primary"
        plain
        @click="exportSelected"
      >
        <el-icon class="mr-1"><Download /></el-icon>导出 ({{ store.selectedCount }})
      </el-button>
      <el-button
        v-if="store.selectedCount && !isExportPage"
        size="default"
        type="danger"
        plain
        @click="deleteSelected"
      >
        <el-icon class="mr-1"><Delete /></el-icon>删除
      </el-button>
    </div>

    <!-- Content -->
    <div class="flex-1 min-h-0 flex">
      <!-- Grid area -->
      <div class="flex-1 overflow-y-auto p-5">
        <el-empty v-if="!store.items.length" description="暂无历史记录" :image-size="100" />

        <div class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3.5">
          <el-card
            v-for="item in store.items"
            :key="item.id"
            shadow="hover"
            body-style="padding:0"
            class="!cursor-pointer transition-all duration-200"
            :class="[
              store.detailItem?.id === item.id ? '!border-teal-400/40' : '',
              store.selected.has(item.id) ? '!border-teal-400/70 ring-2 ring-teal-400/25' : ''
            ]"
            @click="isExportPage ? store.toggleSelect(item.id) : store.openDetail(item.id)"
          >
            <div class="relative aspect-[4/3] bg-[#0a0a12] overflow-hidden rounded-t-xl">
              <img v-if="item.thumbUrl" :src="item.thumbUrl" class="w-full h-full object-cover" alt="" loading="lazy" />
              <el-button
                v-if="isExportPage"
                circle
                size="small"
                class="!absolute !top-2 !left-2"
                :type="store.selected.has(item.id) ? 'primary' : 'default'"
                @click.stop="store.toggleSelect(item.id)"
              >
                <el-icon><Check /></el-icon>
              </el-button>
              <el-button
                v-else
                circle
                size="small"
                class="!absolute !top-2 !left-2"
                :type="item.favorite ? 'warning' : 'default'"
                @click.stop="store.toggleFavorite(item.id)"
              >
                <el-icon><StarFilled v-if="item.favorite" /><Star v-else /></el-icon>
              </el-button>
              <el-tag
                :type="scoreType(item.qualityScore)"
                effect="dark"
                size="default"
                class="!absolute !top-2 !right-2 !font-bold"
              >
                {{ item.qualityScore }}
              </el-tag>
            </div>
            <div class="p-2.5">
              <p class="text-[13px] text-white/65 truncate">{{ (item.prompt_zh || item.prompt_en || item.fileName || '').slice(0, 50) }}</p>
              <p class="text-[11px] text-white/35 mt-1">{{ item.createdAt || '' }} · {{ item.model || 'GPT-4o' }}</p>
            </div>
          </el-card>
        </div>
      </div>

      <!-- Detail Drawer (as side panel) -->
      <el-drawer
        data-testid="history-detail-drawer"
        :model-value="!!store.detailItem"
        :show-close="false"
        :with-header="false"
        size="520px"
        @update:model-value="onDetailVisibilityChange"
      >
        <div v-if="store.detailItem" class="h-full flex flex-col p-5 overflow-y-auto select-text">
          <div class="detail-header mb-4">
            <div>
              <h3 class="text-[18px] font-semibold text-white/90 mb-1">{{ store.detailItem.fileName || '分析结果' }}</h3>
              <p class="text-[12px] text-white/40">{{ store.detailItem.createdAt || '' }}</p>
              <p class="text-[12px] text-white/40">{{ store.detailItem.model || 'GPT-4o Vision' }}</p>
            </div>
            <div class="detail-actions"><el-button class="detail-icon-button" circle size="default" :type="store.detailItem.favorite ? 'warning' : 'default'" @click="store.toggleFavorite(store.detailItem.id)">
                <el-icon><StarFilled v-if="store.detailItem.favorite" /><Star v-else /></el-icon>
              </el-button></div>
            <el-tooltip content="关闭详情" placement="bottom"><el-button data-testid="close-history-detail" class="detail-close-button" circle size="default" aria-label="关闭详情" @click="closeHistoryDetail">
                <el-icon><Close /></el-icon>
              </el-button></el-tooltip>
          </div>

          <el-card shadow="never" body-style="padding:0" class="mb-4 overflow-hidden">
            <el-image
              v-if="store.detailItem.thumbUrl"
              :src="store.detailItem.thumbUrl"
              :preview-src-list="[store.detailItem.thumbUrl]"
              :initial-index="0"
              :z-index="4000"
              fit="cover"
              preview-teleported
              class="history-detail-image"
            />
          </el-card>

          <!-- Quality Score -->
          <div class="flex items-center gap-4 mb-5">
            <RadarChart :data="detailDimensions(store.detailItem)" :size="120" />
            <div class="flex-1 space-y-2">
              <div v-for="dim in detailDimensions(store.detailItem)" :key="dim.label" class="flex items-center gap-2">
                <span class="text-[11px] text-white/45 w-[60px] shrink-0">{{ dim.label }}</span>
                <el-progress :percentage="dim.value || 0" :show-text="false" :stroke-width="6" class="flex-1" />
                <span class="text-[11px] font-semibold text-white/65 w-7 text-right">{{ dim.value }}</span>
              </div>
            </div>
          </div>

          <div class="detail-content-tabs mb-5">
            <el-tabs v-model="detailTab" class="detail-mode-tabs">
              <el-tab-pane label="完整 Prompt" name="prompt">
                <div class="prompt-tabs-wrap">
                  <el-tabs v-model="promptTab" class="prompt-tabs">
                    <el-tab-pane label="中文提示词" name="zh">
                      <div class="prompt-tab-toolbar">
                        <span class="text-[12px] text-white/38">可选取、复制完整内容</span>
                        <el-button size="small" @click="copyText(getModelPrompt(store.detailItem, promptTarget, 'zh'), 'zh')">
                          <el-icon class="mr-1"><Check v-if="copiedField === 'zh'" /><CopyDocument v-else /></el-icon>
                          {{ copiedField === 'zh' ? '已复制' : '复制' }}
                        </el-button>
                      </div>
                      <textarea class="prompt-copy-field" :value="getModelPrompt(store.detailItem, promptTarget, 'zh')" readonly spellcheck="false" />
                    </el-tab-pane>
                    <el-tab-pane label="English Prompt" name="en">
                      <div class="prompt-tab-toolbar">
                        <span class="text-[12px] text-white/38">Select and copy the full prompt</span>
                        <el-button size="small" @click="copyText(getModelPrompt(store.detailItem, promptTarget, 'en'), 'en')">
                          <el-icon class="mr-1"><Check v-if="copiedField === 'en'" /><CopyDocument v-else /></el-icon>
                          {{ copiedField === 'en' ? '已复制' : '复制' }}
                        </el-button>
                      </div>
                      <textarea class="prompt-copy-field" :value="getModelPrompt(store.detailItem, promptTarget, 'en')" readonly spellcheck="false" />
                    </el-tab-pane>
                  </el-tabs>
                </div>
              </el-tab-pane>
              <el-tab-pane label="分析拆解" name="breakdown">
                <AnalysisBreakdown :history-id="store.detailItem.id" />
              </el-tab-pane>
            </el-tabs>
          </div>

          <div class="flex gap-2 mt-auto pt-2">
            <el-button size="default">
              <el-icon class="mr-1"><RefreshLeft /></el-icon>重新分析
            </el-button>
            <el-button type="primary" size="default">
              <el-icon class="mr-1"><Download /></el-icon>导出
            </el-button>
            <el-button type="danger" size="default" plain @click="deleteDetailItem">
              <el-icon class="mr-1"><Delete /></el-icon>删除
            </el-button>
          </div>
        </div>
      </el-drawer>
    </div>

    <!-- Pagination -->
    <div class="shrink-0 px-7 py-3 border-t border-white/[0.06] flex items-center justify-between">
      <span class="text-[13px] text-white/45">共 {{ store.items.length }} 条记录</span>
      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :total="store.items.length"
        :page-sizes="[24, 48, 96]"
        layout="prev, pager, next, sizes"
        background
      />
    </div>
  </div>
</template>

<style scoped>
.prompt-target-row {
  display: flex;
  justify-content: flex-start;
  margin-bottom: 10px;
}


:deep(.el-card) {
  background-color: rgba(14, 17, 23, 0.78);
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 12px;
  box-shadow: none;
}
:deep(.el-card:hover) {
  border-color: rgba(45, 212, 191, 0.28);
}
:deep(.el-drawer) {
  background-color: #10131a;
  border-left: 1px solid rgba(255, 255, 255, 0.08);
}
:deep(.el-drawer__body) {
  padding: 0;
}
.detail-header {
  position: relative;
  min-height: 48px;
}
.detail-actions,
.detail-close-button {
  position: absolute;
  top: 0;
}
.detail-actions {
  right: 46px;
}
.detail-close-button {
  right: 0;
  margin: 0;
}
.detail-icon-button,
.detail-close-button {
  width: 36px;
  height: 36px;
}
.detail-content-tabs {
  min-height: 390px;
}
:deep(.detail-mode-tabs > .el-tabs__header) {
  margin-bottom: 12px;
}
:deep(.detail-mode-tabs > .el-tabs__header .el-tabs__item) {
  height: 36px;
  font-size: 13px;
}
.prompt-tabs-wrap {
  min-height: 340px;
}
:deep(.prompt-tabs .el-tabs__header) {
  margin-bottom: 8px;
}
:deep(.prompt-tabs .el-tabs__nav-wrap::after) {
  background-color: rgba(255, 255, 255, 0.08);
}
.prompt-tab-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}.prompt-copy-field {
  width: 100%;
  min-height: 270px;
  max-height: 460px;
  resize: vertical;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.055);
  padding: 14px;
  color: rgba(255, 255, 255, 0.82);
  font-size: 13px;
  line-height: 1.75;
  outline: none;
  user-select: text;
}
.prompt-copy-field:focus {
  border-color: rgba(45, 212, 191, 0.38);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.08);
}
.history-detail-image {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  cursor: zoom-in;
}
:deep(.history-detail-image .el-image__inner) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
