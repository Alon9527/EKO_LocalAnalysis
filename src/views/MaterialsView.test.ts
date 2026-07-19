import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ElementPlus, { ElMessage } from "element-plus";
import type { MaterialAsset } from "@/lib/api";
import { useMaterialsStore } from "@/stores/materials";
import MaterialsView from "@/views/MaterialsView.vue";

function asset(
  id = "asset-1",
  options: {
    category?: MaterialAsset["category"];
    historyId?: string;
    promptZh?: string;
    promptEn?: string;
    favorite?: boolean;
    createdAt?: number;
  } = {},
): MaterialAsset {
  const historyId = options.historyId || "history-1";
  const promptZh = options.promptZh || "\u73b0\u4ee3\u4f11\u95f2\u6905";
  return {
    id,
    category: options.category || "element",
    generatedName: promptZh,
    generatedExplanation: "Chair explanation",
    generatedPromptZh: promptZh,
    generatedPromptEn: options.promptEn || "modern lounge chair",
    generatedAliases: [],
    userOverride: {
      displayName: null,
      promptZh: null,
      promptEn: null,
      aliases: [],
      favorite: options.favorite || false,
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
        fieldPath: "entities[0].label",
        promptZh,
        promptEn: options.promptEn || "modern lounge chair",
        createdAt: options.createdAt || (historyId === "history-2" ? 2 : 1),
      },
    ],
    createdAt: 1,
    updatedAt: 1,
  };
}

async function mountView(items = [asset()]) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = useMaterialsStore();
  store.items = items;
  const load = vi.spyOn(store, "load").mockResolvedValue(true);
  const rebuild = vi.spyOn(store, "rebuild").mockResolvedValue(true);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/materials", component: { template: "<div />" } },
      { path: "/gallery", component: { template: "<div />" } },
      { path: "/single", component: { template: "<div />" } },
    ],
  });
  await router.push("/materials");
  await router.isReady();
  const wrapper = mount(MaterialsView, {
    global: {
      plugins: [pinia, router, ElementPlus],
      stubs: { teleport: true },
    },
  });
  await flushPromises();
  load.mockClear();
  return { wrapper, store, load, rebuild, router };
}

describe("MaterialsView", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("debounces search and forwards category and favorite filters", async () => {
    const { wrapper, store, load } = await mountView();
    await wrapper.get('[data-testid="materials-search"]').setValue("wood");
    vi.advanceTimersByTime(320);
    await flushPromises();
    expect(store.keyword).toBe("wood");
    expect(load).toHaveBeenCalledTimes(1);

    load.mockClear();
    await wrapper.get('[data-testid="category-material"]').trigger("click");
    await wrapper.get('[data-testid="favorite-only"]').setValue(true);
    await flushPromises();
    expect(store.category).toBe("material");
    expect(store.favoriteOnly).toBe(true);
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("opens a source-image pack through route state", async () => {
    const { wrapper, router } = await mountView();
    await wrapper.get('[data-testid="material-pack"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.query.pack).toBe("history-1");
    expect(wrapper.text()).toContain("modern lounge chair");
  });

  it("groups multiple fragments from one image into one visible pack", async () => {
    const { wrapper } = await mountView([
      asset("chair", { historyId: "history-1", promptZh: "\u4f11\u95f2\u6905", createdAt: 3 }),
      asset("wood", { category: "material", historyId: "history-1", promptZh: "\u80e1\u6843\u6728\u6750\u8d28", createdAt: 3 }),
      asset("light", { category: "lighting", historyId: "history-2", promptZh: "\u67d4\u548c\u81ea\u7136\u5149" }),
    ]);

    expect(wrapper.findAll('[data-testid="material-pack"]')).toHaveLength(2);
    await wrapper.get('[data-testid="material-pack"]').trigger("click");
    expect(wrapper.text()).toContain("\u4f11\u95f2\u6905");
    expect(wrapper.text()).toContain("\u80e1\u6843\u6728\u6750\u8d28");
  });

  it("toggles favorites from a fragment row without changing selected pack", async () => {
    const { wrapper, store, router } = await mountView();
    const favorite = vi.spyOn(store, "setAssetFavorite").mockImplementation(async () => {
      const updated = asset();
      updated.userOverride.favorite = true;
      return updated;
    });

    await wrapper.get('[data-testid="material-pack"]').trigger("click");
    await wrapper.get('[data-testid="favorite-asset-1"]').trigger("click");
    await flushPromises();
    expect(favorite).toHaveBeenCalledWith("asset-1", true);
    expect(router.currentRoute.value.query.pack).toBe("history-1");
  });

  it("explains the empty state and keeps packs visible with an error", async () => {
    const empty = await mountView([]);
    expect(empty.wrapper.text()).toContain("\u65b0\u7684\u7ed3\u6784\u5316\u5206\u6790\u7ed3\u679c\u4f1a\u81ea\u52a8\u6574\u7406\u6210\u56fe\u7247\u7d20\u6750\u5305");
    empty.wrapper.unmount();

    const populated = await mountView([asset()]);
    populated.store.error = "Index rebuild failed";
    await populated.wrapper.vm.$nextTick();
    expect(populated.wrapper.text()).toContain("Index rebuild failed");
    expect(populated.wrapper.find('[data-testid="material-pack"]').exists()).toBe(true);
  });

  it("offers rebuild from the compact overflow menu", async () => {
    const { wrapper, rebuild } = await mountView();
    await wrapper.get('[data-testid="open-library-menu"]').trigger("click");
    await wrapper.get('[data-testid="rebuild-index"]').trigger("click");
    expect(rebuild).toHaveBeenCalledTimes(1);
  });

  it("reports rebuild results", async () => {
    const { wrapper, store, rebuild } = await mountView();
    const success = vi.spyOn(ElMessage, "success");
    const warning = vi.spyOn(ElMessage, "warning");

    store.error = "";
    rebuild.mockResolvedValueOnce(true);
    await wrapper.get('[data-testid="open-library-menu"]').trigger("click");
    await wrapper.get('[data-testid="rebuild-index"]').trigger("click");
    await flushPromises();
    expect(success).toHaveBeenCalledWith("\u7d20\u6750\u7d22\u5f15\u5df2\u91cd\u5efa");

    store.error = "Refresh failed";
    rebuild.mockResolvedValueOnce(true);
    await wrapper.get('[data-testid="open-library-menu"]').trigger("click");
    await wrapper.get('[data-testid="rebuild-index"]').trigger("click");
    await flushPromises();
    expect(warning).toHaveBeenCalledWith(
      "\u7d20\u6750\u7d22\u5f15\u5df2\u91cd\u5efa\uff0c\u4f46\u5217\u8868\u5237\u65b0\u5931\u8d25\uff1aRefresh failed",
    );
  });
});