<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useGalleryStore } from "@/stores/gallery";
import { api } from "@/lib/api";
import RadarChart from "@/components/RadarChart.vue";
import { ElMessageBox } from "element-plus";
import {
  Search, Star, StarFilled, Download, Delete, Close, CopyDocument, Check,
  Grid, List, RefreshLeft, Picture as PicIcon
} from "@element-plus/icons-vue";

const store = useGalleryStore();
const viewMode = ref<"grid" | "list">("grid");
const modelFilter = ref("");
const dateFilter = ref("");
const scoreFilter = ref<number | "">("");
const copiedField = ref("");
const currentPage = ref(1);
const pageSize = ref(24);

onMounted(() => store.load());

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

async function exportSelected() {
  const ids = Array.from(store.selected);
  if (!ids.length) return;
  const outputPath = await api.saveFile(`autoprompt-export-${Date.now()}`, [
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
}

async function deleteSelected() {
  if (!store.selectedCount) return;
  try {
    await ElMessageBox.confirm(`确定删除选中的 ${store.selectedCount} 条记录吗？`, "删除确认", { type: "warning" });
    await store.deleteSelected();
  } catch { /* cancelled */ }
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
    <div data-tauri-drag-region class="shrink-0 px-8 pt-7 pb-5 flex items-center gap-4 flex-wrap border-b border-white/[0.06]">
      <el-input
        :model-value="store.keyword"
        size="large"
        placeholder="搜索提示词关键词..."
        :prefix-icon="Search"
        clearable
        class="!w-[360px]"
        @input="onSearch"
      />

      <el-select v-model="modelFilter" size="large" placeholder="全部模型" clearable class="!w-[160px]">
        <el-option label="GPT-4o Vision" value="gpt-4o" />
        <el-option label="Gemini" value="gemini" />
      </el-select>

      <el-select v-model="scoreFilter" size="large" placeholder="评分范围" clearable class="!w-[160px]" @change="onScoreFilter">
        <el-option label="90+ 优秀" :value="90" />
        <el-option label="78+ 较强" :value="78" />
        <el-option label="64+ 可用" :value="64" />
      </el-select>

      <el-select v-model="dateFilter" size="large" placeholder="日期" clearable class="!w-[140px]">
        <el-option label="今天" value="today" />
        <el-option label="本周" value="week" />
        <el-option label="本月" value="month" />
      </el-select>

      <el-button
        size="large"
        :type="store.favOnly ? 'warning' : ''"
        :plain="!store.favOnly"
        @click="toggleFav"
      >
        <el-icon class="mr-1"><StarFilled v-if="store.favOnly" /><Star v-else /></el-icon>
        收藏
      </el-button>

      <div class="flex-1" />

      <el-button-group>
        <el-button size="large" :type="viewMode === 'grid' ? 'primary' : ''" @click="viewMode = 'grid'">
          <el-icon><Grid /></el-icon>
        </el-button>
        <el-button size="large" :type="viewMode === 'list' ? 'primary' : ''" @click="viewMode = 'list'">
          <el-icon><List /></el-icon>
        </el-button>
      </el-button-group>

      <el-button
        v-if="store.selectedCount"
        size="large"
        type="primary"
        plain
        @click="exportSelected"
      >
        <el-icon class="mr-1"><Download /></el-icon>导出 ({{ store.selectedCount }})
      </el-button>
      <el-button
        v-if="store.selectedCount"
        size="large"
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
      <div class="flex-1 overflow-y-auto p-7">
        <el-empty v-if="!store.items.length" description="暂无历史记录" :image-size="100" />

        <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
          <el-card
            v-for="item in store.items"
            :key="item.id"
            shadow="hover"
            body-style="padding:0"
            class="!cursor-pointer transition-all"
            :class="store.detailItem?.id === item.id ? '!border-teal-400/40' : ''"
            @click="store.openDetail(item.id)"
          >
            <div class="relative aspect-[4/3] bg-[#0a0a12] overflow-hidden rounded-t-2xl">
              <img v-if="item.thumbUrl" :src="item.thumbUrl" class="w-full h-full object-cover" alt="" loading="lazy" />
              <el-button
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
            <div class="p-3">
              <p class="text-[13px] text-white/65 truncate">{{ (item.prompt_zh || item.prompt_en || item.fileName || '').slice(0, 50) }}</p>
              <p class="text-[11px] text-white/35 mt-1">{{ item.createdAt || '' }} · {{ item.model || 'GPT-4o' }}</p>
            </div>
          </el-card>
        </div>
      </div>

      <!-- Detail Drawer (as side panel) -->
      <el-drawer
        v-model="store.detailItem"
        :show-close="false"
        :with-header="false"
        size="420px"
        :before-close="store.closeDetail"
      >
        <div v-if="store.detailItem" class="h-full flex flex-col p-5 overflow-y-auto">
          <div class="flex items-start justify-between mb-4">
            <div>
              <h3 class="text-[18px] font-semibold text-white/90 mb-1">{{ store.detailItem.fileName || '分析结果' }}</h3>
              <p class="text-[12px] text-white/40">{{ store.detailItem.createdAt || '' }}</p>
              <p class="text-[12px] text-white/40">{{ store.detailItem.model || 'GPT-4o Vision' }}</p>
            </div>
            <div class="flex gap-2">
              <el-button circle size="default" :type="store.detailItem.favorite ? 'warning' : 'default'" @click="store.toggleFavorite(store.detailItem.id)">
                <el-icon><StarFilled v-if="store.detailItem.favorite" /><Star v-else /></el-icon>
              </el-button>
              <el-button circle size="default" @click="store.closeDetail">
                <el-icon><Close /></el-icon>
              </el-button>
            </div>
          </div>

          <el-card shadow="never" body-style="padding:0" class="mb-4 overflow-hidden">
            <img v-if="store.detailItem.thumbUrl" :src="store.detailItem.thumbUrl" class="w-full aspect-video object-cover" alt="" />
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

          <div class="mb-4">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-[13px] font-semibold text-white/70">中文提示词</h4>
              <el-button size="small" @click="copyText(store.detailItem.prompt_zh, 'zh')">
                <el-icon class="mr-1"><Check v-if="copiedField === 'zh'" /><CopyDocument v-else /></el-icon>
                复制
              </el-button>
            </div>
            <el-card shadow="never" body-style="padding:14px">
              <p class="text-[13px] text-white/65 leading-relaxed">{{ store.detailItem.prompt_zh }}</p>
            </el-card>
          </div>

          <div class="mb-5">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-[13px] font-semibold text-white/70">English Prompt</h4>
              <el-button size="small" @click="copyText(store.detailItem.prompt_en, 'en')">
                <el-icon class="mr-1"><Check v-if="copiedField === 'en'" /><CopyDocument v-else /></el-icon>
                复制
              </el-button>
            </div>
            <el-card shadow="never" body-style="padding:14px">
              <p class="text-[13px] text-white/65 leading-relaxed">{{ store.detailItem.prompt_en }}</p>
            </el-card>
          </div>

          <div class="flex gap-2 mt-auto">
            <el-button size="default">
              <el-icon class="mr-1"><RefreshLeft /></el-icon>重新分析
            </el-button>
            <el-button type="primary" size="default">
              <el-icon class="mr-1"><Download /></el-icon>导出
            </el-button>
            <el-button type="danger" size="default" plain>
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
:deep(.el-card) {
  background-color: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
}
:deep(.el-card:hover) {
  border-color: rgba(255, 255, 255, 0.14);
}
:deep(.el-drawer) {
  background-color: #12121a;
  border-left: 1px solid rgba(255, 255, 255, 0.08);
}
:deep(.el-drawer__body) {
  padding: 0;
}
</style>
