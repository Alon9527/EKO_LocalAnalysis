<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useGalleryStore } from "@/stores/gallery";
import { useUpdateStore } from "@/stores/update";
import { ElMessage, ElMessageBox } from "element-plus";
import { Setting, Delete, Check, Refresh } from "@element-plus/icons-vue";

const isTauri = !!(window as any).__TAURI_INTERNALS__;
const checking = ref(false);
const currentVersion = ref("1.0.0");

// Read actual app version from Tauri on mount
if (isTauri) {
  import("@tauri-apps/api/app").then(({ getVersion }) => {
    getVersion().then((v) => { currentVersion.value = v; }).catch(() => {});
  });
}

const downloadProgress = ref(0);
const downloadStatus = ref("");
const downloadVisible = ref(false);

async function checkUpdate() {
  if (!isTauri) {
    ElMessage.warning("更新功能仅在桌面应用中可用");
    return;
  }
  checking.value = true;
  try {
    const update = await updateStore.check(false);
    if (!update) {
      ElMessage.success(`已是最新版本 v${currentVersion.value}`);
      updateStore.clear();
      return;
    }
    await ElMessageBox.confirm(
      `发现新版本 v${update.version}\n\n${update.body || "暂无更新说明"}`,
      "版本更新",
      { confirmButtonText: "立即更新", cancelButtonText: "稍后", dangerouslyUseHTMLString: false }
    );

    let totalBytes = 0;
    let downloaded = 0;
    downloadProgress.value = 0;
    downloadStatus.value = "准备下载...";
    downloadVisible.value = true;

    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          totalBytes = event.data.contentLength || 0;
          downloadStatus.value = `开始下载 (${(totalBytes / 1024 / 1024).toFixed(1)} MB)`;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          if (totalBytes > 0) {
            downloadProgress.value = Math.round((downloaded / totalBytes) * 100);
            downloadStatus.value = `下载中 ${downloadProgress.value}% (${(downloaded / 1024 / 1024).toFixed(1)} / ${(totalBytes / 1024 / 1024).toFixed(1)} MB)`;
          } else {
            downloadStatus.value = `已下载 ${(downloaded / 1024 / 1024).toFixed(1)} MB`;
          }
          break;
        case "Finished":
          downloadProgress.value = 100;
          downloadStatus.value = "下载完成，正在启动安装程序...";
          break;
      }
    });

    downloadStatus.value = "安装中，应用即将重启...";
    updateStore.clear();
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (err: any) {
    downloadVisible.value = false;
    if (err !== "cancel") {
      ElMessage.error("更新失败: " + (err?.message || err));
    }
  } finally {
    checking.value = false;
  }
}

const settingsStore = useSettingsStore();
const galleryStore = useGalleryStore();
const updateStore = useUpdateStore();

const form = ref({
  providerType: "gemini-native",
  apiKey: "",
  baseUrl: "",
  model: "",
  timeoutMs: 45000,
  concurrency: 2,
  defaultLanguage: "zh",
  themeMode: "dark",
});

const saving = ref(false);

onMounted(() => {
  const s = settingsStore.settings;
  form.value = { ...s };
});

async function save() {
  if (form.value.timeoutMs < 5000) {
    ElMessage.error("超时不能小于 5000ms");
    return;
  }
  if (form.value.concurrency < 1 || form.value.concurrency > 20) {
    ElMessage.error("并发数应在 1-20 之间");
    return;
  }
  saving.value = true;
  try {
    await settingsStore.save(form.value);
    ElMessage.success("设置已保存");
  } catch (err: any) {
    ElMessage.error(err?.message || "保存失败");
  } finally {
    saving.value = false;
  }
}

async function clearAll() {
  try {
    await ElMessageBox.confirm(
      "确定要清除所有历史记录和缓存的缩略图数据吗？此操作不可恢复。",
      "清除全部数据",
      { type: "warning", confirmButtonText: "确认清除", cancelButtonText: "取消", confirmButtonClass: "el-button--danger" }
    );
    await galleryStore.clearAll();
    ElMessage.success("数据已清除");
  } catch {
    /* cancelled */
  }
}
</script>

