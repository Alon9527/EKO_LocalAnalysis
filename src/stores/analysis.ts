import { defineStore } from "pinia";
import { ref } from "vue";
import { api, type AnalysisTask, type HistoryItem } from "@/lib/api";
import { useSettingsStore } from "./settings";

export interface ProgressState {
  percent: number;
  text: string;
}

export const useAnalysisStore = defineStore("analysis", () => {
  const progress = ref<ProgressState>({ percent: 0, text: "" });
  const analyzing = ref(false);
  const result = ref<HistoryItem | null>(null);
  const error = ref<string | null>(null);
  const previewSrc = ref("");

  async function analyze(task: AnalysisTask, previewUrl?: string) {
    const settingsStore = useSettingsStore();
    error.value = null;
    result.value = null;
    if (previewUrl) previewSrc.value = previewUrl;

    // Pre-flight checks
    if (!settingsStore.settings.apiKey || settingsStore.settings.apiKey.trim() === "") {
      error.value = "API 密钥未配置，请到「设置中心」填入你的 API Key（Gemini 或 OpenAI）";
      return;
    }
    if (!settingsStore.settings.model || settingsStore.settings.model.trim() === "") {
      error.value = "模型名称未配置，请到「设置中心」填入模型名（如 gemini-2.5-flash 或 gpt-4o）";
      return;
    }

    analyzing.value = true;
    progress.value = { percent: 5, text: "准备识别..." };

    // Simulated progress while waiting for API response
    let progressTimer: ReturnType<typeof setInterval> | null = null;
    const stages = [
      { p: 20, t: "上传图片..." },
      { p: 40, t: "调用 AI 模型..." },
      { p: 65, t: "分析图像内容..." },
      { p: 85, t: "生成提示词..." },
    ];
    let stageIdx = 0;
    progressTimer = setInterval(() => {
      if (stageIdx < stages.length && analyzing.value) {
        progress.value = { percent: stages[stageIdx].p, text: stages[stageIdx].t };
        stageIdx++;
      }
    }, 2500);

    try {
      const res = await api.analyzeImage(task, settingsStore.settings);
      result.value = res;
      progress.value = { percent: 100, text: "识别完成" };
    } catch (err: any) {
      error.value = err?.message || String(err) || "分析失败";
      progress.value = { percent: 0, text: error.value! };
    } finally {
      if (progressTimer) clearInterval(progressTimer);
      analyzing.value = false;
    }
  }

  function reset() {
    progress.value = { percent: 0, text: "" };
    analyzing.value = false;
    result.value = null;
    error.value = null;
    previewSrc.value = "";
  }

  return { progress, analyzing, result, error, previewSrc, analyze, reset };
});
