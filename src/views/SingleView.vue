<script setup lang="ts">
import { ref, computed, onBeforeUnmount, onMounted } from "vue";
import { useAnalysisStore } from "@/stores/analysis";
import { api } from "@/lib/api";
import { uid, pathBasename, urlBasename } from "@/lib/utils";
import RadarChart from "@/components/RadarChart.vue";
import {
  Upload as ElUploadIcon, Document, Link, CopyDocument, Check, RefreshLeft,
  Download as DownloadIcon, Star, Back, Picture as PicIcon, User, Sunny,
  Camera, EditPen, Aim
} from "@element-plus/icons-vue";

const store = useAnalysisStore();
const preparing = ref(false);
const inputHub = ref<HTMLElement | null>(null);
const retryAction = ref<(() => Promise<void>) | null>(null);
const urlValue = ref("");
const currentLang = ref<"zh" | "en">("zh");
const copied = ref(false);
const dragOver = ref(false);
let unlistenTauriDrop: (() => void) | null = null;

onMounted(async () => {
  if (!(window as any).__TAURI_INTERNALS__) return;
  try {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    unlistenTauriDrop = await getCurrentWebview().onDragDropEvent((event) => {
      if (store.result || store.analyzing || preparing.value) return;
      if (event.payload.type === "enter" || event.payload.type === "over") {
        dragOver.value = true;
      } else if (event.payload.type === "leave") {
        dragOver.value = false;
      } else if (event.payload.type === "drop") {
        dragOver.value = false;
        const filePath = event.payload.paths?.[0];
        if (filePath) analyzeFilePath(filePath);
      }
    });
  } catch {
    // Browser preview keeps using regular drop events.
  }
});

onBeforeUnmount(() => {
  unlistenTauriDrop?.();
  unlistenTauriDrop = null;
});

const promptText = computed({
  get() {
    if (!store.result) return "";
    return currentLang.value === "zh"
      ? store.result.prompt_zh || store.result.prompt_en
      : store.result.prompt_en || store.result.prompt_zh;
  },
  set(value: string) {
    if (!store.result) return;
    if (currentLang.value === "zh") {
      store.result.prompt_zh = value;
    } else {
      store.result.prompt_en = value;
    }
  },
});

