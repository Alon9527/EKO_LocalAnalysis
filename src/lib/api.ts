export interface Settings {
  providerType: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  timeoutMs: number;
  defaultLanguage: string;
  themeMode: string;
  concurrency: number;
}

export interface AnalysisTask {
  id: string;
  sourceType: "file" | "url" | "clipboard";
  filePath?: string;
  fileName?: string;
  imageUrl?: string;
  base64Data?: string;
  mimeType?: string;
}

export interface HistoryItem {
  id: string;
  fileName: string;
  filePath: string;
  imageUrl: string;
  sourceType: string;
  aspect_ratio: string;
  contains_people: boolean;
  reconstructed_prompt: any;
  reconstructed_prompt_zh?: any;
  quality_notes: string[];
  prompt_en: string;
  prompt_zh: string;
  qualityScore: number;
  qualityLabel: string;
  qualityBreakdown: Record<string, number>;
  qualityWarnings: string[];
  model: string;
  provider: string;
  elapsedMs: number;
  favorite: boolean;
  createdAt: number;
  thumbUrl?: string;
}

export interface HistoryQuery {
  keyword?: string;
  minScore?: number;
  maxScore?: number;
  favorite?: boolean;
  pageSize?: number;
  page?: number;
}

export interface ImportSummary {
  imported: number;
  renamed: number;
  skipped: number;
  total: number;
}

export type MaterialCategory =
  | "element"
  | "material"
  | "color"
  | "lighting"
  | "camera"
  | "composition"
  | "style"
  | "environment";

export interface MaterialSourceVariant {
  id: string;
  historyId: string;
  thumbnailId: string;
  fieldPath: string;
  promptZh: string;
  promptEn?: string;
  createdAt: number;
}

export interface MaterialOverride {
  displayName?: string;
  promptZh?: string;
  promptEn?: string;
  aliases: string[];
  favorite: boolean;
  manuallyEdited: boolean;
  mergedInto?: string;
  splitFrom?: string;
  splitSourceIds?: string[];
}

export interface MaterialAsset {
  id: string;
  category: MaterialCategory;
  generatedName: string;
  generatedExplanation: string;
  generatedPromptZh: string;
  generatedPromptEn?: string;
  generatedAliases: string[];
  userOverride: MaterialOverride;
  sources: MaterialSourceVariant[];
  createdAt: number;
  updatedAt: number;
}

export interface MaterialIndexWarning {
  historyId: string;
  message: string;
}

export interface MaterialQuery {
  keyword?: string;
  category?: MaterialCategory;
  favorite?: boolean;
  minSources?: number;
}

export interface MaterialPatch {
  displayName?: string;
  promptZh?: string;
  promptEn?: string;
  aliases?: string[];
  favorite?: boolean;
}

export interface MaterialListResponse {
  items: MaterialAsset[];
  total: number;
  stale: boolean;
  warnings: MaterialIndexWarning[];
}

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;
const EMPTY_MATERIAL_RESPONSE: MaterialListResponse = {
  items: [],
  total: 0,
  stale: false,
  warnings: [],
};

const DEFAULT_SETTINGS: Settings = {
  providerType: "gemini-native",
  apiKey: "",
  baseUrl: "",
  model: "gemini-2.5-flash",
  timeoutMs: 45000,
  defaultLanguage: "zh",
  themeMode: "dark",
  concurrency: 2,
};

const SETTINGS_KEY = "autoprompt:settings";
const HISTORY_KEY = "autoprompt:history";

// Browser fallback: read/write to localStorage
const browserMock = {
  getSettings(): Settings {
    try {
      const raw = localStorage.getItem(SETTINGS_KEY);
      if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    } catch {}
    return { ...DEFAULT_SETTINGS };
  },
  saveSettings(data: Partial<Settings>): Settings {
    const current = browserMock.getSettings();
    const merged = { ...current, ...data };
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(merged));
    return merged;
  },
  getHistory(): { items: HistoryItem[]; total: number } {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      const items = raw ? JSON.parse(raw) : [];
      return { items, total: items.length };
    } catch {
      return { items: [], total: 0 };
    }
  },
  clearHistory() { localStorage.removeItem(HISTORY_KEY); },
};

function browserUnsupported(action: string): never {
  throw new Error(`${action} 需要在桌面应用（Tauri 窗口）中运行，浏览器预览不支持`);
}

