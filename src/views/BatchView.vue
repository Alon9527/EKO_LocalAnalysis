<script setup lang="ts">
import { ref, computed } from "vue";
import { api } from "@/lib/api";
import { useSettingsStore } from "@/stores/settings";
import { uid, pathBasename } from "@/lib/utils";
import {
  Plus, FolderOpened, VideoPlay, VideoPause, Close, Check, Loading,
  Clock, EditPen, User, Picture as PicIcon, Sunny, Camera, CopyDocument, Aim
} from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";

interface BatchItem {
  id: string;
  filePath: string;
  fileName: string;
  status: "pending" | "processing" | "done" | "failed";
  progress: number;
  result: any;
  error: string | null;
  thumbUrl: string;
}

const settingsStore = useSettingsStore();
const queue = ref<BatchItem[]>([]);
const running = ref(false);
const paused = ref(false);
const doneCount = ref(0);
const failedCount = ref(0);
const currentItem = ref<BatchItem | null>(null);
const avgTime = ref(0);
const currentLang = ref<"zh" | "en">("zh");

const promptText = computed({
  get() {
    const r = currentItem.value?.result;
    if (!r) return "";
    return currentLang.value === "zh" ? (r.prompt_zh || r.prompt_en) : (r.prompt_en || r.prompt_zh);
  },
  set(value: string) {
    const r = currentItem.value?.result;
    if (!r) return;
    if (currentLang.value === "zh") {
      r.prompt_zh = value;
    } else {
      r.prompt_en = value;
    }
  },
});

async function copyPromptBatch() {
  if (!promptText.value) return;
  await navigator.clipboard.writeText(promptText.value);
  ElMessage.success("已复制");
}

const totalPct = computed(() => {
  if (!queue.value.length) return 0;
  return Math.round(((doneCount.value + failedCount.value) / queue.value.length) * 100);
});

const remainingTime = computed(() => {
  if (!avgTime.value || !running.value) return "";
  const remaining = queue.value.length - doneCount.value - failedCount.value;
  const secs = Math.round((remaining * avgTime.value) / 1000);
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return m > 0 ? `${m}分${s.toString().padStart(2, "0")}秒` : `${s}秒`;
});

const structFieldsForCurrent = computed(() => {
  const item = currentItem.value;
  const r: any = item?.result;
  if (!r?.reconstructed_prompt) return [];
  const nextFields = structuredFieldsFromPrompt(r.reconstructed_prompt);
  if (nextFields.length) return nextFields;
  const en = r.reconstructed_prompt;
  const zh = r.reconstructed_prompt_zh;
  const useZh = currentLang.value === "zh" && zh;
  const pick = useZh ? zh : en;
  const labels = useZh
    ? { style_prefix: "风格", subject: "主体", context: "场景", lighting: "光影", camera: "镜头" }
    : { style_prefix: "style_prefix", subject: "subject", context: "context", lighting: "lighting", camera: "camera" };
  return [
    { key: "style_prefix", label: labels.style_prefix, value: pick.style_prefix, icon: EditPen, color: "#a78bfa" },
    { key: "subject", label: labels.subject, value: pick.subject, icon: User, color: "#60a5fa" },
    { key: "context_and_background", label: labels.context, value: pick.context_and_background, icon: PicIcon, color: "#34d399" },
    { key: "lighting", label: labels.lighting, value: pick.lighting, icon: Sunny, color: "#fbbf24" },
    { key: "camera_and_composition", label: labels.camera, value: pick.camera_and_composition, icon: Camera, color: "#22d3ee" },
  ].filter((f) => f.value);
});

function formatStructuredValue(value: any) {
  if (Array.isArray(value)) {
    if (!value.length) return "";
    if (value.every((item) => typeof item === "string")) return value.join("、");
    return JSON.stringify(value, null, 2);
  }
  if (value && typeof value === "object") return JSON.stringify(value, null, 2);
  return value || "";
}