const structFields = computed(() => {
  const r: any = store.result;
  if (!r?.reconstructed_prompt) return [];
  const nextFields = structuredFieldsFromPrompt(r.reconstructed_prompt);
  if (nextFields.length) return nextFields;
  const useZh = currentLang.value === "zh" && r.reconstructed_prompt_zh;
  const rp = useZh ? r.reconstructed_prompt_zh : r.reconstructed_prompt;
  const labels = useZh
    ? { style: "风格", subject: "主体", context: "场景", lighting: "光影", camera: "镜头" }
    : { style: "style_prefix", subject: "subject", context: "context_and_background", lighting: "lighting", camera: "camera_and_composition" };
  return [
    { key: "style_prefix", label: labels.style, value: rp.style_prefix, icon: EditPen, color: "#a78bfa" },
    { key: "subject", label: labels.subject, value: rp.subject, icon: User, color: "#60a5fa" },
    { key: "context_and_background", label: labels.context, value: rp.context_and_background, icon: PicIcon, color: "#34d399" },
    { key: "lighting", label: labels.lighting, value: rp.lighting, icon: Sunny, color: "#fbbf24" },
    { key: "camera_and_composition", label: labels.camera, value: rp.camera_and_composition, icon: Camera, color: "#22d3ee" },
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

const qualityDimensions = computed(() => {
  if (!store.result?.qualityBreakdown) return [];
  const bd = store.result.qualityBreakdown;
  const dims = [
    { key: "subject", label: "主体 (Subject)", value: bd.subject, icon: User, color: "from-blue-500 to-blue-400" },
    { key: "context", label: "场景 (Context)", value: bd.context, icon: PicIcon, color: "from-emerald-500 to-emerald-400" },
    { key: "lighting", label: "光照 (Lighting)", value: bd.lighting, icon: Sunny, color: "from-amber-500 to-amber-400" },
    { key: "camera", label: "构图 (Camera)", value: bd.camera, icon: Camera, color: "from-cyan-500 to-cyan-400" },
    { key: "text", label: "文本 (Text)", value: bd.text, icon: EditPen, color: "from-violet-500 to-violet-400" },
    { key: "imagen", label: "图像契合度 (Imagen Fit)", value: bd.imagen, icon: Aim, color: "from-teal-500 to-teal-400" },
  ];
  return dims.filter((d) => d.value != null);
});

const radarData = computed(() =>
  qualityDimensions.value.map((d) => ({ label: d.label.split(" ")[0], value: d.value || 0 }))
);

const qualityDescription = computed(() => {
  if (!store.result) return "";
  const s = store.result.qualityScore;
  if (s >= 90) return "整体质量极高，各维度表现均衡出色";
  if (s >= 78) return "整体质量良好，细节丰富，构图与光影表现出色";
  if (s >= 64) return "质量可用，部分维度有提升空间";
  return "质量偏低，建议优化后重新分析";
});

async function handleFileClick() {
  const files = await api.openFiles();
  if (files.length) {
    await analyzeFilePath(files[0]);
  }
}

async function analyzeFilePath(filePath: string) {
  retryAction.value = () => analyzeFilePath(filePath);
  preparing.value = true;
  try {
    const dataUrl = await api.readFileAsDataUrl(filePath);
    await store.analyze({ id: uid(), sourceType: "file", filePath, fileName: pathBasename(filePath) }, dataUrl);
  } finally {
    preparing.value = false;
  }
}

async function analyzeDataUrl(file: File, dataUrl: string) {
  retryAction.value = () => analyzeDataUrl(file, dataUrl);
  preparing.value = true;
  try {
    const base64 = dataUrl.split(",")[1];
    const mimeType = dataUrl.split(";")[0].split(":")[1] || file.type || "image/png";
    await store.analyze(
      { id: uid(), sourceType: "clipboard", base64Data: base64, mimeType, fileName: file.name || "dropped-image.png" },
      dataUrl
    );
  } finally {
    preparing.value = false;
  }
}
async function handleDrop(e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  dragOver.value = false;
  const file = e.dataTransfer?.files?.[0];
  if (file) {
    const filePath = (file as any).path;
    if (filePath) {
      await analyzeFilePath(filePath);
      return;
    }

    const reader = new FileReader();
    reader.onload = () => analyzeDataUrl(file, reader.result as string);
    reader.readAsDataURL(file);
  }
}

function handleDragLeave(e: DragEvent) {
  const current = e.currentTarget as HTMLElement | null;
  const related = e.relatedTarget as Node | null;
  if (current && related && current.contains(related)) return;
  dragOver.value = false;
}

function handlePaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  for (const item of items) {
    if (item.type.startsWith("image/")) {
      const blob = item.getAsFile();
      if (!blob) continue;
      const reader = new FileReader();
      reader.onload = () => analyzeDataUrl(blob, reader.result as string);
      reader.readAsDataURL(blob);
      return;
    }
  }
}

async function analyzeUrl(url: string) {
  retryAction.value = () => analyzeUrl(url);
  await store.analyze({ id: uid(), sourceType: "url", imageUrl: url, fileName: urlBasename(url) }, url);
}

function focusInputHub() {
  inputHub.value?.focus();
}

function handleUrl() {
  const url = urlValue.value.trim();
  if (!url) return;
  void analyzeUrl(url);
}

async function retryAnalysis() {
  if (retryAction.value) await retryAction.value();
}

function resetForNewInput() {
  retryAction.value = null;
  urlValue.value = "";
  store.reset();
}
async function copyPrompt() {
  if (!promptText.value) return;
  await navigator.clipboard.writeText(promptText.value);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}

function updateStructField(key: string, value: string) {
  const r: any = store.result;
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
    <div v-if="!store.result && !store.analyzing && !preparing" data-tauri-drag-region class="shrink-0 px-8 pt-6 pb-5 flex items-center justify-between">
      <div>
        <span class="text-[15px] font-semibold text-white/78">输入图片</span>
        <span class="ml-3 text-[12px] text-white/38">拖入、粘贴或使用图片 URL</span>
      </div>
    </div>

    <div v-if="!store.result && !store.analyzing && !preparing" class="flex-1 px-8 pb-8 min-h-0 flex items-center justify-center">
      <div
        ref="inputHub"
        tabindex="0"
        @click="focusInputHub"
        @paste="handlePaste"
        class="input-hub"
      >
        <div
          @click.stop="handleFileClick"
          @dragover.prevent="dragOver = true"
          @dragleave="handleDragLeave"
          @drop.prevent="handleDrop"
          class="input-hub__dropzone"
          :class="dragOver ? 'is-dragging' : ''"
        >
          <div class="input-hub__upload-icon">
            <el-icon :size="42" color="#5eead4"><ElUploadIcon /></el-icon>
          </div>
          <p class="text-[22px] font-semibold text-white/90 mb-2">拖拽图片到此处</p>
          <p class="text-[14px] text-white/45 mb-4">或</p>
          <el-button type="primary" plain size="default">
            <el-icon class="mr-1.5"><ElUploadIcon /></el-icon>点击选择文件
          </el-button>
          <p class="text-[13px] text-white/45 mt-5">支持 JPG、PNG、WebP 格式，最大 20MB</p>
        </div>

        <div class="input-hub__tools">
          <el-button plain @click.stop="focusInputHub">
            <el-icon class="mr-1"><Document /></el-icon>粘贴图片
          </el-button>
          <div class="input-url-row">
            <el-input v-model="urlValue" placeholder="粘贴图片 URL" clearable @keydown.enter="handleUrl">
              <template #prefix><el-icon><Link /></el-icon></template>
            </el-input>
            <el-button type="primary" @click.stop="handleUrl">
              <el-icon class="mr-1"><Link /></el-icon>开始分析
            </el-button>
          </div>
        </div>
      </div>
    </div>

    <!-- Progress -->
    <div v-if="store.analyzing || preparing" class="flex-1 flex items-center justify-center px-10">
      <div class="w-full max-w-[480px] text-center">
        <el-progress
          type="circle"
          :percentage="preparing ? 2 : store.progress.percent"
          :width="120"
          :stroke-width="6"
          color="#2dd4bf"
        />
        <p class="text-[16px] font-semibold text-white/75 mt-6">{{ preparing ? "正在读取图片..." : store.progress.text }}</p>
      </div>
    </div>

    <!-- Error -->
    <div v-if="store.error && !store.analyzing && !preparing && !store.result" class="flex-1 flex items-center justify-center px-10">
      <div class="w-full max-w-2xl">
        <div v-if="store.previewSrc" class="retry-preview mb-4">
          <img :src="store.previewSrc" alt="" class="retry-preview__image" />
          <div>
            <p class="text-[14px] font-semibold text-white/78">图片已保留</p>
            <p class="text-[12px] text-white/42 mt-1">无需重新粘贴，点击下方按钮再次分析</p>
          </div>
        </div>
        <el-alert
          type="error"
          :title="'分析失败'"
          :closable="false"
          show-icon
        >
          <template #default>
            <p class="text-[14px] leading-relaxed mb-3 break-words">{{ store.error }}</p>
            <div class="text-[12px] text-white/55 mb-4 space-y-1">
              <p>💡 可能原因：</p>
              <p>· API 密钥未配置或错误 → 设置中心填写正确的 API Key</p>
              <p>· 模型名称错误 → 检查“模型”字段（如 gemini-2.5-flash / gpt-4o）</p>
              <p>· 网络无法访问 API → 尤其 Gemini 可能需要科学上网</p>
              <p>· Base URL 错误（OpenAI 兼容接口）→ 检查格式（如 https://api.openai.com/v1）</p>
            </div>
            <div class="flex gap-2 flex-wrap">
              <el-button v-if="retryAction" type="primary" size="default" @click="retryAnalysis">
                <el-icon class="mr-1"><RefreshLeft /></el-icon>重新分析
              </el-button>
              <el-button size="default" @click="resetForNewInput">
                <el-icon class="mr-1"><RefreshLeft /></el-icon>重新选择
              </el-button>
            </div>
          </template>
        </el-alert>
      </div>
    </div>

    <!-- Result View -->
    <div v-if="store.result" class="flex-1 min-h-0 flex gap-5 px-6 py-5">
      <!-- Left: Image Preview -->
      <div class="w-[38%] shrink-0">
        <el-card class="!h-full" body-style="height:100%;padding:10px">
          <div class="relative h-full rounded-xl overflow-hidden bg-[#0a0a12]">
            <img
              v-if="store.previewSrc"
              :src="store.previewSrc"
              class="w-full h-full object-contain"
              alt=""
            />
            <div class="absolute top-3 left-3 flex flex-col gap-2">
              <el-tag v-if="store.result.aspect_ratio" type="info" effect="dark" size="large">
                <el-icon class="mr-1"><PicIcon /></el-icon>{{ store.result.aspect_ratio }}
              </el-tag>
              <el-tag v-if="store.result.contains_people !== undefined" type="info" effect="dark" size="large">
                <el-icon class="mr-1"><User /></el-icon>contains people: {{ store.result.contains_people ? 'yes' : 'no' }}
              </el-tag>
            </div>
          </div>
        </el-card>
      </div>

      <!-- Right: Results -->
      <div class="flex-1 min-w-0 overflow-y-auto space-y-3 pr-1">
        <!-- 1. Quality Score -->
        <el-card>
          <template #header>
            <div class="flex items-center gap-2.5">
              <div class="w-7 h-7 rounded-lg bg-teal-500/20 flex items-center justify-center text-[13px] font-bold text-teal-300">1</div>
              <span class="text-[16px] font-semibold text-white/90">质量评分</span>
            </div>
          </template>
          <div class="flex gap-5">
            <div class="flex-1 min-w-0">
              <div class="flex items-end gap-3 mb-1">
                <span class="text-[48px] font-bold text-teal-300 leading-none">{{ store.result.qualityScore }}</span>
                <span class="text-[15px] font-semibold text-teal-300/75 pb-1.5">{{ store.result.qualityLabel }}</span>
              </div>
              <p class="text-[12px] text-white/40 mb-4">{{ qualityDescription }}</p>
              <div class="space-y-3">
                <div v-for="dim in qualityDimensions" :key="dim.key" class="flex items-center gap-3">
                  <el-icon :size="16" color="rgba(255,255,255,0.5)"><component :is="dim.icon" /></el-icon>
                  <span class="text-[13px] text-white/55 w-[150px] shrink-0">{{ dim.label }}</span>
                  <el-progress
                    :percentage="dim.value || 0"
                    :show-text="false"
                    :stroke-width="8"
                    class="flex-1"
                  />
                  <span class="text-[13px] font-semibold text-white/70 w-9 text-right">{{ dim.value }}</span>
                </div>
              </div>
            </div>
            <div class="shrink-0">
              <RadarChart :data="radarData" :size="180" />
            </div>
          </div>
        </el-card>

        <!-- 2. Complete Prompt -->
        <el-card>
          <template #header>
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div class="w-7 h-7 rounded-lg bg-teal-500/20 flex items-center justify-center text-[13px] font-bold text-teal-300">2</div>
                <span class="text-[16px] font-semibold text-white/90">完整提示词</span>
                <el-radio-group v-model="currentLang" size="small" class="ml-3">
                  <el-radio-button value="zh">中文</el-radio-button>
                  <el-radio-button value="en">English</el-radio-button>
                </el-radio-group>
              </div>
              <el-button :type="copied ? 'success' : 'default'" size="small" @click="copyPrompt">
                <el-icon class="mr-1"><Check v-if="copied" /><CopyDocument v-else /></el-icon>
                {{ copied ? '已复制!' : '复制' }}
              </el-button>
            </div>
          </template>
          <textarea
            v-model="promptText"
            class="prompt-editor"
            spellcheck="false"
          />
        </el-card>

        <!-- 3. Structured Prompt -->
        <el-card v-if="structFields.length">
          <template #header>
            <div class="flex items-center gap-2.5">
              <div class="w-7 h-7 rounded-lg bg-teal-500/20 flex items-center justify-center text-[13px] font-bold text-teal-300">3</div>
              <span class="text-[16px] font-semibold text-white/90">结构化提示词解析</span>
            </div>
          </template>
          <el-descriptions :column="1" border>
            <el-descriptions-item
              v-for="field in structFields"
              :key="field.key"
            >
              <template #label>
                <span class="inline-flex items-center gap-2">
                  <el-icon :size="14" :color="field.color"><component :is="field.icon" /></el-icon>
                  {{ field.label }}
                </span>
              </template>
              <textarea
                :value="field.value"
                class="struct-editor"
                spellcheck="false"
                @input="updateStructField(field.key, ($event.target as HTMLTextAreaElement).value)"
              />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>

        <!-- 4. Meta Data -->
        <el-card>
          <template #header>
            <div class="flex items-center gap-2.5">
              <div class="w-7 h-7 rounded-lg bg-teal-500/20 flex items-center justify-center text-[13px] font-bold text-teal-300">4</div>
              <span class="text-[16px] font-semibold text-white/90">元数据信息</span>
            </div>
          </template>
          <el-descriptions :column="2" border>
            <el-descriptions-item label="耗时">{{ (store.result.elapsedMs / 1000).toFixed(2) }} 秒</el-descriptions-item>
            <el-descriptions-item label="图像比例">{{ store.result.aspect_ratio }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </div>
    </div>

    <!-- Bottom Action Bar -->
    <div v-if="store.result" class="shrink-0 px-6 py-3 border-t border-white/[0.06] flex items-center justify-end gap-2.5 bg-black/10">
      <el-button size="default" @click="resetForNewInput">
        <el-icon class="mr-1"><RefreshLeft /></el-icon>重新分析
      </el-button>
      <el-button type="primary" size="default">
        <el-icon class="mr-1"><DownloadIcon /></el-icon>导出结果
      </el-button>
      <el-button type="warning" size="default" plain>
        <el-icon class="mr-1"><Star /></el-icon>加入收藏
      </el-button>
      <el-button size="default" plain @click="resetForNewInput">
        <el-icon class="mr-1"><Back /></el-icon>返回
      </el-button>
    </div>
  </div>
</template>

<style scoped>
:deep(.el-card) {
  background-color: rgba(14, 17, 23, 0.78);
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 12px;
  box-shadow: none;
}
:deep(.el-card__header) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  padding: 12px 16px;
}
:deep(.el-card__body) {
  padding: 16px;
}
:deep(.el-radio-button__inner) {
  background-color: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.6);
  font-weight: 500;
}
:deep(.el-radio-button.is-active .el-radio-button__inner) {
  background-color: rgba(45, 212, 191, 0.18) !important;
  border-color: rgba(45, 212, 191, 0.4) !important;
  color: #2dd4bf !important;
  box-shadow: -1px 0 0 0 rgba(45, 212, 191, 0.4) !important;
}
:deep(.el-descriptions__label) {
  background-color: rgba(255, 255, 255, 0.03) !important;
  color: rgba(255, 255, 255, 0.6) !important;
}
:deep(.el-descriptions__content) {
  color: rgba(255, 255, 255, 0.8) !important;
}
.input-hub {
  width: 100%;
  max-width: 1040px;
  padding: 16px;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 22px;
  background: rgba(16, 19, 26, 0.62);
  outline: none;
  transition: border-color 180ms ease, box-shadow 180ms ease;
}
.input-hub:focus-within {
  border-color: rgba(45, 212, 191, 0.42);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.08);
}
.input-hub__dropzone {
  min-height: 360px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px dashed rgba(255, 255, 255, 0.2);
  border-radius: 18px;
  cursor: pointer;
  background: rgba(16, 19, 26, 0.6);
  transition: border-color 180ms ease, background 180ms ease;
}
.input-hub__dropzone:hover,
.input-hub__dropzone.is-dragging {
  border-color: rgba(45, 212, 191, 0.55);
  background: rgba(45, 212, 191, 0.06);
}
.input-hub__upload-icon {
  width: 82px;
  height: 82px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 22px;
  border-radius: 22px;
  background: rgba(20, 184, 166, 0.12);
  border: 1px solid rgba(94, 234, 212, 0.18);
}
.input-hub__tools {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 4px 2px;
}
.input-url-row {
  display: flex;
  flex: 1;
  gap: 10px;
}
.retry-preview {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  background: rgba(16, 19, 26, 0.72);
}
.retry-preview__image {
  width: 72px;
  height: 72px;
  flex: 0 0 auto;
  border-radius: 8px;
  object-fit: cover;
}
@media (max-width: 900px) {
  .input-hub__tools {
    flex-direction: column;
    align-items: stretch;
  }
  .input-url-row {
    width: 100%;
  }
}.prompt-editor {
  width: 100%;
  min-height: 200px;
  resize: vertical;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.055);
  padding: 14px 16px;
  color: rgba(255, 255, 255, 0.84);
  font-size: 14px;
  line-height: 1.8;
  outline: none;
  user-select: text;
}
.prompt-editor:focus {
  border-color: rgba(45, 212, 191, 0.45);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.1);
}
.struct-editor {
  width: 100%;
  min-height: 76px;
  resize: vertical;
  border: 1px solid transparent;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.05);
  padding: 10px 12px;
  color: rgba(255, 255, 255, 0.86);
  font-size: 14px;
  line-height: 1.75;
  outline: none;
  user-select: text;
}
.struct-editor:focus {
  border-color: rgba(45, 212, 191, 0.42);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.09);
}
</style>