async function invokeTauri<T>(cmd: string, args?: any): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const api = {
  async getSettings(): Promise<Settings> {
    if (!isTauri) return browserMock.getSettings();
    return invokeTauri<Settings>("get_settings");
  },

  async saveSettings(data: Partial<Settings>): Promise<Settings> {
    if (!isTauri) return browserMock.saveSettings(data);
    return invokeTauri<Settings>("save_settings", { data });
  },

  async analyzeImage(task: AnalysisTask, settings: Settings): Promise<HistoryItem> {
    if (!isTauri) browserUnsupported("图像分析");
    return invokeTauri<HistoryItem>("analyze_image", { task, settings });
  },

  async getHistory(query: HistoryQuery): Promise<{ items: HistoryItem[]; total: number }> {
    if (!isTauri) return browserMock.getHistory();
    return invokeTauri("get_history", { query });
  },

  async deleteHistory(ids: string[]): Promise<void> {
    if (!isTauri) browserUnsupported("删除历史");
    return invokeTauri("delete_history", { ids });
  },

  async toggleFavorite(id: string): Promise<boolean> {
    if (!isTauri) browserUnsupported("收藏切换");
    return invokeTauri("toggle_favorite", { id });
  },

  async clearHistory(): Promise<void> {
    if (!isTauri) { browserMock.clearHistory(); return; }
    return invokeTauri("clear_history");
  },

  async exportItems(ids: string[], format: string, outputPath: string) {
    if (!isTauri) browserUnsupported("导出文件");
    return invokeTauri<{ path: string; count: number }>("export_items", { ids, format, outputPath });
  },

  async importItems(inputPath: string): Promise<ImportSummary> {
    if (!isTauri) browserUnsupported("导入结果");
    return invokeTauri<ImportSummary>("import_items", { inputPath });
  },

  async listMaterials(query: MaterialQuery): Promise<MaterialListResponse> {
    if (!isTauri) return { ...EMPTY_MATERIAL_RESPONSE, items: [], warnings: [] };
    return invokeTauri<MaterialListResponse>("list_materials", { query });
  },

  async getHistoryMaterials(historyId: string): Promise<MaterialAsset[]> {
    if (!isTauri) return [];
    return invokeTauri<MaterialAsset[]>("get_history_materials", { historyId });
  },

  async rebuildMaterialIndex(): Promise<MaterialListResponse> {
    if (!isTauri) return { ...EMPTY_MATERIAL_RESPONSE, items: [], warnings: [] };
    return invokeTauri<MaterialListResponse>("rebuild_material_index");
  },

  async updateMaterial(id: string, patch: MaterialPatch): Promise<MaterialAsset> {
    if (!isTauri) browserUnsupported("Update material");
    return invokeTauri<MaterialAsset>("update_material", { id, patch });
  },

  async mergeMaterials(ids: string[], displayName?: string): Promise<MaterialAsset> {
    if (!isTauri) browserUnsupported("Merge materials");
    return invokeTauri<MaterialAsset>("merge_materials", { ids, displayName });
  },

  async splitMaterial(
    id: string,
    sourceIds: string[],
    displayName: string,
  ): Promise<MaterialAsset[]> {
    if (!isTauri) browserUnsupported("Split material");
    return invokeTauri<MaterialAsset[]>("split_material", {
      id,
      sourceIds,
      displayName,
    });
  },

  async readFileAsDataUrl(filePath: string): Promise<string> {
    if (!isTauri) browserUnsupported("读取本地文件");
    return invokeTauri("read_file_as_data_url", { filePath });
  },

  async readThumbnailAsDataUrl(id: string): Promise<string> {
    if (!isTauri) browserUnsupported("读取缩略图");
    return invokeTauri("read_thumbnail_as_data_url", { id });
  },

  async scanFolder(folderPath: string): Promise<string[]> {
    if (!isTauri) browserUnsupported("扫描文件夹");
    return invokeTauri("scan_folder", { folderPath });
  },

  async openFiles(): Promise<string[]> {
    if (!isTauri) browserUnsupported("打开文件对话框");
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({
      multiple: true,
      filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "webp", "bmp", "gif"] }],
    });
    if (!result) return [];
    if (Array.isArray(result)) return result as string[];
    return [result as string];
  },

  async openFolder(): Promise<string | null> {
    if (!isTauri) browserUnsupported("打开文件夹对话框");
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({ directory: true });
    if (!result) return null;
    return result as string;
  },

  async openImportFile(): Promise<string | null> {
    if (!isTauri) browserUnsupported("打开导入文件");
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({
      multiple: false,
      filters: [{ name: "AutoPrompt 结果", extensions: ["zip", "json"] }],
    });
    if (!result) return null;
    return result as string;
  },

  async saveFile(defaultName: string, filters: { name: string; extensions: string[] }[]): Promise<string | null> {
    if (!isTauri) browserUnsupported("保存文件对话框");
    const { save } = await import("@tauri-apps/plugin-dialog");
    const result = await save({ defaultPath: defaultName, filters });
    return result || null;
  },
};
