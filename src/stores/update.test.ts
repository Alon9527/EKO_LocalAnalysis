import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const updaterCheck = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: updaterCheck,
}));

class PrivateUpdate {
  #ready = true;
  version = "9.9.9";
  body = "test update";

  downloadAndInstall() {
    return this.#ready;
  }
}

describe("update store", () => {
  beforeEach(() => {
    vi.resetModules();
    setActivePinia(createPinia());
    (window as any).__TAURI_INTERNALS__ = {};
    updaterCheck.mockReset();
  });

  it("keeps the Tauri update instance unproxied so private fields still work", async () => {
    const update = new PrivateUpdate();
    updaterCheck.mockResolvedValue(update);
    const { useUpdateStore } = await import("@/stores/update");
    const store = useUpdateStore();

    await store.check(false);

    expect(store.updateRef).toBe(update);
    expect(store.updateRef.downloadAndInstall()).toBe(true);
  });
});