function structuredFieldsFromPrompt(rp: any) {
  if (!rp?.global_scene && !rp?.composition && !rp?.entities && !rp?.environment_details && !rp?.technical_specs) {
    return [];
  }
  return [
    { key: "global_scene.art_style", label: "画面风格", value: rp.global_scene?.art_style, icon: EditPen, color: "#a78bfa" },
    { key: "global_scene.atmosphere", label: "整体氛围", value: rp.global_scene?.atmosphere, icon: PicIcon, color: "#60a5fa" },
    { key: "global_scene.color_palette", label: "色彩", value: formatStructuredValue(rp.global_scene?.color_palette), icon: PicIcon, color: "#34d399" },
    { key: "global_scene.lighting", label: "光线", value: rp.global_scene?.lighting, icon: Sunny, color: "#fbbf24" },
    { key: "composition.camera_angle", label: "视角", value: rp.composition?.camera_angle, icon: Camera, color: "#22d3ee" },
    { key: "composition.focal_length", label: "镜头感", value: rp.composition?.focal_length, icon: Camera, color: "#22d3ee" },
    { key: "composition.framing", label: "构图", value: rp.composition?.framing, icon: Camera, color: "#22d3ee" },
    { key: "composition.depth_of_field", label: "景深", value: rp.composition?.depth_of_field, icon: Camera, color: "#22d3ee" },
    { key: "entities", label: "主体与物体", value: formatStructuredValue(rp.entities), icon: User, color: "#60a5fa" },
    { key: "environment_details.foreground", label: "前景", value: rp.environment_details?.foreground, icon: PicIcon, color: "#34d399" },
    { key: "environment_details.midground", label: "中景", value: rp.environment_details?.midground, icon: PicIcon, color: "#34d399" },
    { key: "environment_details.background", label: "背景", value: rp.environment_details?.background, icon: PicIcon, color: "#34d399" },
    { key: "technical_specs.texture_fidelity", label: "材质细节", value: rp.technical_specs?.texture_fidelity, icon: Aim, color: "#2dd4bf" },
    { key: "technical_specs.render_engine_style", label: "技术质感", value: rp.technical_specs?.render_engine_style, icon: Aim, color: "#2dd4bf" },
    { key: "technical_specs.vfx", label: "视觉效果", value: formatStructuredValue(rp.technical_specs?.vfx), icon: Aim, color: "#2dd4bf" },
    { key: "embedded_text", label: "画面文字", value: rp.embedded_text, icon: EditPen, color: "#a78bfa" },
  ].filter((field) => field.value);
}

async function addFiles() {
  const files = await api.openFiles();
  if (!files.length) return;
  const existing = new Set(queue.value.map((q) => q.filePath));
  for (const fp of files) {
    if (existing.has(fp)) continue;
    const item: BatchItem = { id: uid(), filePath: fp, fileName: pathBasename(fp), status: "pending", progress: 0, result: null, error: null, thumbUrl: "" };
    queue.value.push(item);
    api.readFileAsDataUrl(fp).then((url) => (item.thumbUrl = url)).catch(() => {});
  }
}

async function addFolder() {
  const folder = await api.openFolder();
  if (!folder) return;
  const files = await api.scanFolder(folder);
  if (!files.length) return;
  const existing = new Set(queue.value.map((q) => q.filePath));
  for (const fp of files) {
    if (existing.has(fp)) continue;
    const item: BatchItem = { id: uid(), filePath: fp, fileName: pathBasename(fp), status: "pending", progress: 0, result: null, error: null, thumbUrl: "" };
    queue.value.push(item);
    api.readFileAsDataUrl(fp).then((url) => (item.thumbUrl = url)).catch(() => {});
  }
}

async function startBatch() {
  if (!settingsStore.loaded) {
    await settingsStore.load();
  }
  if (!settingsStore.settings.apiKey || settingsStore.settings.apiKey.trim() === "") {
    ElMessage.error("API Key 未配置，请先到设置中心填写。");
    return;
  }
  if (!settingsStore.settings.model || settingsStore.settings.model.trim() === "") {
    ElMessage.error("模型名称未配置，请先到设置中心填写。");
    return;
  }

  running.value = true;
  paused.value = false;
  doneCount.value = 0;
  failedCount.value = 0;
  const concurrency = settingsStore.settings.concurrency || 2;
  const pending = queue.value.filter((q) => q.status === "pending");
  let index = 0;

  const runNext = async () => {
    while (index < pending.length && running.value) {
      if (paused.value) {
        await new Promise((r) => {
          const check = setInterval(() => { if (!paused.value || !running.value) { clearInterval(check); r(undefined); } }, 200);
        });
        if (!running.value) return;
      }
      const item = pending[index++];
      item.status = "processing";
      currentItem.value = item;
      const t0 = Date.now();
      try {
        const result = await api.analyzeImage(
          { id: item.id, sourceType: "file", filePath: item.filePath, fileName: item.fileName },
          settingsStore.settings
        );
        item.status = "done";
        item.result = result;
        doneCount.value++;
        const elapsed = Date.now() - t0;
        avgTime.value = avgTime.value ? (avgTime.value + elapsed) / 2 : elapsed;
      } catch (err: any) {
        item.status = "failed";
        item.error = err?.message || "失败";
        currentItem.value = item;
        failedCount.value++;
      }
    }
  };

  await Promise.all(Array.from({ length: Math.min(concurrency, pending.length) }, () => runNext()));
  running.value = false;
  // keep currentItem so user can still see the last result; allow switching by clicking thumbnails
}

