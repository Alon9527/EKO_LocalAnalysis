<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Check, CopyDocument, Right } from "@element-plus/icons-vue";
import type {
  MaterialAsset,
  MaterialCategory,
  MaterialSourceVariant,
} from "@/lib/api";
import {
  MATERIAL_CATEGORY_LABELS,
  materialDisplayName,
} from "@/lib/materials";
import { useMaterialsStore } from "@/stores/materials";

const props = defineProps<{
  historyId: string;
}>();

interface BreakdownRow {
  asset: MaterialAsset;
  source: MaterialSourceVariant;
}

interface BreakdownSection {
  category: MaterialCategory;
  rows: BreakdownRow[];
}

const CATEGORY_ORDER: MaterialCategory[] = [
  "element",
  "material",
  "color",
  "lighting",
  "camera",
  "composition",
  "style",
  "environment",
];

const store = useMaterialsStore();
const loading = ref(false);
const copiedId = ref("");
let latestRequest = 0;

const sections = computed<BreakdownSection[]>(() => {
  const assets = store.historyItems[props.historyId] || [];
  return CATEGORY_ORDER.map((category) => ({
    category,
    rows: assets
      .filter((asset) => asset.category === category)
      .map((asset) => ({
        asset,
        source: asset.sources.find(
          (source) => source.historyId === props.historyId,
        ),
      }))
      .filter((row): row is BreakdownRow => !!row.source),
  })).filter((section) => section.rows.length > 0);
});

watch(
  () => props.historyId,
  async (historyId) => {
    const requestId = ++latestRequest;
    copiedId.value = "";
    if (!historyId) return;
    loading.value = true;
    await store.loadForHistory(historyId);
    if (requestId === latestRequest) loading.value = false;
  },
  { immediate: true },
);

async function copySource(row: BreakdownRow) {
  try {
    await navigator.clipboard.writeText(row.source.promptZh);
    copiedId.value = row.asset.id;
    window.setTimeout(() => {
      if (copiedId.value === row.asset.id) copiedId.value = "";
    }, 1200);
  } catch {
    copiedId.value = "";
  }
}
</script>

<template>
  <div class="analysis-breakdown">
    <div v-if="loading" class="breakdown-state">
      正在加载分析拆解...
    </div>

    <div v-else-if="store.error" class="breakdown-state breakdown-state--error">
      <strong>分析拆解加载失败</strong>
      <span>{{ store.error }}</span>
    </div>

    <div v-else-if="!sections.length" class="breakdown-state breakdown-state--empty">
      <strong>暂无结构化分析内容</strong>
      <span>旧记录或仅保存完整 Prompt 的记录不会自动补造拆解内容。</span>
    </div>

    <div v-else class="breakdown-sections">
      <section
        v-for="section in sections"
        :key="section.category"
        :data-testid="`breakdown-section-${section.category}`"
        :data-category="section.category"
        class="breakdown-section"
      >
        <header class="breakdown-section__header">
          <h4>{{ MATERIAL_CATEGORY_LABELS[section.category] }}</h4>
          <span>{{ section.rows.length }}</span>
        </header>

        <div class="breakdown-section__rows">
          <article
            v-for="row in section.rows"
            :key="row.asset.id"
            class="breakdown-row"
          >
            <div class="breakdown-row__main">
              <div class="breakdown-row__title-line">
                <strong>{{ materialDisplayName(row.asset) }}</strong>
                <span
                  :data-testid="`language-${row.asset.id}`"
                  class="language-state"
                >
                  {{ row.source.promptEn ? "中 / EN" : "中" }}
                </span>
              </div>
              <p class="breakdown-row__prompt">{{ row.source.promptZh }}</p>
              <code>{{ row.source.fieldPath }}</code>
            </div>

            <div class="breakdown-row__actions">
              <button
                :data-testid="`copy-material-${row.asset.id}`"
                type="button"
                class="icon-action"
                :title="copiedId === row.asset.id ? '已复制' : '复制中文片段'"
                :aria-label="copiedId === row.asset.id ? '已复制' : '复制中文片段'"
                @click="copySource(row)"
              >
                <el-icon>
                  <Check v-if="copiedId === row.asset.id" />
                  <CopyDocument v-else />
                </el-icon>
              </button>
              <RouterLink
                :data-testid="`open-material-${row.asset.id}`"
                :to="{ path: '/materials', query: { asset: row.asset.id } }"
                class="asset-link"
              >
                <span>在素材库查看</span>
                <el-icon><Right /></el-icon>
              </RouterLink>
            </div>
          </article>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.analysis-breakdown {
  min-height: 340px;
  color: rgba(255, 255, 255, 0.86);
}

.breakdown-state {
  min-height: 220px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 28px;
  text-align: center;
  color: rgba(255, 255, 255, 0.46);
}

.breakdown-state strong {
  color: rgba(255, 255, 255, 0.78);
  font-size: 14px;
}

.breakdown-state span {
  max-width: 340px;
  font-size: 12px;
  line-height: 1.65;
}

.breakdown-state--error strong {
  color: #fda4af;
}

.breakdown-sections {
  display: grid;
  gap: 18px;
}

.breakdown-section {
  min-width: 0;
}

.breakdown-section__header {
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(255, 255, 255, 0.09);
}

.breakdown-section__header h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 650;
  color: rgba(255, 255, 255, 0.86);
}

.breakdown-section__header span {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.35);
}

.breakdown-section__rows {
  display: grid;
}

.breakdown-row {
  min-height: 100px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 14px;
  align-items: start;
  padding: 13px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.breakdown-row__main {
  min-width: 0;
}

.breakdown-row__title-line {
  display: flex;
  align-items: center;
  gap: 8px;
}

.breakdown-row__title-line strong {
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 13px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.88);
}

.language-state {
  flex: 0 0 auto;
  font-size: 10px;
  color: #5eead4;
}

.breakdown-row__prompt {
  margin: 7px 0 6px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-size: 12px;
  line-height: 1.65;
  color: rgba(255, 255, 255, 0.64);
  user-select: text;
}

.breakdown-row code {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.3);
  overflow-wrap: anywhere;
}

.breakdown-row__actions {
  width: 104px;
  display: grid;
  justify-items: end;
  gap: 10px;
}

.icon-action {
  width: 32px;
  height: 32px;
  display: inline-grid;
  place-items: center;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  background: transparent;
  color: rgba(255, 255, 255, 0.58);
  cursor: pointer;
}

.icon-action:hover,
.icon-action:focus-visible {
  border-color: rgba(45, 212, 191, 0.5);
  color: #5eead4;
  outline: none;
}

.asset-link {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  font-size: 11px;
  color: rgba(94, 234, 212, 0.78);
  text-decoration: none;
}

.asset-link:hover,
.asset-link:focus-visible {
  color: #5eead4;
  outline: none;
}
</style>
