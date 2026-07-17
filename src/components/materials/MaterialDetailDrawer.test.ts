import { describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import ElementPlus from "element-plus";
import type { MaterialAsset, MaterialCategory } from "@/lib/api";
import MaterialDetailDrawer from "@/components/materials/MaterialDetailDrawer.vue";

function asset(
  id: string,
  category: MaterialCategory = "element",
  sourceCount = 2,
): MaterialAsset {
  return {
    id,
    category,
    generatedName: id,
    generatedExplanation: `${id} explanation`,
    generatedPromptZh: `${id} zh`,
    generatedPromptEn: `${id} en`,
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
    sources: Array.from({ length: sourceCount }, (_, index) => ({
      id: `${id}-source-${index + 1}`,
      historyId: `history-${index + 1}`,
      thumbnailId: `history-${index + 1}`,
      fieldPath: `entities[${index}].label`,
      promptZh: `${id} source ${index + 1}`,
      promptEn: null,
      createdAt: index + 1,
    })),
    createdAt: 1,
    updatedAt: 1,
  };
}

function mountDrawer(current = asset("chair")) {
  return mount(MaterialDetailDrawer, {
    attachTo: document.body,
    props: {
      modelValue: true,
      asset: current,
      candidates: [current, asset("table"), asset("warm-light", "lighting")],
    },
    global: {
      plugins: [ElementPlus],
      stubs: { teleport: true, MaterialSourceGrid: true },
    },
  });
}

describe("MaterialDetailDrawer", () => {
  it("saves edited resolved fields", async () => {
    const wrapper = mountDrawer();
    await flushPromises();
    await wrapper.get('[data-testid="display-name"] input').setValue("Reading chair");
    await wrapper.get('[data-testid="prompt-zh"] textarea').setValue("Edited chair prompt");
    await wrapper.get('[data-testid="save-material"]').trigger("click");

    expect(wrapper.emitted("save")?.[0]).toEqual([
      "chair",
      expect.objectContaining({
        displayName: "Reading chair",
        promptZh: "Edited chair prompt",
      }),
    ]);
  });

  it("omits a blank English override when the asset has no English fragment", async () => {
    const current = asset("chair");
    current.generatedPromptEn = null;
    const wrapper = mountDrawer(current);
    await flushPromises();
    await wrapper.get('[data-testid="save-material"]').trigger("click");

    const patch = wrapper.emitted("save")?.[0]?.[1] as Record<string, unknown>;
    expect(patch).not.toHaveProperty("promptEn");
    expect(patch.promptZh).toBe("chair zh");
  });

  it("offers only same-category merge candidates", async () => {
    const wrapper = mountDrawer();
    await wrapper.get('[data-testid="open-merge"]').trigger("click");
    await flushPromises();

    expect(wrapper.find('[data-testid="merge-option-table"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="merge-option-warm-light"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="confirm-merge"]').attributes("disabled")).toBeDefined();
  });

  it("keeps keyboard focus inside operation dialogs and restores the trigger", async () => {
    const wrapper = mountDrawer();
    const trigger = wrapper.get('[data-testid="open-merge"]');
    await trigger.trigger("click");
    await flushPromises();
    const panel = wrapper.get('[aria-labelledby="merge-material-title"]');

    (panel.element as HTMLElement).focus();
    await panel.trigger("keydown", { key: "Tab", shiftKey: true });
    expect(panel.element.contains(document.activeElement)).toBe(true);

    await panel.trigger("keydown", { key: "Escape" });
    await flushPromises();
    expect(wrapper.find('[aria-labelledby="merge-material-title"]').exists()).toBe(false);
    expect(document.activeElement).toBe(
      wrapper.get('[data-testid="open-merge"]').element,
    );
  });

  it("enables split only for a non-empty proper source subset", async () => {
    const wrapper = mountDrawer();
    await wrapper.get('[data-testid="open-split"]').trigger("click");
    await flushPromises();
    const confirm = () => wrapper.get('[data-testid="confirm-split"]');
    const checkboxes = wrapper.findAll('[data-testid^="split-source-"] input');

    expect(confirm().attributes("disabled")).toBeDefined();
    await checkboxes[0].setValue(true);
    await flushPromises();
    expect(confirm().attributes("disabled")).toBeUndefined();
    await checkboxes[1].setValue(true);
    await flushPromises();
    expect(confirm().attributes("disabled")).toBeDefined();
  });
});