function viewItem(item: BatchItem) {
  if (item.status === "done" || item.status === "failed") {
    currentItem.value = item;
  }
}

function togglePause() { paused.value = !paused.value; }
function cancelBatch() {
  running.value = false;
  paused.value = false;
  currentItem.value = null;
}
function clearQueue() {
  if (running.value) return;
  queue.value = [];
  doneCount.value = 0;
  failedCount.value = 0;
}

function updateStructFieldForCurrent(key: string, value: string) {
  const r: any = currentItem.value?.result;
  if (!r?.reconstructed_prompt) return;
  if (key.includes(".") || key === "entities" || key === "embedded_text") {
    setStructuredValue(r.reconstructed_prompt, key, value);
    return;
  }
  const useZh = currentLang.value === "zh" && r.reconstructed_prompt_zh;
  const target = useZh ? r.reconstructed_prompt_zh : r.reconstructed_prompt;
  target[key] = value;
}

function setStructuredValue(target: any, key: string, value: string) {
  const parsed = parseStructuredEdit(key, value);
  if (!key.includes(".")) {
    target[key] = parsed;
    return;
  }
  const parts = key.split(".");
  let cursor = target;
  for (const part of parts.slice(0, -1)) {
    if (!cursor[part] || typeof cursor[part] !== "object") cursor[part] = {};
    cursor = cursor[part];
  }
  cursor[parts[parts.length - 1]] = parsed;
}

