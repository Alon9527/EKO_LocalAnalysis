<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Close, CopyDocument, Star, StarFilled } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import type { MaterialAsset, MaterialPatch } from "@/lib/api";

type OperationComplete = (success: boolean) => void;
import {
  MATERIAL_CATEGORY_LABELS,
  materialDisplayName,
  materialPromptEn,
  materialPromptZh,
} from "@/lib/materials";
import MaterialSourceGrid from "@/components/materials/MaterialSourceGrid.vue";

const props = defineProps<{
  modelValue: boolean;
  asset: MaterialAsset | null;
  candidates: MaterialAsset[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  save: [id: string, patch: MaterialPatch];
  merge: [ids: string[], displayName: string | undefined, complete: OperationComplete];
  split: [id: string, sourceIds: string[], displayName: string, complete: OperationComplete];
  "toggle-favorite": [id: string, favorite: boolean];
}>();

const activeTab = ref("zh");
const displayName = ref("");
const promptZh = ref("");
const promptEn = ref("");
const aliasesText = ref("");
const mergeOpen = ref(false);
const mergePending = ref(false);
const mergeTarget = ref("");
const mergeName = ref("");
const splitOpen = ref(false);
const splitPending = ref(false);
const splitSourceIds = ref<string[]>([]);
const splitName = ref("");
const copied = ref("");
const mergePanel = ref<HTMLDivElement | null>(null);
const splitPanel = ref<HTMLDivElement | null>(null);
const mergeTrigger = ref<HTMLButtonElement | null>(null);
const splitTrigger = ref<HTMLButtonElement | null>(null);
const FOCUSABLE_SELECTOR = 'a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';
let mergeOperationId = 0;
let splitOperationId = 0;

const sameCategoryCandidates = computed(() => {
  if (!props.asset) return [];
  return props.candidates.filter(
    (candidate) => candidate.id !== props.asset?.id && candidate.category === props.asset?.category,
  );
});

const canSave = computed(
  () => !!displayName.value.trim() && !!promptZh.value.trim(),
);

const canSplit = computed(() => {
  const count = splitSourceIds.value.length;
  const total = props.asset?.sources.length || 0;
  return count > 0 && count < total && !!splitName.value.trim();
});

function focusFirst(panelRef: { value: HTMLElement | null }) {
  nextTick(() => {
    const panel = panelRef.value;
    const first = panel?.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
    (first || panel)?.focus();
  });
}

function cancelMergeOperation() {
  mergeOperationId += 1;
  mergePending.value = false;
}

function cancelSplitOperation() {
  splitOperationId += 1;
  splitPending.value = false;
}

function closeMerge() {
  cancelMergeOperation();
  mergeOpen.value = false;
  nextTick(() => mergeTrigger.value?.focus());
}

function closeSplit() {
  cancelSplitOperation();
  splitOpen.value = false;
  nextTick(() => splitTrigger.value?.focus());
}

function resetForm(asset: MaterialAsset | null) {
  activeTab.value = "zh";
  copied.value = "";
  mergeOpen.value = false;
  splitOpen.value = false;
  cancelMergeOperation();
  cancelSplitOperation();
  mergeTarget.value = "";
  splitSourceIds.value = [];
  if (!asset) return;
  displayName.value = materialDisplayName(asset);
  promptZh.value = materialPromptZh(asset);
  promptEn.value = materialPromptEn(asset);
  aliasesText.value = (asset.userOverride.aliases.length
    ? asset.userOverride.aliases
    : asset.generatedAliases
  ).join("，");
  mergeName.value = materialDisplayName(asset);
  splitName.value = `${materialDisplayName(asset)} - 新素材`;
}

watch(() => props.asset, resetForm, { immediate: true });

function save() {
  if (!props.asset || !canSave.value) return;
  const aliases = aliasesText.value
    .split(/[，,]/)
    .map((alias) => alias.trim())
    .filter(Boolean);
  const patch: MaterialPatch = {
    displayName: displayName.value.trim(),
    promptZh: promptZh.value.trim(),
    aliases,
  };
  const normalizedPromptEn = promptEn.value.trim();
  if (normalizedPromptEn) patch.promptEn = normalizedPromptEn;
  emit("save", props.asset.id, patch);
}

function openMerge() {
  if (!props.asset) return;
  cancelSplitOperation();
  splitOpen.value = false;
  cancelMergeOperation();
  mergeTarget.value = "";
  mergeName.value = materialDisplayName(props.asset);
  mergeOpen.value = true;
  focusFirst(mergePanel);
}

function confirmMerge() {
  if (!props.asset || !mergeTarget.value || mergePending.value) return;
  mergePending.value = true;
  const operationId = ++mergeOperationId;
  emit(
    "merge",
    [props.asset.id, mergeTarget.value],
    mergeName.value.trim() || undefined,
    (success) => {
      if (operationId !== mergeOperationId) return;
      mergePending.value = false;
      if (success) closeMerge();
    },
  );
}

function openSplit() {
  if (!props.asset) return;
  cancelMergeOperation();
  mergeOpen.value = false;
  cancelSplitOperation();
  splitSourceIds.value = [];
  splitName.value = `${materialDisplayName(props.asset)} - 新素材`;
  splitOpen.value = true;
  focusFirst(splitPanel);
}

function confirmSplit() {
  if (!props.asset || !canSplit.value || splitPending.value) return;
  splitPending.value = true;
  const operationId = ++splitOperationId;
  emit(
    "split",
    props.asset.id,
    [...splitSourceIds.value],
    splitName.value.trim(),
    (success) => {
      if (operationId !== splitOperationId) return;
      splitPending.value = false;
      if (success) closeSplit();
    },
  );
}

function setSplitSource(sourceId: string, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  if (checked && !splitSourceIds.value.includes(sourceId)) {
    splitSourceIds.value = [...splitSourceIds.value, sourceId];
  } else if (!checked) {
    splitSourceIds.value = splitSourceIds.value.filter((id) => id !== sourceId);
  }
}

function trapFocus(event: KeyboardEvent, panel: HTMLElement | null) {
  if (!panel) return;
  const focusable = Array.from(
    panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
  if (!focusable.length) {
    event.preventDefault();
    panel.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  const active = document.activeElement;
  if (event.shiftKey && (active === panel || active === first || !panel.contains(active))) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

async function copyPrompt(language: "zh" | "en") {
  if (!props.asset) return;
  const value = language === "zh" ? promptZh.value : promptEn.value;
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    ElMessage.success("已复制");
    copied.value = language;
    window.setTimeout(() => {
      if (copied.value === language) copied.value = "";
    }, 1200);
  } catch {
    copied.value = "";
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="modelValue && asset"
      class="material-detail-overlay"
      role="presentation"
      @click.self="emit('update:modelValue', false)"
    >
      <aside class="material-detail-drawer" role="dialog" aria-modal="true" aria-label="素材详情">
        <div class="material-detail">
      <header class="material-detail__header">
        <div class="min-w-0">
          <div class="material-detail__eyebrow">
            {{ MATERIAL_CATEGORY_LABELS[asset.category] }} · {{ asset.sources.length }} 个来源
          </div>
          <h2>{{ materialDisplayName(asset) }}</h2>
        </div>
        <div class="material-detail__header-actions">
          <button
            type="button"
            class="icon-button"
            :class="{ 'is-favorite': asset.userOverride.favorite }"
            :title="asset.userOverride.favorite ? '取消收藏' : '收藏素材'"
            @click="emit('toggle-favorite', asset.id, !asset.userOverride.favorite)"
          >
            <el-icon :size="18">
              <StarFilled v-if="asset.userOverride.favorite" />
              <Star v-else />
            </el-icon>
          </button>
          <button
            type="button"
            class="icon-button"
            title="关闭"
            @click="emit('update:modelValue', false)"
          >
            <el-icon :size="19"><Close /></el-icon>
          </button>
        </div>
      </header>

      <section class="material-detail__section material-detail__editor">
        <label class="field-label">素材名称</label>
        <div data-testid="display-name">
          <el-input
            v-model="displayName"
            maxlength="80"
            placeholder="便于检索的素材名称"
          />
        </div>

        <label class="field-label field-label--spaced">别名</label>
        <el-input
          v-model="aliasesText"
          placeholder="多个别名用逗号分隔"
        />

        <el-tabs v-model="activeTab" class="prompt-tabs">
          <el-tab-pane label="中文 Prompt" name="zh">
            <div class="prompt-editor">
              <div data-testid="prompt-zh">
                <el-input
                  v-model="promptZh"
                  type="textarea"
                  :rows="6"
                  resize="vertical"
                />
              </div>
              <button type="button" class="copy-button" @click="copyPrompt('zh')">
                <el-icon><CopyDocument /></el-icon>
                {{ copied === "zh" ? "已复制" : "复制" }}
              </button>
            </div>
          </el-tab-pane>
          <el-tab-pane label="English Prompt" name="en" :disabled="!promptEn">
            <div v-if="promptEn" class="prompt-editor">
              <el-input v-model="promptEn" type="textarea" :rows="6" resize="vertical" />
              <button type="button" class="copy-button" @click="copyPrompt('en')">
                <el-icon><CopyDocument /></el-icon>
                {{ copied === "en" ? "Copied" : "Copy" }}
              </button>
            </div>
            <p v-else class="muted-text">这条素材暂时没有英文 Prompt。</p>
          </el-tab-pane>
        </el-tabs>
        <div v-if="!promptEn" class="english-source-note">
          <span>未提取到可靠的英文片段。</span>
          <RouterLink
            v-if="asset.sources[0]"
            :to="{ path: '/gallery', query: { history: asset.sources[0].historyId } }"
          >
            查看来源图片的完整英文 Prompt
          </RouterLink>
        </div>

        <div class="material-detail__actions">
          <button ref="mergeTrigger" data-testid="open-merge" type="button" class="secondary-button" @click="openMerge">
            合并
          </button>
          <button
            ref="splitTrigger"
            data-testid="open-split"
            type="button"
            class="secondary-button"
            :disabled="asset.sources.length < 2"
            @click="openSplit"
          >
            拆分
          </button>
          <button
            data-testid="save-material"
            type="button"
            class="primary-button"
            :disabled="!canSave"
            @click="save"
          >
            保存修改
          </button>
        </div>
      </section>

      <section class="material-detail__section">
        <div class="section-heading">
          <div>
            <h3>来源与变体</h3>
            <p>每条片段都可追溯到原始图片和结构化字段。</p>
          </div>
        </div>
        <MaterialSourceGrid :sources="asset.sources" />
      </section>

      <div v-if="mergeOpen" class="operation-backdrop" @click="closeMerge" />
      <div
        v-if="mergeOpen"
        ref="mergePanel"
        class="operation-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="merge-material-title"
        tabindex="-1"
        @keydown.esc.stop="closeMerge"
        @keydown.tab="trapFocus($event, mergePanel)"
      >
        <div class="operation-panel__header">
          <div>
            <h3 id="merge-material-title">合并同类素材</h3>
            <p>只显示 {{ MATERIAL_CATEGORY_LABELS[asset.category] }} 分类中的其他素材。</p>
          </div>
          <button type="button" class="icon-button" title="关闭" @click="closeMerge">
            <el-icon><Close /></el-icon>
          </button>
        </div>
        <div class="operation-panel__body">
          <label class="field-label">合并后的名称</label>
          <el-input v-model="mergeName" />
          <div class="choice-list">
            <label
              v-for="candidate in sameCategoryCandidates"
              :key="candidate.id"
              :data-testid="`merge-option-${candidate.id}`"
              class="choice-row"
            >
              <input v-model="mergeTarget" type="radio" name="merge-target" :value="candidate.id" />
              <span>{{ materialDisplayName(candidate) }}</span>
              <small>{{ candidate.sources.length }} 个来源</small>
            </label>
            <p v-if="!sameCategoryCandidates.length" class="muted-text">暂无可合并的同类素材。</p>
          </div>
        </div>
        <div class="operation-panel__footer">
          <button type="button" class="secondary-button" @click="closeMerge">取消</button>
          <button
            data-testid="confirm-merge"
            type="button"
            class="primary-button"
            :disabled="!mergeTarget || mergePending"
            @click="confirmMerge"
          >
            {{ mergePending ? "正在合并" : "确认合并" }}
          </button>
        </div>
      </div>

      <div v-if="splitOpen" class="operation-backdrop" @click="closeSplit" />
      <div
        v-if="splitOpen"
        ref="splitPanel"
        class="operation-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="split-material-title"
        tabindex="-1"
        @keydown.esc.stop="closeSplit"
        @keydown.tab="trapFocus($event, splitPanel)"
      >
        <div class="operation-panel__header">
          <div>
            <h3 id="split-material-title">拆分来源片段</h3>
            <p>选择一部分来源创建新素材，至少保留一条在当前素材中。</p>
          </div>
          <button type="button" class="icon-button" title="关闭" @click="closeSplit">
            <el-icon><Close /></el-icon>
          </button>
        </div>
        <div class="operation-panel__body">
          <label class="field-label">新素材名称</label>
          <el-input v-model="splitName" />
          <div class="choice-list">
            <label
              v-for="source in asset.sources"
              :key="source.id"
              :data-testid="`split-source-${source.id}`"
              class="choice-row choice-row--source"
            >
              <input
                type="checkbox"
                :value="source.id"
                :checked="splitSourceIds.includes(source.id)"
                @change="setSplitSource(source.id, $event)"
              />
              <span>{{ source.promptZh }}</span>
              <small>{{ source.fieldPath }}</small>
            </label>
          </div>
        </div>
        <div class="operation-panel__footer">
          <button type="button" class="secondary-button" @click="closeSplit">取消</button>
          <button
            data-testid="confirm-split"
            type="button"
            class="primary-button"
            :disabled="!canSplit || splitPending"
            @click="confirmSplit"
          >
            {{ splitPending ? "正在创建" : "创建新素材" }}
          </button>
        </div>
      </div>
        </div>
      </aside>
    </div>
  </Teleport>
</template>

<style>
.material-detail-overlay {
  position: fixed;
  z-index: 2000;
  inset: 0;
  display: flex;
  justify-content: flex-end;
  background: rgba(0, 0, 0, 0.46);
}

.material-detail-drawer {
  width: min(620px, 92vw);
  height: 100%;
  overflow-y: auto;
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  background: #101319;
  box-shadow: -20px 0 60px rgba(0, 0, 0, 0.46);
  color: rgba(255, 255, 255, 0.9);
  animation: material-drawer-enter 180ms ease-out;
}

@keyframes material-drawer-enter {
  from { transform: translateX(18px); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
</style>

<style scoped>
.material-detail {
  position: relative;
  min-height: 100%;
}

.material-detail__header {
  position: sticky;
  z-index: 4;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(16, 19, 25, 0.96);
  padding: 18px 20px;
  backdrop-filter: blur(14px);
}

.material-detail__eyebrow {
  margin-bottom: 4px;
  color: #5eead4;
  font-size: 12px;
}

.material-detail__header h2,
.operation-panel h3,
.section-heading h3 {
  margin: 0;
  letter-spacing: 0;
}

.material-detail__header h2 {
  overflow: hidden;
  font-size: 19px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.material-detail__header-actions,
.material-detail__actions,
.operation-panel__footer {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-button,
.copy-button,
.primary-button,
.secondary-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  cursor: pointer;
}

.icon-button {
  width: 34px;
  height: 34px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.035);
  color: rgba(255, 255, 255, 0.65);
}

.icon-button:hover {
  border-color: rgba(255, 255, 255, 0.22);
  color: white;
}

.icon-button.is-favorite {
  border-color: rgba(251, 191, 36, 0.35);
  color: #fbbf24;
}

.material-detail__section {
  padding: 20px;
}

.material-detail__section + .material-detail__section {
  border-top: 1px solid rgba(255, 255, 255, 0.07);
}

.field-label {
  display: block;
  margin-bottom: 7px;
  color: rgba(255, 255, 255, 0.58);
  font-size: 12px;
  font-weight: 600;
}

.field-label--spaced {
  margin-top: 14px;
}

.prompt-tabs {
  margin-top: 18px;
}

.prompt-editor {
  position: relative;
}

.copy-button {
  position: absolute;
  right: 8px;
  bottom: 8px;
  gap: 5px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: #171b22;
  padding: 6px 9px;
  color: rgba(255, 255, 255, 0.66);
  font-size: 12px;
}

.material-detail__actions {
  justify-content: flex-end;
  margin-top: 16px;
}

.primary-button,
.secondary-button {
  min-height: 34px;
  border: 1px solid transparent;
  padding: 0 13px;
  font-size: 13px;
}

.primary-button {
  border-color: rgba(45, 212, 191, 0.55);
  background: #20b8a6;
  color: #06110f;
  font-weight: 650;
}

.secondary-button {
  border-color: rgba(255, 255, 255, 0.11);
  background: rgba(255, 255, 255, 0.035);
  color: rgba(255, 255, 255, 0.72);
}

.primary-button:disabled,
.secondary-button:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.section-heading {
  margin-bottom: 13px;
}

.section-heading h3 {
  font-size: 15px;
}

.section-heading p,
.operation-panel p,
.muted-text {
  margin: 5px 0 0;
  color: rgba(255, 255, 255, 0.42);
  font-size: 12px;
  line-height: 1.5;
}

.english-source-note {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: -6px;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
  padding-top: 10px;
  color: rgba(255, 255, 255, 0.42);
  font-size: 12px;
}

.english-source-note a {
  flex-shrink: 0;
  color: #5eead4;
  text-decoration: none;
}

.english-source-note a:hover {
  color: #99f6e4;
}

.operation-backdrop {
  position: fixed;
  z-index: 9;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
}

.operation-panel {
  position: fixed;
  z-index: 10;
  top: 50%;
  left: 50%;
  width: min(480px, calc(100vw - 40px));
  max-height: min(620px, calc(100vh - 40px));
  overflow: auto;
  border: 1px solid rgba(255, 255, 255, 0.13);
  border-radius: 8px;
  background: #151920;
  box-shadow: 0 22px 70px rgba(0, 0, 0, 0.55);
  color: rgba(255, 255, 255, 0.9);
  transform: translate(-50%, -50%);
}

.operation-panel__header,
.operation-panel__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 15px 16px;
}

.operation-panel__header,
.operation-panel__body {
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.operation-panel__header h3 {
  font-size: 15px;
}

.operation-panel__body {
  padding: 16px;
}

.operation-panel__footer {
  justify-content: flex-end;
}

.choice-list {
  display: grid;
  gap: 6px;
  margin-top: 13px;
}

.choice-row {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  padding: 9px 10px;
  color: rgba(255, 255, 255, 0.76);
  cursor: pointer;
}

.choice-row:hover {
  border-color: rgba(45, 212, 191, 0.3);
  background: rgba(45, 212, 191, 0.04);
}

.choice-row input {
  accent-color: #2dd4bf;
}

.choice-row small {
  color: rgba(255, 255, 255, 0.36);
}

.choice-row--source {
  grid-template-columns: 18px minmax(0, 1fr);
}

.choice-row--source small {
  grid-column: 2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 680px) {
  .material-detail__header,
  .material-detail__section {
    padding-right: 14px;
    padding-left: 14px;
  }
}
</style>
