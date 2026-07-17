import { describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ElementPlus from "element-plus";
import { api, type MaterialSourceVariant } from "@/lib/api";
import { useGalleryStore } from "@/stores/gallery";
import MaterialSourceGrid from "@/components/materials/MaterialSourceGrid.vue";

const source: MaterialSourceVariant = {
  id: "source-1",
  historyId: "history-1",
  thumbnailId: "history-1",
  fieldPath: "entities[0].appearance",
  promptZh: "焦糖色皮革与细腻纹理",
  promptEn: null,
  createdAt: 1,
};

describe("MaterialSourceGrid", () => {
  it("keeps the source prompt visible and opens its history when the thumbnail fails", async () => {
    vi.spyOn(api, "readThumbnailAsDataUrl").mockRejectedValue(new Error("missing"));
    const pinia = createPinia();
    setActivePinia(pinia);
    const gallery = useGalleryStore();
    const load = vi.spyOn(gallery, "load").mockResolvedValue();
    const openDetail = vi.spyOn(gallery, "openDetail");
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: "/materials", component: { template: "<div />" } },
        { path: "/gallery", component: { template: "<div />" } },
      ],
    });
    await router.push("/materials");
    await router.isReady();
    const wrapper = mount(MaterialSourceGrid, {
      props: { sources: [source] },
      global: { plugins: [pinia, router, ElementPlus] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain("图片不可用");
    expect(wrapper.text()).toContain(source.promptZh);
    await wrapper.get('[data-testid="open-history-source-1"]').trigger("click");
    await flushPromises();

    expect(load).toHaveBeenCalledTimes(1);
    expect(openDetail).toHaveBeenCalledWith("history-1");
    expect(router.currentRoute.value.path).toBe("/gallery");
    expect(router.currentRoute.value.query.history).toBe("history-1");
  });
});
