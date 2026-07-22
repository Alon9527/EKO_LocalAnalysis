import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { api, type HistoryItem } from "@/lib/api";
import { useAnalysisStore } from "@/stores/analysis";
import { useSettingsStore } from "@/stores/settings";

function historyItem(): HistoryItem {
  return {
    id: "history-1",
    fileName: "image.jpg",
    filePath: "",
    imageUrl: "",
    sourceType: "file",
    aspect_ratio: "16:9",
    contains_people: false,
    reconstructed_prompt: {},
    quality_notes: [],
    prompt_en: "prompt",
    prompt_zh: "提示词",
    qualityScore: 80,
    qualityLabel: "good",
    qualityBreakdown: {},
    qualityWarnings: [],
    model: "test-model",
    provider: "test-provider",
    elapsedMs: 10,
    favorite: false,
    createdAt: 1,
  };
}

describe("analysis progress", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("keeps progress moving while the model request is pending", async () => {
    vi.useFakeTimers();
    setActivePinia(createPinia());
    const settings = useSettingsStore();
    settings.settings.apiKey = "test-key";
    settings.settings.model = "test-model";
    const store = useAnalysisStore();

    let resolveAnalysis!: (item: HistoryItem) => void;
    vi.spyOn(api, "analyzeImage").mockImplementation(
      () => new Promise<HistoryItem>((resolve) => { resolveAnalysis = resolve; }),
    );

    const promise = store.analyze({ id: "task-1", sourceType: "file", filePath: "image.jpg" }, "preview-url");
    await nextTick();
    const initial = store.progress.percent;

    await vi.advanceTimersByTimeAsync(1000);

    expect(store.progress.percent).toBeGreaterThan(initial);
    expect(store.progress.percent).toBeLessThan(95);

    resolveAnalysis(historyItem());
    await promise;
    expect(store.progress.percent).toBe(100);
  });
});