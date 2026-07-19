import type {
  MaterialAsset,
  MaterialCategory,
  MaterialSourceVariant,
} from "@/lib/api";

export interface MaterialPackRow {
  asset: MaterialAsset;
  source: MaterialSourceVariant;
}

export interface MaterialPack {
  id: string;
  thumbnailId: string;
  title: string;
  rows: MaterialPackRow[];
  categories: MaterialCategory[];
  sourceCount: number;
  latestCreatedAt: number;
  favorite: boolean;
}

const CATEGORY_ORDER: MaterialCategory[] = [
  "element",
  "material",
  "color",
  "lighting",
  "camera",
  "composition",
  "style",
  "environment",
];

export function buildMaterialPacks(assets: MaterialAsset[]): MaterialPack[] {
  const packs = new Map<string, MaterialPack>();
  const seenRows = new Set<string>();

  for (const asset of assets) {
    for (const source of asset.sources) {
      const rowKey = `${asset.id}:${source.id}`;
      if (seenRows.has(rowKey)) continue;
      seenRows.add(rowKey);

      const existing = packs.get(source.historyId);
      const pack = existing || {
        id: source.historyId,
        thumbnailId: source.thumbnailId,
        title: source.promptZh || asset.generatedName || "图片素材包",
        rows: [],
        categories: [],
        sourceCount: 0,
        latestCreatedAt: source.createdAt,
        favorite: false,
      };

      pack.rows.push({ asset, source });
      pack.latestCreatedAt = Math.max(pack.latestCreatedAt, source.createdAt);
      pack.favorite ||= asset.userOverride.favorite;
      if (!pack.categories.includes(asset.category)) {
        pack.categories.push(asset.category);
        pack.categories.sort(
          (left, right) => CATEGORY_ORDER.indexOf(left) - CATEGORY_ORDER.indexOf(right),
        );
      }
      pack.sourceCount = pack.rows.length;
      packs.set(source.historyId, pack);
    }
  }

  return Array.from(packs.values()).sort(
    (left, right) =>
      right.latestCreatedAt - left.latestCreatedAt ||
      right.sourceCount - left.sourceCount ||
      left.id.localeCompare(right.id),
  );
}

export function categoryCountsForPack(pack: MaterialPack) {
  return pack.rows.reduce<Partial<Record<MaterialCategory, number>>>((counts, row) => {
    counts[row.asset.category] = (counts[row.asset.category] || 0) + 1;
    return counts;
  }, {});
}

export function filterPackRowsByCategory(
  pack: MaterialPack,
  category: MaterialCategory | "all",
) {
  if (category === "all") return pack.rows;
  return pack.rows.filter((row) => row.asset.category === category);
}