function parseStructuredEdit(key: string, value: string) {
  const trimmed = value.trim();
  if (key === "global_scene.color_palette" || key === "technical_specs.vfx") {
    return trimmed ? trimmed.split(/[、,\n]/).map((item) => item.trim()).filter(Boolean) : [];
  }
  if (key === "entities") {
    try {
      return JSON.parse(trimmed);
    } catch {
      return trimmed
        ? [{ label: "主体", appearance: trimmed, pose: { action_description: "", body_language: "", spatial_position: "" }, sub_elements: [] }]
        : [];
    }
  }
  return value;
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Top: Queue Header + Strip -->
    <div data-tauri-drag-region class="shrink-0 px-8 pt-7 pb-5 border-b border-white/[0.06]">
      <div class="flex items-center justify-between mb-5">
        <div data-tauri-drag-region class="flex items-center gap-3">
          <h2 class="text-[22px] font-semibold text-white/90">分析队列</h2>
          <el-tag v-if="queue.length" size="large" type="info">
            {{ doneCount + failedCount }}/{{ queue.length }}
          </el-tag>
        </div>
        <div class="flex gap-3">
          <el-button size="large" @click="addFiles">
            <el-icon class="mr-1.5"><Plus /></el-icon>添加图片
          </el-button>
          <el-button size="large" @click="addFolder">
            <el-icon class="mr-1.5"><FolderOpened /></el-icon>添加文件夹
          </el-button>
          <el-button size="large" type="danger" plain :disabled="!queue.length || running" @click="clearQueue">
            <el-icon class="mr-1.5"><Close /></el-icon>清空
          </el-button>
          <el-divider direction="vertical" style="height: 32px; margin: 0 4px;" />
          <el-button
            v-if="!running"
            type="primary"
            size="large"
            :disabled="!queue.length"
            @click="startBatch"
          >
            <el-icon class="mr-1.5"><VideoPlay /></el-icon>开始分析
          </el-button>
          <el-button
            v-if="running"
            :type="paused ? 'primary' : 'warning'"
            size="large"
            plain
            @click="togglePause"
          >
            <el-icon class="mr-1.5"><VideoPause v-if="!paused" /><VideoPlay v-else /></el-icon>
            {{ paused ? '继续' : '暂停' }}
          </el-button>
          <el-button v-if="running" type="danger" size="large" plain @click="cancelBatch">
            <el-icon class="mr-1.5"><Close /></el-icon>取消
          </el-button>
        </div>
      </div>

      <!-- Horizontal thumbnails -->
      <div v-if="queue.length" class="flex gap-3 overflow-x-auto pb-2">
        <div
          v-for="(item, idx) in queue"
          :key="item.id"
          @click="viewItem(item)"
          class="relative shrink-0 w-[120px] h-[88px] rounded-xl overflow-hidden border-2 transition-all duration-300"
          :class="[
            currentItem?.id === item.id ? 'ring-2 ring-teal-400/60' : '',
            (item.status === 'done' || item.status === 'failed') ? 'cursor-pointer hover:scale-[1.03]' : '',
            {
              'border-teal-400/55 shadow-[0_0_16px_rgba(45,212,191,0.25)]': item.status === 'processing',
              'border-emerald-400/30': item.status === 'done',
              'border-red-400/30': item.status === 'failed',
              'border-white/[0.08]': item.status === 'pending',
            }
          ]"
        >
          <img v-if="item.thumbUrl" :src="item.thumbUrl" class="w-full h-full object-cover" alt="" />
          <div v-else class="w-full h-full bg-white/[0.03]" />
          <div class="absolute top-1.5 left-1.5 px-1.5 h-5 rounded-md bg-[rgba(10,10,15,0.8)] flex items-center text-[11px] font-bold text-white/75">
            {{ String(idx + 1).padStart(2, "0") }}
          </div>
          <div class="absolute bottom-1.5 right-1.5">
            <div v-if="item.status === 'done'" class="w-6 h-6 rounded-full bg-emerald-500 flex items-center justify-center">
              <el-icon :size="14" color="white"><Check /></el-icon>
            </div>
            <div v-else-if="item.status === 'processing'" class="w-6 h-6 rounded-full bg-teal-500 flex items-center justify-center animate-pulse">
              <el-icon :size="13" color="white" class="animate-spin"><Loading /></el-icon>
            </div>
            <div v-else-if="item.status === 'failed'" class="w-6 h-6 rounded-full bg-red-500 flex items-center justify-center">
              <el-icon :size="14" color="white"><Close /></el-icon>
            </div>
            <div v-else class="w-6 h-6 rounded-full bg-white/[0.12] flex items-center justify-center">
              <el-icon :size="12" color="rgba(255,255,255,0.4)"><Clock /></el-icon>
            </div>
          </div>
        </div>
      </div>

      <el-empty
        v-else
        description="添加图片开始批量分析"
        :image-size="60"
      />
    </div>

    <!-- Middle: Current analysis -->
    <div class="flex-1 min-h-0 flex gap-6 p-7">
      <!-- Left: Current image -->
      <div class="w-[48%] flex flex-col">
        <h3 class="text-[18px] font-semibold text-white/85 mb-4">当前分析图像</h3>
        <el-card class="flex-1" body-style="height:100%;padding:14px">
          <div class="relative h-full rounded-xl overflow-hidden bg-[#0a0a12]">
            <img
              v-if="currentItem?.thumbUrl"
              :src="currentItem.thumbUrl"
              class="w-full h-full object-contain"
              alt=""
            />
            <el-empty
              v-else
              description="等待开始分析..."
              :image-size="64"
              class="!h-full !flex !flex-col !items-center !justify-center"
            />
            <el-tag
              v-if="currentItem?.status === 'processing'"
              type="success"
              effect="dark"
              size="large"
              class="!absolute !top-4 !right-4"
            >
              <div class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-teal-300 animate-pulse" />
                AI 分析中
              </div>
            </el-tag>
          </div>
        </el-card>
      </div>

      <!-- Right: Real-time results -->
      <div class="flex-1 flex flex-col min-w-0">
        <h3 class="text-[18px] font-semibold text-white/85 mb-4">实时分析结果</h3>
        <el-card class="flex-1" body-style="height:100%;padding:24px;overflow-y:auto">
          <div v-if="currentItem?.result" class="space-y-8">
            <!-- Top: lang toggle + copy -->
            <div class="flex items-center justify-between">
              <el-radio-group v-model="currentLang" size="default">
                <el-radio-button value="zh">中文</el-radio-button>
                <el-radio-button value="en">English</el-radio-button>
              </el-radio-group>
              <el-button size="default" @click="copyPromptBatch">
                <el-icon class="mr-1"><CopyDocument /></el-icon>复制
              </el-button>
            </div>

            <!-- Full prompt (top) -->
            <div>
              <p class="text-[14px] font-semibold text-white/80 mb-3">
                {{ currentLang === 'zh' ? '完整提示词（中文）' : '完整提示词（English）' }}
              </p>
              <textarea
                v-model="promptText"
                class="prompt-editor"
                spellcheck="false"
              />
            </div>

            <!-- Structured fields (bottom) -->
            <template v-if="structFieldsForCurrent.length">
              <div class="pt-6 border-t border-white/[0.08]">
                <p class="text-[14px] font-semibold text-white/80 mb-6">结构化拆解</p>
                <div class="space-y-7">
                  <div v-for="field in structFieldsForCurrent" :key="field.key" class="flex items-start gap-4">
                    <div class="w-11 h-11 rounded-xl flex items-center justify-center shrink-0" :style="{ backgroundColor: field.color + '22' }">
                      <el-icon :size="20" :color="field.color"><component :is="field.icon" /></el-icon>
                    </div>
                    <div class="flex-1 min-w-0 pt-1">
                      <p class="text-[13px] font-semibold text-white/55 mb-2">{{ field.label }}</p>
                      <textarea
                        :value="field.value"
                        class="struct-editor"
                        spellcheck="false"
                        @input="updateStructFieldForCurrent(field.key, ($event.target as HTMLTextAreaElement).value)"
                      />
                    </div>
                    <el-icon :size="20" color="#34d399" class="shrink-0 mt-2"><Check /></el-icon>
                  </div>
                </div>
              </div>
            </template>
          </div>
          <div v-else-if="currentItem?.status === 'failed'" class="h-full flex items-center justify-center">
            <el-alert
              type="error"
              title="分析失败"
              :description="currentItem.error || '请检查 API Key、模型名称、网络连接或图片文件是否可读取。'"
              :closable="false"
              show-icon
            />
          </div>
          <el-empty
            v-else
            :description="currentItem?.status === 'processing' ? '正在分析中...' : '等待分析结果'"
            :image-size="80"
            class="!h-full !flex !flex-col !items-center !justify-center"
          />
        </el-card>
      </div>
    </div>

    <!-- Bottom: Progress only -->
    <div class="shrink-0 px-8 py-3 border-t border-white/[0.06]">
      <div class="flex items-center gap-5 mb-2">
        <span class="text-[14px] text-white/65">
          已完成 <strong class="text-white/90">{{ doneCount + failedCount }}/{{ queue.length }}</strong>
        </span>
        <span v-if="remainingTime" class="text-[12px] text-white/40">预计剩余 {{ remainingTime }}</span>
        <span v-if="avgTime" class="text-[12px] text-white/40">平均耗时 {{ (avgTime / 1000).toFixed(0) }}s</span>
        <span class="ml-auto text-[14px] font-semibold text-teal-300">{{ totalPct }}%</span>
      </div>
      <el-progress
        :percentage="totalPct"
        :show-text="false"
        :stroke-width="8"
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
:deep(.el-empty__description) {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.4);
}
.prompt-editor {
  width: 100%;
  min-height: 220px;
  resize: vertical;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.045);
  padding: 14px 16px;
  color: rgba(255, 255, 255, 0.78);
  font-size: 14px;
  line-height: 1.85;
  outline: none;
}
.prompt-editor:focus {
  border-color: rgba(45, 212, 191, 0.45);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.1);
}
.struct-editor {
  width: 100%;
  min-height: 88px;
  resize: vertical;
  border: 1px solid transparent;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.035);
  padding: 10px 12px;
  color: rgba(255, 255, 255, 0.82);
  font-size: 15px;
  line-height: 1.75;
  outline: none;
}
.struct-editor:focus {
  border-color: rgba(45, 212, 191, 0.42);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.09);
}
</style>
