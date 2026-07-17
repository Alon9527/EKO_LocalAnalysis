import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ElementPlus from "element-plus";
import type { MaterialAsset } from "@/lib/api";
import { useMaterialsStore } from "@/stores/materials";
import MaterialsView from "@/views/MaterialsView.vue";

function asset(id = "asset-1"): MaterialAsset {
  return {
    id,
    category: "element",
    generatedName: "Chair",
    generatedExplanation: "Chair explanation",
    generatedPromptZh: "\u73b0\u4ee3\u4f11\u95f2\u6905",
    generatedPromptEn: "modern lounge chair",
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
    sources: [],
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
  const rebuild = vi.spyOn(store, "rebuild").mockResolvedValue();
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
      stubs: { teleport: true, MaterialDetailDrawer: true },
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

  it("debounces search and forwards category, favorite, and source filters", async () => {
    const { wrapper, store, load } = await mountView();
    await wrapper.get('[data-testid="materials-search"]').setValue("wood");
    expect(load).not.toHaveBeenCalled();
    vi.advanceTimersByTime(320);
    await flushPromises();
    expect(store.keyword).toBe("wood");
    expect(load).toHaveBeenCalledTimes(1);

    load.mockClear();
    await wrapper.get('[data-testid="category-material"]').trigger("click");
    await wrapper.get('[data-testid="favorite-only"]').setValue(true);
    await wrapper.get('[data-testid="min-sources"]').setValue("2");
    await flushPromises();
    expect(store.category).toBe("material");
    expect(store.favoriteOnly).toBe(true);
    expect(store.minSources).toBe(2);
    expect(load).toHaveBeenCalledTimes(3);
  });

  it("opens a card through route state", async () => {
    const { wrapper, store, router } = await mountView();
    const loadCandidates = vi.spyOn(store, "loadMergeCandidates").mockResolvedValue();
    await wrapper.get('[data-testid="material-card"]').trigger("click");
    await flushPromises();

    expect(store.selectedAsset?.id).toBe("asset-1");
    expect(router.currentRoute.value.query.asset).toBe("asset-1");
    expect(loadCandidates).toHaveBeenCalledWith("element");
  });

  it("does not open the detail drawer when a card favorite is toggled", async () => {
    const { wrapper, store, router } = await mountView();
    const favorite = vi.spyOn(store, "setAssetFavorite").mockImplementation(async () => {
      const updated = asset();
      updated.userOverride.favorite = true;
      return updated;
    });

    await wrapper.get('[data-testid="favorite"]').trigger("click");
    await flushPromises();
    expect(favorite).toHaveBeenCalledWith("asset-1", true);
    expect(store.selectedAsset).toBeNull();
    expect(router.currentRoute.value.query.asset).toBeUndefined();
  });

  it("explains the empty state and keeps cards visible with an error", async () => {
    const empty = await mountView([]);
    expect(empty.wrapper.text()).toContain("\u65b0\u7684\u7ed3\u6784\u5316\u5206\u6790\u7ed3\u679c\u4f1a\u81ea\u52a8\u51fa\u73b0\u5728\u8fd9\u91cc");
    empty.wrapper.unmount();

    const populated = await mountView([asset()]);
    populated.store.error = "Index rebuild failed";
    await populated.wrapper.vm.$nextTick();
    expect(populated.wrapper.text()).toContain("Index rebuild failed");
    expect(populated.wrapper.find('[data-testid="material-card"]').exists()).toBe(true);
  });

  it("offers rebuild from the compact overflow menu", async () => {
    const { wrapper, rebuild } = await mountView();
    await wrapper.get('[data-testid="open-library-menu"]').trigger("click");
    await wrapper.get('[data-testid="rebuild-index"]').trigger("click");
    expect(rebuild).toHaveBeenCalledTimes(1);
  });
});