<template>
  <div class="h-full overflow-y-auto px-8 py-6">
    <div data-tauri-drag-region class="h-1" />
    <div class="max-w-[820px] mx-auto space-y-4">
      <!-- API Settings -->
      <el-card>
        <template #header>
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-lg bg-teal-500/15 flex items-center justify-center">
              <el-icon :size="18" color="#2dd4bf"><Setting /></el-icon>
            </div>
            <span class="text-[18px] font-semibold text-white/90">API 设置</span>
          </div>
        </template>

        <el-form :model="form" label-position="top" size="default" class="settings-form">
          <el-form-item label="服务提供方">
            <el-select v-model="form.providerType" size="default" style="width: 100%">
              <el-option label="Gemini 原生接口" value="gemini-native" />
              <el-option label="OpenAI 兼容接口" value="openai-compatible" />
            </el-select>
          </el-form-item>

          <el-form-item label="API 密钥">
            <el-input
              v-model="form.apiKey"
              type="password"
              size="default"
              placeholder="请输入 API 密钥"
              show-password
            />
          </el-form-item>

          <el-form-item label="Base URL（仅 OpenAI 兼容接口）">
            <el-input
              v-model="form.baseUrl"
              size="default"
              placeholder="https://api.openai.com/v1"
            />
          </el-form-item>

          <el-form-item label="模型">
            <el-input
              v-model="form.model"
              size="default"
              placeholder="gemini-2.5-flash / gpt-4o"
            />
          </el-form-item>

          <div class="grid grid-cols-2 gap-5">
            <el-form-item label="超时（毫秒）">
              <el-input-number
                v-model="form.timeoutMs"
                :min="5000"
                :step="1000"
                size="default"
                controls-position="right"
                style="width: 100%"
              />
            </el-form-item>
            <el-form-item label="批量并发数">
              <el-input-number
                v-model="form.concurrency"
                :min="1"
                :max="20"
                size="default"
                controls-position="right"
                style="width: 100%"
              />
            </el-form-item>
          </div>

          <div class="grid grid-cols-2 gap-5">
            <el-form-item label="默认语言">
              <el-select v-model="form.defaultLanguage" size="default" style="width: 100%">
                <el-option label="中文" value="zh" />
                <el-option label="English" value="en" />
              </el-select>
            </el-form-item>
            <el-form-item label="主题">
              <el-select v-model="form.themeMode" size="default" style="width: 100%">
                <el-option label="深色" value="dark" />
                <el-option label="浅色" value="light" />
              </el-select>
            </el-form-item>
          </div>

          <div class="pt-3">
            <el-button type="primary" size="default" :loading="saving" @click="save">
              <el-icon class="mr-1.5"><Check /></el-icon>保存设置
            </el-button>
          </div>
        </el-form>
      </el-card>

      <!-- About / Update -->
      <el-card>
        <template #header>
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-lg bg-blue-500/15 flex items-center justify-center">
              <el-icon :size="18" color="#60a5fa"><Refresh /></el-icon>
            </div>
            <span class="text-[18px] font-semibold text-white/90">关于与更新</span>
          </div>
        </template>
        <div class="flex items-center justify-between flex-wrap gap-4">
          <div>
            <p class="text-[13px] text-white/75 font-medium mb-1">图片反推工具</p>
            <p class="text-[13px] text-white/45">当前版本：v{{ currentVersion }}</p>
          </div>
          <el-button type="primary" size="default" :loading="checking" @click="checkUpdate">
            <el-icon class="mr-1.5"><Refresh /></el-icon>检查更新
          </el-button>
        </div>
      </el-card>

      <!-- Download Progress Dialog -->
      <el-dialog v-model="downloadVisible" title="正在更新" :show-close="false" :close-on-click-modal="false" :close-on-press-escape="false" width="420px">
        <p class="text-[14px] text-white/70 mb-4">{{ downloadStatus }}</p>
        <el-progress :percentage="downloadProgress" :stroke-width="14" :text-inside="true" />
        <p class="text-[12px] text-white/40 mt-3">下载完成后会自动重启应用</p>
      </el-dialog>

      <!-- Data Management -->
      <el-card>
        <template #header>
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-lg bg-red-500/15 flex items-center justify-center">
              <el-icon :size="18" color="#f87171"><Delete /></el-icon>
            </div>
            <span class="text-[18px] font-semibold text-white/90">数据管理</span>
          </div>
        </template>

        <p class="text-[13px] text-white/55 leading-relaxed mb-4">
          清除所有历史记录和缓存的缩略图数据。此操作不可恢复。
        </p>
        <el-button type="danger" size="default" plain @click="clearAll">
          <el-icon class="mr-1.5"><Delete /></el-icon>清除全部数据
        </el-button>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
:deep(.el-card) {
  background-color: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 14px;
}
:deep(.el-card__header) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  padding: 14px 20px;
}
:deep(.el-card__body) {
  padding: 18px 20px;
}
:deep(.settings-form .el-form-item) {
  margin-bottom: 14px;
}
:deep(.settings-form .el-form-item__label) {
  font-size: 13px !important;
  font-weight: 500 !important;
  color: rgba(255, 255, 255, 0.75) !important;
  padding-bottom: 5px !important;
  line-height: 1.4 !important;
}
:deep(.settings-form .el-input__wrapper),
:deep(.settings-form .el-select__wrapper),
:deep(.settings-form .el-input-number) {
  font-size: 13px !important;
}
:deep(.settings-form .el-input__inner) {
  height: 36px !important;
  font-size: 13px !important;
}
</style>
