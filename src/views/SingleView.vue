<script setup lang="ts">
import { ref, computed } from "vue";
import { useAnalysisStore } from "@/stores/analysis";
import { api } from "@/lib/api";
import { uid, pathBasename, urlBasename } from "@/lib/utils";
import RadarChart from "@/components/RadarChart.vue";
import {
  Upload as ElUploadIcon, Document, Link, CopyDocument, Check, RefreshLeft,
  Download as DownloadIcon, Star, Delete, Picture as PicIcon, User, Sunny,
  Camera, EditPen, Aim
} from "@element-plus/icons-vue";

const store = useAnalysisStore();
const inputMode = ref<"file" | "clipboard" | "url">("file");
const urlValue = ref("");
const currentLang = ref<"zh" | "en">("zh");
const copied = ref(false);
const dragOver = ref(false);

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
    const filePath = files[0];
    const dataUrl = await api.readFileAsDataUrl(filePath);
    store.analyze({ id: uid(), sourceType: "file", filePath, fileName: pathBasename(filePath) }, dataUrl);
  }
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;
  const file = e.dataTransfer?.files?.[0];
  if (file) {
    const filePath = (file as any).path;
    if (filePath) {
      api.readFileAsDataUrl(filePath).then((dataUrl) => {
        store.analyze({ id: uid(), sourceType: "file", filePath, fileName: file.name }, dataUrl);
      });
    }
  }
}

function handlePaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  for (const item of items) {
    if (item.type.startsWith("image/")) {
      const blob = item.getAsFile();
      if (!blob) continue;
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        const base64 = dataUrl.split(",")[1];
        const mimeType = dataUrl.split(";")[0].split(":")[1];
        store.analyze(
          { id: uid(), sourceType: "clipboard", base64Data: base64, mimeType, fileName: "clipboard.png" },
          dataUrl
        );
      };
      reader.readAsDataURL(blob);
      return;
    }
  }
}

