import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ElementPlus from "element-plus";
import type { MaterialAsset } from "@/lib/api";
import MaterialAssetCard from "@/components/materials/MaterialAssetCard.vue";

function material(): MaterialAsset {
  return {
    id: "material-1",
    category: "material",
    generatedName: "Leather",
    generatedExplanation: "Fine leather grain",
    generatedPromptZh: "\u7ec6\u817b\u7684\u7126\u7cd6\u8272\u76ae\u9769\u7eb9\u7406",
    generatedPromptEn: "fine caramel leather grain",
    generatedAliases: [],
    userOverride: {
      displayName: "\u7126\u7cd6\u8272\u76ae\u9769",
      promptZh: null,
      promptEn: null,
      aliases: [],
      favorite: true,
      manuallyEdited: true,
      mergedInto: null,
      splitFrom: null,
      splitSourceIds: [],
    },
    sources: [
      {
        id: "source-1",
        historyId: "history-1",
        thumbnailId: "history-1",
        fieldPath: "entities[0].appearance",
        promptZh: "\u7126\u7cd6\u8272\u76ae\u9769",
        promptEn: "caramel leather",
        createdAt: 1,
      },
      {
        id: "source-2",
        historyId: "history-2",
        thumbnailId: "history-2",
        fieldPath: "technical_specs.texture_fidelity",
        promptZh: "\u7ec6\u817b\u76ae\u9769\u7eb9\u7406",
        promptEn: null,
        createdAt: 2,
      },
    ],
    createdAt: 1,
    updatedAt: 2,
  };
}

describe("MaterialAssetCard", () => {
  it("shows resolved metadata and emits open and favorite actions", async () => {
    const wrapper = mount(MaterialAssetCard, {
      props: { asset: material() },
      global: { plugins: [ElementPlus] },
    });

    expect(wrapper.text()).toContain("\u6750\u8d28");
    expect(wrapper.text()).toContain("\u7126\u7cd6\u8272\u76ae\u9769");
    expect(wrapper.text()).toContain("\u7ec6\u817b\u7684\u7126\u7cd6\u8272\u76ae\u9769\u7eb9\u7406");
    expect(wrapper.text()).toContain("2 \u4e2a\u6765\u6e90");
    expect(wrapper.get('[data-testid="favorite"]').classes()).toContain("is-favorite");

    await wrapper.get('[data-testid="material-card"]').trigger("click");
    await wrapper.get('[data-testid="favorite"]').trigger("click");
    expect(wrapper.emitted("open")).toHaveLength(1);
    expect(wrapper.emitted("toggle-favorite")).toHaveLength(1);
    await wrapper.get('[data-testid="favorite"]').trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("open")).toHaveLength(1);
  });
});
