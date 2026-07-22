const IMAGE_MIME_BY_EXT = {
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  png: "image/png",
  webp: "image/webp",
  gif: "image/gif",
  bmp: "image/bmp",
  tif: "image/tiff",
  tiff: "image/tiff",
};

export const BRIDGE_BASE = "http://127.0.0.1:17621";

export function selectedItemKey(item) {
  return item ? String(item.id || item.filePath || item.fileURL || item.thumbnailURL || "") : "";
}

export function mimeFromPath(filePath = "") {
  const ext = String(filePath).split(".").pop()?.toLowerCase() || "png";
  return IMAGE_MIME_BY_EXT[ext] || "image/png";
}

export function filenameFromItem(item = {}) {
  const ext = String(item.ext || item.filePath?.split(".").pop() || "png").replace(/^\./, "") || "png";
  const rawName = String(item.name || item.filePath?.split(/[\\/]/).pop() || "eagle-image");
  const name = rawName.toLowerCase().endsWith("." + ext.toLowerCase()) ? rawName.slice(0, -(ext.length + 1)) : rawName;
  return `${name || "eagle-image"}.${ext}`;
}

export function buildAnalyzePayload(item, bytes) {
  if (!item?.filePath) throw new Error("当前 Eagle 项目没有可读取的 filePath");
  if (!bytes?.length) throw new Error("图片文件为空或读取失败");
  const buffer = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes);
  const stableId = String(item.id || Date.now()).replace(/[^a-zA-Z0-9_-]/g, "-");
  return {
    id: `eagle-${stableId}-${Date.now()}`,
    fileName: filenameFromItem(item),
    base64Data: buffer.toString("base64"),
    mimeType: mimeFromPath(item.filePath),
  };
}

export function promptForItem(item, target, lang) {
  if (!item) return "";
  const first = (...values) => values.find((value) => typeof value === "string" && value.trim()) || "";
  if (target === "nano") {
    return lang === "zh"
      ? first(item.promptNanoBananaZh, item.promptNanoBananaEn, item.prompt_zh, item.prompt_en)
      : first(item.promptNanoBananaEn, item.promptNanoBananaZh, item.prompt_en, item.prompt_zh);
  }
  return lang === "zh"
    ? first(item.promptGptImageZh, item.prompt_zh, item.promptGptImageEn, item.prompt_en)
    : first(item.promptGptImageEn, item.prompt_en, item.promptGptImageZh, item.prompt_zh);
}

export function normalizeError(error) {
  const message = error?.message || String(error || "未知错误");
  if (/ECONNREFUSED|ENOTFOUND|EHOSTUNREACH|Failed to fetch|NetworkError|fetch failed|socket hang up/i.test(message)) {
    return "无法连接 EKO 本地软件。请先打开 EKO，并确认设置中心的 API 可用后重试。";
  }
  if (/timeout|timed out|aborted/i.test(message)) {
    return "连接 EKO 本地软件超时。请确认软件已打开，并稍后重试。";
  }
  return message;
}

export function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}