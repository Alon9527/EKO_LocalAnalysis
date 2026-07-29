import { defineStore } from "pinia";
import { ref } from "vue";
import { api, type Settings } from "@/lib/api";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings>({
    providerType: "gemini-native",
    apiKey: "",
    baseUrl: "",
    model: "gemini-2.5-flash",
    timeoutMs: 45000,
    defaultLanguage: "zh",
    themeMode: "dark",
    concurrency: 2,
  });
  const loaded = ref(false);

  async function load() {
    try {
      settings.value = await api.getSettings();
    } catch {
      /* use defaults */
    }
    loaded.value = true;
    applyTheme(settings.value.themeMode);
  }

  async function save(next: Partial<Settings>) {
    settings.value = await api.saveSettings(next);
    applyTheme(settings.value.themeMode);
  }

  function applyTheme(mode: string) {
    const root = document.documentElement;
    const nextMode = mode === "light" || mode === "macos" ? mode : "dark";
    root.dataset.theme = nextMode;
    root.classList.toggle("light", nextMode === "light" || nextMode === "macos");
    root.classList.toggle("dark", nextMode === "dark");
  }

  return { settings, loaded, load, save };
});
