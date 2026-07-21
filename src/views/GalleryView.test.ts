import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, shallowMount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ElementPlus from "element-plus";
import { api, type HistoryItem } from "@/lib/api";
import { useGalleryStore } from "@/stores/gallery";
import GalleryView from "@/views/GalleryView.vue";

const TabsStub = {
  name: "ElTabs",
  props: ["modelValue"],
  emits: ["update:modelValue"],
  template: '<div><slot /></div>',
};

const TabPaneStub = {
  name: "ElTabPane",
  props: ["label", "name"],
  template: '<section><span>{{ label }}</span><slot /></section>',
};
const DrawerStub = {
  name: "ElDrawer",
  props: ["modelValue"],
  emits: ["update:modelValue"],
  template: '<section v-if="modelValue"><slot /></section>',
};

function historyItem(): HistoryItem {
  return {
    id: "history-1",
    fileName: "chair.jpg",
    filePath: "",
    imageUrl: "",
    sourceType: "clipboard",
    aspect_ratio: "4:3",
    contains_people: false,
    reconstructed_prompt: {},
    quality_notes: [],
    prompt_en: "chair prompt",
    prompt_zh: "椅子提示词",
    qualityScore: 80,
    qualityLabel: "较好",
    qualityBreakdown: {},
    qualityWarnings: [],
    model: "test-model",
    provider: "test-provider",
    elapsedMs: 10,
    favorite: false,
    createdAt: 1,
  };
}

describe("GalleryView direct history navigation", () => {
  afterEach(() => vi.restoreAllMocks());

  async function mountAtHistory(getHistory: typeof api.getHistory) {
    vi.spyOn(api, "getHistory").mockImplementation(getHistory);
    vi.spyOn(api, "readThumbnailAsDataUrl").mockResolvedValue("data:image/png;base64,AA==");
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: "/gallery", component: GalleryView }],
    });
    await router.push("/gallery?history=history-1");
    await router.isReady();
    const wrapper = shallowMount(GalleryView, {
      global: {
        plugins: [pinia, router, ElementPlus],
        stubs: { teleport: true, RadarChart: true, ElDrawer: DrawerStub, ElTabs: TabsStub, ElTabPane: TabPaneStub },
      },
    });
    await flushPromises();
    return { wrapper, router, store: useGalleryStore() };
  }

  it("opens the requested history detail after loading, including direct URLs", async () => {
    const item = historyItem();
    vi.spyOn(api, "getHistory").mockResolvedValue({ items: [item], total: 1 });
    vi.spyOn(api, "readThumbnailAsDataUrl").mockResolvedValue("data:image/png;base64,AA==");
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: "/gallery", component: GalleryView }],
    });
    await router.push("/gallery?history=history-1");
    await router.isReady();
    const wrapper = shallowMount(GalleryView, {
      global: { plugins: [pinia, router, ElementPlus], stubs: { teleport: true, RadarChart: true, ElDrawer: DrawerStub, ElTabs: TabsStub, ElTabPane: TabPaneStub } },
    });
    await flushPromises();

    expect(useGalleryStore().detailItem?.id).toBe("history-1");
    wrapper.unmount();
  });

  it("loads a requested history by id when active filters exclude it", async () => {
    const item = historyItem();
    const getHistory = vi.fn(async (query) => {
      if ((query as { id?: string }).id === item.id) {
        return { items: [item], total: 1 };
      }
      return { items: [], total: 0 };
    });
    const { wrapper, store } = await mountAtHistory(getHistory);

    expect(store.detailItem?.id).toBe("history-1");
    expect(getHistory).toHaveBeenCalledWith({
      id: "history-1",
      page: 1,
      pageSize: 1,
    });
    wrapper.unmount();
  });

  it("removes only the history query when the detail closes", async () => {
    const item = historyItem();
    const { wrapper, router, store } = await mountAtHistory(
      vi.fn().mockResolvedValue({ items: [item], total: 1 }),
    );
    await router.replace({
      path: "/gallery",
      query: { history: "history-1", keyword: "chair" },
    });
    await flushPromises();

    wrapper
      .getComponent({ name: "ElDrawer" })
      .vm.$emit("update:modelValue", false);
    await flushPromises();

    expect(store.detailItem).toBeNull();
    expect(router.currentRoute.value.query).toEqual({ keyword: "chair" });
    wrapper.unmount();
  });
  it("refreshes history records from the toolbar", async () => {
    const first = historyItem();
    const second = { ...historyItem(), id: "history-2", fileName: "new-image.jpg" };
    const getHistory = vi
      .fn()
      .mockResolvedValueOnce({ items: [first], total: 1 })
      .mockResolvedValueOnce({ items: [first, second], total: 2 });
    vi.spyOn(api, "getHistory").mockImplementation(getHistory);
    vi.spyOn(api, "readThumbnailAsDataUrl").mockResolvedValue("data:image/png;base64,AA==");
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: "/gallery", component: GalleryView }],
    });
    await router.push("/gallery");
    await router.isReady();
    const wrapper = shallowMount(GalleryView, {
      global: { plugins: [pinia, router, ElementPlus], stubs: { teleport: true, RadarChart: true, ElDrawer: DrawerStub, ElTabs: TabsStub, ElTabPane: TabPaneStub } },
    });
    await flushPromises();

    expect(useGalleryStore().items).toHaveLength(1);
    await wrapper.get('[data-testid="refresh-history"]').trigger("click");
    await flushPromises();

    expect(getHistory).toHaveBeenCalledTimes(2);
    expect(useGalleryStore().items.map((item) => item.id)).toEqual(["history-1", "history-2"]);
    wrapper.unmount();
  });
  it("switches history detail prompts between GPT Image and Nano Banana", async () => {
    const item = {
      ...historyItem(),
      prompt_en: "legacy en",
      prompt_zh: "legacy zh",
      promptGptImageEn: "gpt image prompt en",
      promptGptImageZh: "gpt image prompt zh",
      promptNanoBananaEn: "nano banana prompt en",
      promptNanoBananaZh: "nano banana prompt zh",
    };
    const { wrapper } = await mountAtHistory(vi.fn().mockResolvedValue({ items: [item], total: 1 }));

    expect(wrapper.text()).toContain("GPT Image");
    expect(wrapper.text()).toContain("Nano Banana");
    expect((wrapper.get('[data-testid="detail-prompt-textarea"]').element as HTMLTextAreaElement).value).toBe("gpt image prompt zh");

    await wrapper.get('[data-testid="prompt-target-nano"]').trigger("click");
    await flushPromises();

    expect((wrapper.get('[data-testid="detail-prompt-textarea"]').element as HTMLTextAreaElement).value).toBe("nano banana prompt zh");
    wrapper.unmount();
  });
});
