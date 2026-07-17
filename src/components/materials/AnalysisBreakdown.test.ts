import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ElementPlus from "element-plus";
import { api, type MaterialAsset, type MaterialCategory } from "@/lib/api";
import AnalysisBreakdown from "@/components/materials/AnalysisBreakdown.vue";

function material(
  id: string,
  category: MaterialCategory,
  historyId = "history-1",
  promptEn: string | null = `${id} prompt`,
): MaterialAsset {
  return {
    id,
    category,
    generatedName: id,
    generatedExplanation: `${id} explanation`,
    generatedPromptZh: `${id} 中文`,
    generatedPromptEn: promptEn,
    generatedAliases: [],
    userOverride: {
      displayName: null,
      promptZh: null,
      promptEn: null,
      aliases: [],
      favorite: false,
      manuallyEdited: false,
      mergedInto: null,
      splitFrom: null,
      splitSourceIds: [],
    },
    sources: [
      {
        id: `${id}-source`,
        historyId,
        thumbnailId: historyId,
        fieldPath: `${category}.field`,
        promptZh: `${id} 来源中文`,
        promptEn,
        createdAt: 1,
      },
    ],
    createdAt: 1,
    updatedAt: 1,
  };
}

async function mountBreakdown(
  getHistoryMaterials: (historyId: string) => Promise<MaterialAsset[]>,
) {
  vi.spyOn(api, "getHistoryMaterials").mockImplementation(getHistoryMaterials);
  const pinia = createPinia();
  setActivePinia(pinia);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/gallery", component: { template: "<div />" } },
      { path: "/materials", component: { template: "<div />" } },
    ],
  });
  await router.push("/gallery");
  await router.isReady();
  const wrapper = mount(AnalysisBreakdown, {
    props: { historyId: "history-1" },
    global: { plugins: [pinia, router, ElementPlus] },
  });
  await flushPromises();
  return { wrapper, router };
}

describe("AnalysisBreakdown", () => {
  afterEach(() => vi.restoreAllMocks());

  it("groups rows in the fixed category order", async () => {
    const { wrapper } = await mountBreakdown(async () => [
      material("editorial", "style"),
      material("warm-light", "lighting"),
      material("chair", "element"),
      material("oak", "material"),
    ]);

    const categories = wrapper
      .findAll('[data-testid^="breakdown-section-"]')
      .map((section) => section.attributes("data-category"));
    expect(categories).toEqual(["element", "material", "lighting", "style"]);
    expect(wrapper.text()).toContain("chair 来源中文");
    expect(wrapper.text()).toContain("oak 来源中文");
  });

  it("shows language availability, copies the source fragment, and links to the asset", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const { wrapper } = await mountBreakdown(async () => [
      material("chair", "element"),
      material("soft-shadow", "lighting", "history-1", null),
    ]);

    expect(wrapper.get('[data-testid="language-chair"]').text()).toBe("中 / EN");
    expect(wrapper.get('[data-testid="language-soft-shadow"]').text()).toBe("中");
    expect(wrapper.get('[data-testid="open-material-chair"]').attributes("href"))
      .toBe("/materials?asset=chair");

    await wrapper.get('[data-testid="copy-material-chair"]').trigger("click");
    await flushPromises();
    expect(writeText).toHaveBeenCalledWith("chair 来源中文");
    expect(wrapper.get('[data-testid="copy-material-chair"]').attributes("title"))
      .toBe("已复制");
  });

  it("does not fabricate rows for full-prompt-only history", async () => {
    const { wrapper } = await mountBreakdown(async () => []);

    expect(wrapper.text()).toContain("暂无结构化分析内容");
    expect(wrapper.find('[data-testid^="breakdown-section-"]').exists()).toBe(false);
  });

  it("reloads assets when the history item changes", async () => {
    const getHistoryMaterials = vi.fn(async (historyId: string) => (
      historyId === "history-1"
        ? [material("chair", "element", historyId)]
        : [material("marble", "material", historyId)]
    ));
    const { wrapper } = await mountBreakdown(getHistoryMaterials);

    await wrapper.setProps({ historyId: "history-2" });
    await flushPromises();

    expect(getHistoryMaterials).toHaveBeenNthCalledWith(1, "history-1");
    expect(getHistoryMaterials).toHaveBeenNthCalledWith(2, "history-2");
    expect(wrapper.text()).toContain("marble 来源中文");
    expect(wrapper.text()).not.toContain("chair 来源中文");
  });

  it("ignores a stale failure after a newer history has loaded", async () => {
    let rejectFirst: (reason?: unknown) => void = () => {};
    const getHistoryMaterials = vi.fn((historyId: string) => {
      if (historyId === "history-1") {
        return new Promise<MaterialAsset[]>((_, reject) => {
          rejectFirst = reject;
        });
      }
      return Promise.resolve([
        material("marble", "material", historyId),
      ]);
    });
    const { wrapper } = await mountBreakdown(getHistoryMaterials);

    await wrapper.setProps({ historyId: "history-2" });
    await flushPromises();
    expect(wrapper.text()).toContain("marble 来源中文");

    rejectFirst(new Error("stale history failed"));
    await flushPromises();

    expect(wrapper.text()).toContain("marble 来源中文");
    expect(wrapper.text()).not.toContain("分析拆解加载失败");
  });
});
