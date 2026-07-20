import { defineStore } from "pinia";
import { markRaw, ref, shallowRef } from "vue";

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

export const useUpdateStore = defineStore("update", () => {
  const checking = ref(false);
  const updateAvailable = ref(false);
  const version = ref("");
  const notes = ref("");
  const error = ref("");
  const updateRef = shallowRef<any>(null);

  async function check(silent = true) {
    if (!isTauri) return null;
    checking.value = true;
    error.value = "";
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      updateRef.value = update ? markRaw(update) : null;
      updateAvailable.value = !!update;
      version.value = update?.version || "";
      notes.value = update?.body || "";
      return update;
    } catch (err: any) {
      error.value = err?.message || String(err);
      if (!silent) throw err;
      return null;
    } finally {
      checking.value = false;
    }
  }

  function clear() {
    updateAvailable.value = false;
    version.value = "";
    notes.value = "";
    updateRef.value = null;
  }

  return { checking, updateAvailable, version, notes, error, updateRef, check, clear };
});