function handleUrl() {
  const url = urlValue.value.trim();
  if (!url) return;
  store.analyze({ id: uid(), sourceType: "url", imageUrl: url, fileName: urlBasename(url) }, url);
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
  const useZh = currentLang.value === "zh" && r.reconstructed_prompt_zh;
  const target = useZh ? r.reconstructed_prompt_zh : r.reconstructed_prompt;
  target[key] = value;
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Top bar: input mode selector (only when no result) -->
    <div v-if="!store.result && !store.analyzing" data-tauri-drag-region class="px-10 pt-8 pb-6 flex items-center gap-6">
      <span class="text-[17px] font-semibold text-white/70">选择输入方式</span>
      <el-radio-group v-model="inputMode" size="large">
        <el-radio-button value="file">
          <el-icon class="mr-2"><ElUploadIcon /></el-icon>本地上传
        </el-radio-button>
        <el-radio-button value="clipboard">
          <el-icon class="mr-2"><Document /></el-icon>粘贴图片
        </el-radio-button>
        <el-radio-button value="url">
          <el-icon class="mr-2"><Link /></el-icon>图片URL
        </el-radio-button>
      </el-radio-group>
    </div>

    <!-- URL Input bar -->
    <div v-if="!store.result && !store.analyzing && inputMode === 'url'" class="px-10 pb-4 flex gap-3">
      <el-input
        v-model="urlValue"
        size="large"
        placeholder="https://example.com/image.jpg"
        :prefix-icon="Link"
        @keydown.enter="handleUrl"
        class="flex-1"
      />
      <el-button type="primary" size="large" @click="handleUrl">
        <el-icon class="mr-1"><Star /></el-icon>开始分析
      </el-button>
    </div>

    <!-- Upload Area (file) -->
    <div v-if="!store.result && !store.analyzing && inputMode === 'file'" class="flex-1 px-10 pb-10 min-h-0 flex items-center justify-center">
      <div
        @click="handleFileClick"
        @dragover.prevent="dragOver = true"
        @dragleave="dragOver = false"
        @drop="handleDrop"
        class="w-full max-w-[1100px] h-[520px] flex flex-col items-center justify-center border-2 border-dashed rounded-[28px] cursor-pointer transition-all duration-300 bg-gradient-to-b from-white/[0.03] to-white/[0.01]"
        :class="dragOver
          ? 'border-teal-400/55 bg-teal-400/[0.05]'
          : 'border-white/[0.2] hover:border-teal-400/45 hover:bg-white/[0.035]'"
      >
        <div class="w-[120px] h-[120px] rounded-[28px] bg-gradient-to-br from-teal-500/22 to-blue-500/14 border border-white/[0.12] flex items-center justify-center mb-7 shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_12px_36px_rgba(45,212,191,0.18)]">
          <el-icon :size="56" color="#5eead4"><ElUploadIcon /></el-icon>
        </div>
        <p class="text-[26px] font-semibold text-white/90 mb-3">拖拽图片到此处</p>
        <p class="text-[16px] text-white/40 mb-5">或</p>
        <el-button type="primary" plain size="large">
          <el-icon class="mr-1.5"><ElUploadIcon /></el-icon>点击选择文件
        </el-button>
        <p class="text-[14px] text-white/40 mt-6">支持 JPG、PNG、WebP 格式，最大 20MB</p>
      </div>
    </div>

    <!-- Paste Zone -->
    <div v-if="!store.result && !store.analyzing && inputMode === 'clipboard'" class="flex-1 px-10 pb-10 min-h-0 flex items-center justify-center">
      <div
        @paste="handlePaste"
        tabindex="0"
        class="w-full max-w-[1100px] h-[520px] flex flex-col items-center justify-center border-2 border-dashed border-white/[0.2] rounded-[28px] cursor-pointer outline-none focus:border-blue-400/45 focus:bg-blue-400/[0.04] transition-all duration-300 bg-gradient-to-b from-white/[0.03] to-white/[0.01]"
      >
        <div class="w-[120px] h-[120px] rounded-[28px] bg-gradient-to-br from-blue-500/22 to-violet-500/14 border border-white/[0.12] flex items-center justify-center mb-7 shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_12px_36px_rgba(59,130,246,0.18)]">
          <el-icon :size="56" color="#93c5fd"><Document /></el-icon>
        </div>
        <p class="text-[26px] font-semibold text-white/90 mb-3">按 Ctrl+V 粘贴截图</p>
        <p class="text-[15px] text-white/45">点击此区域后粘贴剪贴板中的图片</p>
      </div>
    </div>

    <!-- Progress -->
    <div v-if="store.analyzing" class="flex-1 flex items-center justify-center px-10">
      <div class="w-full max-w-[480px] text-center">
        <el-progress
          type="circle"
          :percentage="store.progress.percent"
          :width="120"
          :stroke-width="6"
          color="#2dd4bf"
        />
        <p class="text-[16px] font-semibold text-white/75 mt-6">{{ store.progress.text }}</p>
      </div>
    </div>

    <!-- Error -->
    <div v-if="store.error && !store.analyzing && !store.result" class="flex-1 flex items-center justify-center px-10">
      <div class="w-full max-w-2xl">
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
              <p>· 模型名称错误 → 检查"模型"字段（如 gemini-2.5-flash / gpt-4o）</p>
              <p>· 网络无法访问 API → 尤其 Gemini 可能需要科学上网</p>
              <p>· Base URL 错误（OpenAI 兼容接口）→ 检查格式（如 https://api.openai.com/v1）</p>
            </div>
            <el-button type="primary" size="default" @click="store.reset()">
              <el-icon class="mr-1"><RefreshLeft /></el-icon>重新选择
            </el-button>
          </template>
        </el-alert>
      </div>
    </div>

    <!-- Result View -->
    <div v-if="store.result" class="flex-1 min-h-0 flex gap-6 px-8 py-6">
      <!-- Left: Image Preview -->
      <div class="w-[40%] shrink-0">
        <el-card class="!h-full" body-style="height:100%;padding:12px">
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
      <div class="flex-1 min-w-0 overflow-y-auto space-y-4 pr-1">
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
                <span class="text-[56px] font-bold text-teal-300 leading-none">{{ store.result.qualityScore }}</span>
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
    <div v-if="store.result" class="shrink-0 px-8 py-3 border-t border-white/[0.06] flex items-center justify-end gap-3">
      <el-button size="large">
        <el-icon class="mr-1"><RefreshLeft /></el-icon>重新分析
      </el-button>
      <el-button type="primary" size="large">
        <el-icon class="mr-1"><DownloadIcon /></el-icon>导出结果
      </el-button>
      <el-button type="warning" size="large" plain>
        <el-icon class="mr-1"><Star /></el-icon>加入收藏
      </el-button>
      <el-button type="danger" size="large" plain>
        <el-icon class="mr-1"><Delete /></el-icon>删除
      </el-button>
    </div>
  </div>
</template>

<style scoped>
:deep(.el-card) {
  background-color: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
}
:deep(.el-card__header) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  padding: 14px 20px;
}
:deep(.el-card__body) {
  padding: 18px 20px;
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
  line-height: 1.8;
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
  font-size: 14px;
  line-height: 1.75;
  outline: none;
}
.struct-editor:focus {
  border-color: rgba(45, 212, 191, 0.42);
  box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.09);
}
</style>
