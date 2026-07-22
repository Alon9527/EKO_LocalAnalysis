import { defineStore } from "pinia";
import { ref } from "vue";
import { api, type AnalysisTask, type HistoryItem } from "@/lib/api";
import { useSettingsStore } from "./settings";

export interface ProgressState {
  percent: number;
  text: string;
}

const ANALYSIS_STAGES = [
  { at: 5, text: "准备识别..." },
  { at: 16, text: "正在读取图片..." },
  { at: 28, text: "上传图片到模型..." },
  { at: 44, text: "调用 AI 模型..." },
  { at: 62, text: "分析画面结构..." },
  { at: 78, text: "生成双模型提示词..." },
  { at: 90, text: "整理分析结果..." },
];

function stageText(percent: number) {
  let text = ANALYSIS_STAGES[0].text;
  for (const stage of ANALYSIS_STAGES) {
    if (percent >= stage.at) text = stage.text;
  }
  return text;
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
    progress.value = { percent: 5, text: stageText(5) };

    let progressTimer: ReturnType<typeof setInterval> | null = null;
    progressTimer = setInterval(() => {
      if (!analyzing.value) return;
      const current = progress.value.percent;
      if (current >= 92) {
        progress.value = { percent: 92, text: stageText(92) };
        return;
      }
      const increment = current < 30 ? 3 : current < 70 ? 2 : 1;
      const next = Math.min(92, current + increment);
      progress.value = { percent: next, text: stageText(next) };
    }, 350);

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

  function setPreparing(text = "正在读取图片...") {
    analyzing.value = false;
    progress.value = { percent: 2, text };
  }

  function reset() {
    progress.value = { percent: 0, text: "" };
    analyzing.value = false;
    result.value = null;
    error.value = null;
    previewSrc.value = "";
  }

  return { progress, analyzing, result, error, previewSrc, analyze, setPreparing, reset };
});
